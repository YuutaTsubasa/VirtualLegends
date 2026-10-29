//! 自動戰鬥：替現在輪到的我方角色選一個指令。不用亂數，同一個局面一定選一樣的。
//!
//! - 招式從大招往下挑：冷卻好了、而且「有用」的（純治療在沒人受傷時不用）。
//! - 單體攻擊：有敵人在蓄力，帶暈眩或延遲的招先打它；否則打剋得到、HP 比例最低的。
//! - 單體輔助：治療給 HP 比例最低的；其他給攻擊力最高的隊友。
//! - 聲援滿了就接棒給大招冷卻好的隊友（攻擊力高的先）。

use crate::battle::{Battle, Command, Phase};
use crate::data::{Effect, SkillDef, StatKey, StatusKind, Target};
use crate::state::{Side, Who};

fn heals_only(s: &SkillDef) -> bool {
    s.effects.iter().all(|e| {
        matches!(
            e,
            Effect::Heal { .. } | Effect::Shield { .. } | Effect::Cleanse { .. }
        )
    })
}

fn interrupts(s: &SkillDef) -> bool {
    s.effects.iter().any(|e| {
        matches!(
            e,
            Effect::Status {
                status: StatusKind::Stun,
                ..
            }
        ) || matches!(e, Effect::Gauge { pct, .. } if *pct < 0)
    })
}

fn heals(s: &SkillDef) -> bool {
    s.effects.iter().any(|e| matches!(e, Effect::Heal { .. }))
}

pub fn choose(b: &Battle) -> Option<Command> {
    if b.phase() != Phase::Command {
        return None;
    }
    let actor = b.current()?;
    let me = &b.allies()[actor];
    let st = &b.rules().status;
    let allies: Vec<Who> = (0..b.allies().len())
        .filter(|&i| b.allies()[i].alive())
        .map(|i| (Side::Ally, i))
        .collect();
    let foes: Vec<usize> = (0..b.enemies().len())
        .filter(|&i| b.enemies()[i].alive())
        .collect();
    let hurt = allies.iter().any(|&w| b.unit(w).hp_pct() < 70);

    let slot = (0..me.skills.len())
        .rev()
        .find(|&s| me.can_use(s) && (!heals_only(&me.skills[s]) || hurt))
        .unwrap_or(0);
    let skill = &me.skills[slot];

    let target = match skill.target {
        Target::Enemy => {
            let forced = me
                .taunted_by()
                .filter(|w| w.0 == Side::Enemy && b.enemies()[w.1].alive())
                .map(|w| w.1);
            let charging = foes
                .iter()
                .copied()
                .find(|&i| b.enemies()[i].charge.is_some());
            forced
                .or(charging.filter(|_| interrupts(skill)))
                .or_else(|| {
                    foes.iter().copied().min_by_key(|&i| {
                        let e = &b.enemies()[i];
                        (!me.element.beats(e.element), e.hp_pct(), i)
                    })
                })
        }
        Target::Ally => {
            let pick = if heals(skill) {
                allies.iter().min_by_key(|&&w| (b.unit(w).hp_pct(), w.1))
            } else {
                allies
                    .iter()
                    .filter(|&&w| w.1 != actor || allies.len() == 1)
                    .max_by_key(|&&w| (b.unit(w).eff(StatKey::Atk, st), std::cmp::Reverse(w.1)))
            };
            pick.map(|w| w.1)
        }
        _ => None,
    };

    let relay = (b.cheer() >= b.rules().cheer_max)
        .then(|| {
            allies
                .iter()
                .filter(|&&w| {
                    w.1 != actor && b.unit(w).can_use(2) && !b.unit(w).has(StatusKind::Stun)
                })
                .max_by_key(|&&w| (b.unit(w).eff(StatKey::Atk, st), std::cmp::Reverse(w.1)))
                .map(|w| w.1)
        })
        .flatten();

    Some(Command {
        actor,
        skill: slot,
        target,
        relay,
    })
}
