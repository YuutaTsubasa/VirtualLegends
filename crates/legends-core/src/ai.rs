//! 自動戰鬥：玩家按「自動」時由這裡出指令，指令一樣進紀錄、一樣由 Server 重算。
//! 規則很單純：能放大招就放（刻子優先）、能放技能就放、不然普攻；
//! 打敵人時先打聽牌的，再打血最少的；補血只在有人低於七成時用。

use crate::battle::{Action, Battle, Command, Phase};
use crate::data::{Effect, Target};
use crate::tiles::{meld_of, Suit, Tile};

/// 手牌裡最強的一組三張（位置），沒有就 None
pub fn best_meld(hand: &[Tile]) -> Option<[usize; 3]> {
    let n = hand.len();
    let mut best: Option<(i64, [usize; 3])> = None;
    for i in 0..n {
        for j in i + 1..n {
            for k in j + 1..n {
                if let Some(m) = meld_of([hand[i], hand[j], hand[k]]) {
                    if best.is_none_or(|(p, _)| m.power() > p) {
                        best = Some((m.power(), [i, j, k]));
                    }
                }
            }
        }
    }
    best.map(|(_, t)| t)
}

pub fn choose(b: &Battle) -> Option<Command> {
    if b.phase() != Phase::Command {
        return None;
    }
    let pending = b.pending();
    let hand = b.hand();
    // 大招：手上有一組就給第一個還沒動的人放
    if let Some(tiles) = best_meld(hand) {
        let actor = pending[0];
        let skill = b.skill(&b.allies()[actor].skills[2]);
        return Some(Command {
            actor,
            action: Action::Ultimate {
                target: target(b, actor, skill.target),
                tiles,
            },
        });
    }
    // 技能：誰付得起、而且用得上
    for &actor in &pending {
        let a = &b.allies()[actor];
        let skill = b.skill(&a.skills[1]);
        let heals = skill
            .effects
            .iter()
            .any(|e| matches!(e, Effect::Heal { .. }));
        if heals
            && !b
                .allies()
                .iter()
                .any(|f| f.alive() && f.hp * 10 < f.max_hp * 7)
        {
            continue;
        }
        let payable: Vec<usize> = (0..hand.len())
            .filter(|&i| {
                a.suit == Suit::Honor || hand[i].suit == a.suit || hand[i].suit == Suit::Honor
            })
            .collect();
        // 字牌留給大招當萬用牌，先花同花色的
        let mut tiles: Vec<usize> = payable
            .iter()
            .copied()
            .filter(|&i| hand[i].suit != Suit::Honor)
            .collect();
        tiles.extend(
            payable
                .iter()
                .copied()
                .filter(|&i| hand[i].suit == Suit::Honor),
        );
        if tiles.len() >= skill.cost as usize {
            tiles.truncate(skill.cost as usize);
            return Some(Command {
                actor,
                action: Action::Skill {
                    target: target(b, actor, skill.target),
                    tiles,
                },
            });
        }
    }
    let actor = pending[0];
    let skill = b.skill(&b.allies()[actor].skills[0]);
    Some(Command {
        actor,
        action: Action::Basic {
            target: target(b, actor, skill.target),
        },
    })
}

fn target(b: &Battle, actor: usize, t: Target) -> Option<usize> {
    match t {
        Target::Enemy => {
            let e = b.enemies();
            (0..e.len())
                .filter(|&i| e[i].alive())
                .min_by_key(|&i| (!e[i].ready(), e[i].hp, i))
        }
        Target::Ally => {
            let a = b.allies();
            (0..a.len())
                .filter(|&i| a[i].alive())
                .min_by_key(|&i| (a[i].hp * 1000 / a[i].max_hp, i))
        }
        _ => Some(actor),
    }
}
