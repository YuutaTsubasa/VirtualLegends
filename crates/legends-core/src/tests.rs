use crate::ai;
use crate::battle::{Action, Battle, BattleSetup, Command, CommandError, Event, Phase, Side};
use crate::data::{Catalog, StageDef};
use crate::replay::{outcome, simulate, ReplayError};
use crate::stats::{Member, SetupError};
use crate::tiles::{has_combo, meld_of, MeldKind, Suit, Tile, Wall};

fn t(suit: Suit, rank: u8) -> Tile {
    Tile::new(suit, rank)
}
const M: Suit = Suit::Man;
const P: Suit = Suit::Pin;
const S: Suit = Suit::Sou;
const H: Suit = Suit::Honor;

fn member(id: &str, level: u32) -> Member {
    let ascension = if level > 20 {
        (level - 20).div_ceil(8) as u8
    } else {
        0
    };
    Member {
        costume: format!("{id}.uniform"),
        level,
        ascension,
        bond: 0,
    }
}

fn setup(stage: &str, level: u32) -> BattleSetup {
    BattleSetup {
        stage: stage.into(),
        team: ["aming", "xiaocui", "tangtang", "lanjie"]
            .iter()
            .map(|c| member(c, level))
            .collect(),
    }
}

/// 全部交給自動戰鬥，回傳結束時的戰鬥
fn auto(cat: &Catalog, s: &BattleSetup, seed: u64) -> Battle {
    let mut b = Battle::new(cat, s, seed).unwrap();
    while let Some(cmd) = ai::choose(&b) {
        b.apply(&cmd).unwrap();
        b.take_events();
    }
    b
}

#[test]
fn builtin_data_loads() {
    let cat = Catalog::builtin();
    assert_eq!(cat.costumes.len(), 15);
    assert!(cat.stages.contains_key("1-1"));
}

#[test]
fn melds() {
    let seq = meld_of([t(M, 3), t(M, 1), t(M, 2)]).unwrap();
    assert_eq!((seq.kind, seq.wild), (MeldKind::Sequence, false));
    let tri = meld_of([t(P, 5), t(P, 5), t(P, 5)]).unwrap();
    assert_eq!((tri.kind, tri.wild), (MeldKind::Triplet, false));
    assert!(tri.power() > seq.power());
    // 字牌當萬用牌：組得起來但降一級
    let wild = meld_of([t(S, 4), t(S, 6), t(H, 1)]).unwrap();
    assert_eq!((wild.kind, wild.wild), (MeldKind::Sequence, true));
    assert!(wild.power() < seq.power());
    assert!(!meld_of([t(H, 2), t(H, 2), t(H, 2)]).unwrap().wild);
    // 不同花色、跳號都不行
    assert!(meld_of([t(M, 1), t(P, 2), t(M, 3)]).is_none());
    assert!(meld_of([t(M, 1), t(M, 2), t(M, 4)]).is_none());
    assert!(meld_of([t(S, 1), t(S, 4), t(H, 1)]).is_none());
}

#[test]
fn combo_needs_a_sequence_and_a_triplet() {
    let spent = [t(M, 1), t(P, 7), t(M, 2), t(P, 7), t(M, 3), t(P, 7)];
    assert!(has_combo(&spent));
    let two_sequences = [t(M, 1), t(M, 2), t(M, 3), t(S, 4), t(S, 5), t(S, 6)];
    assert!(!has_combo(&two_sequences));
}

#[test]
fn wall_has_rare_honors_and_refills() {
    let set = Wall::full_set();
    assert_eq!(set.len(), 27 * 4 + 7 * 2);
    let mut rng = crate::rng::Rng::new(1);
    let mut w = Wall::new(&mut rng);
    for _ in 0..set.len() + 5 {
        w.draw(&mut rng);
    }
    assert_eq!(w.len(), set.len() - 5);
}

#[test]
fn same_seed_same_battle() {
    let cat = Catalog::builtin();
    let s = setup("1-1", 10);
    for seed in [1, 42, 0xdead_beef] {
        let a = auto(&cat, &s, seed);
        let b = auto(&cat, &s, seed);
        assert_eq!(outcome(&a), outcome(&b));
        assert_eq!(a.log(), b.log());
    }
    let digests: std::collections::BTreeSet<u64> =
        (0..8).map(|seed| auto(&cat, &s, seed).digest()).collect();
    assert!(digests.len() > 1, "不同種子應該打出不同的戰鬥");
}

#[test]
fn auto_battle_clears_first_stage() {
    let cat = Catalog::builtin();
    let wins = (0..20)
        .filter(|&seed| auto(&cat, &setup("1-1", 5), seed).phase() == Phase::Won)
        .count();
    assert!(wins >= 18, "5 級隊伍打 1-1 自動戰鬥只贏 {wins}/20");
}

#[test]
fn boss_needs_training() {
    let cat = Catalog::builtin();
    let low = (0..20)
        .filter(|&seed| auto(&cat, &setup("1-10", 1), seed).phase() == Phase::Won)
        .count();
    let high = (0..20)
        .filter(|&seed| auto(&cat, &setup("1-10", 40), seed).phase() == Phase::Won)
        .count();
    assert!(low < high, "1 級贏 {low}、40 級贏 {high}");
    assert!(high >= 15, "40 級打魔王只贏 {high}/20");
}

#[test]
fn replay_matches_client() {
    let cat = Catalog::builtin();
    let s = setup("1-2", 12);
    let b = auto(&cat, &s, 7);
    let log = b.log().to_vec();
    // 指令紀錄走 JSON（Client 送給 Server 的樣子）
    let json = serde_json::to_string(&log).unwrap();
    let back: Vec<Command> = serde_json::from_str(&json).unwrap();
    assert_eq!(simulate(&cat, &s, 7, &back), Ok(outcome(&b)));
}

#[test]
fn replay_rejects_tampering() {
    let cat = Catalog::builtin();
    let s = setup("1-1", 10);
    let b = auto(&cat, &s, 3);
    let log = b.log().to_vec();
    let real = outcome(&b);

    // 換種子（自己挑好的亂數）：結果或指令就對不上
    let other = simulate(&cat, &s, 4, &log);
    assert!(other.as_ref() != Ok(&real));

    // 改等級：digest 不一樣
    let mut strong = s.clone();
    strong.team[0].level = 20;
    assert!(simulate(&cat, &strong, 3, &log).map_or(true, |o| o != real));

    // 少送指令：沒打完
    assert_eq!(
        simulate(&cat, &s, 3, &log[..log.len() - 1]),
        Err(ReplayError::Unfinished)
    );

    // 多送指令
    let mut extra = log.clone();
    extra.push(log[0].clone());
    assert!(matches!(
        simulate(&cat, &s, 3, &extra),
        Err(ReplayError::Extra { .. })
    ));
}

#[test]
fn bad_teams_are_rejected() {
    let cat = Catalog::builtin();
    let mut s = setup("1-1", 1);
    s.team.push(member("aming", 1));
    assert_eq!(
        Battle::new(&cat, &s, 1).err(),
        Some(SetupError::TeamTooLarge)
    );
    s.team.truncate(3);
    s.team.push(member("aming", 1));
    assert_eq!(
        Battle::new(&cat, &s, 1).err(),
        Some(SetupError::DuplicateCharacter("aming".into()))
    );
    let over = BattleSetup {
        stage: "1-1".into(),
        team: vec![Member {
            costume: "aming.uniform".into(),
            level: 21,
            ascension: 0,
            bond: 0,
        }],
    };
    assert_eq!(
        Battle::new(&cat, &over, 1).err(),
        Some(SetupError::BadLevel("aming.uniform".into()))
    );
}

#[test]
fn command_rules() {
    let cat = Catalog::builtin();
    let mut b = Battle::new(&cat, &setup("1-1", 1), 9).unwrap();
    // 阿明是萬；手牌：一二三萬、五筒、東
    b.set_hand(vec![t(M, 1), t(M, 2), t(M, 3), t(P, 5), t(H, 1)]);
    let target = Some(0);
    let cmd = |action| Command { actor: 0, action };
    assert_eq!(
        b.validate(&cmd(Action::Skill {
            target,
            tiles: vec![0, 3]
        })),
        Err(CommandError::WrongSuit)
    );
    assert_eq!(
        b.validate(&cmd(Action::Skill {
            target,
            tiles: vec![0]
        })),
        Err(CommandError::WrongCost)
    );
    assert_eq!(
        b.validate(&cmd(Action::Skill {
            target,
            tiles: vec![0, 0]
        })),
        Err(CommandError::BadTiles)
    );
    assert_eq!(
        b.validate(&cmd(Action::Skill {
            target,
            tiles: vec![0, 4]
        })),
        Ok(()),
        "字牌可以代替"
    );
    assert_eq!(
        b.validate(&cmd(Action::Ultimate {
            target,
            tiles: [0, 1, 3]
        })),
        Err(CommandError::NotMeld)
    );
    assert_eq!(
        b.validate(&cmd(Action::Basic { target: Some(9) })),
        Err(CommandError::BadTarget)
    );

    b.apply(&cmd(Action::Ultimate {
        target,
        tiles: [0, 1, 2],
    }))
    .unwrap();
    assert_eq!(b.hand().len(), 2, "花掉三張");
    assert_eq!(
        b.validate(&cmd(Action::Basic { target })),
        Err(CommandError::AlreadyActed)
    );
}

#[test]
fn faster_enemies_act_first() {
    let cat = Catalog::builtin();
    // 1-1 第一波：骰子（速度 104）、籌碼（86）。金爺 88：骰子比他快、籌碼比他慢
    let s = BattleSetup {
        stage: "1-1".into(),
        team: vec![member("jinye", 1), member("tangtang", 1)],
    };
    let mut b = Battle::new(&cat, &s, 5).unwrap();
    b.take_events();
    b.apply(&Command {
        actor: 0,
        action: Action::Basic { target: Some(1) },
    })
    .unwrap();
    let acts: Vec<(Side, usize)> = b
        .take_events()
        .into_iter()
        .filter_map(|e| {
            if let Event::Act { side, slot, .. } = e {
                Some((side, slot))
            } else {
                None
            }
        })
        .collect();
    assert_eq!(acts, vec![(Side::Enemy, 0), (Side::Ally, 0)]);
}

#[test]
fn disruptor_breaks_enemy_tenpai() {
    let cat = Catalog::builtin();
    let s = BattleSetup {
        stage: "1-1".into(),
        team: vec![member("cangya", 1), member("lanjie", 1)],
    };
    let mut b = Battle::new(&cat, &s, 11).unwrap();
    b.set_enemy_hand(0, vec![t(S, 2), t(S, 3), t(S, 4)]);
    assert!(b.enemies()[0].ready());
    b.set_hand(vec![t(M, 9)]);
    // 蒼牙（114）比骰子（104）快，先拿掉它一張牌
    b.apply(&Command {
        actor: 0,
        action: Action::Skill {
            target: Some(0),
            tiles: vec![0],
        },
    })
    .unwrap();
    let ev = b.take_events();
    assert!(!b.enemies()[0].ready(), "{ev:?}");
    assert_eq!(b.enemies()[0].hand.len(), 2);
}

#[test]
fn stolen_tiles_fall_back_to_basic() {
    let mut cat = Catalog::builtin();
    // 只有千面莊家（速度 106）的測試關；金爺（88）比他慢
    cat.stages.insert(
        "test-boss".into(),
        StageDef {
            id: "test-boss".into(),
            name: String::new(),
            scale: 100,
            waves: vec![vec!["dealer".into()]],
        },
    );
    let s = BattleSetup {
        stage: "test-boss".into(),
        team: vec![member("jinye", 1), member("lanjie", 1)],
    };
    let mut b = Battle::new(&cat, &s, 2).unwrap();
    // 莊家聽牌：輪到他就放「千面洗牌」，拿掉我方兩張牌
    b.set_enemy_hand(0, vec![t(H, 5), t(H, 5), t(H, 5)]);
    b.set_hand(vec![t(P, 1), t(P, 2)]);
    b.take_events();
    b.apply(&Command {
        actor: 0,
        action: Action::Skill {
            target: None,
            tiles: vec![1],
        },
    })
    .unwrap();
    let events = b.take_events();
    assert!(events.contains(&Event::Fizzle { slot: 0 }), "{events:?}");
}

/// 平衡用：`cargo test --release -- --ignored --nocapture balance`
#[test]
#[ignore]
fn balance_table() {
    let cat = Catalog::builtin();
    for stage in ["1-1", "1-2", "1-10"] {
        let row: Vec<String> = [1, 5, 10, 20, 30, 40, 60]
            .iter()
            .map(|&lv| {
                let runs: Vec<Battle> = (0..100)
                    .map(|seed| auto(&cat, &setup(stage, lv), seed))
                    .collect();
                let wins = runs.iter().filter(|b| b.phase() == Phase::Won).count();
                let rounds = runs.iter().map(|b| b.round()).sum::<u32>() / 100;
                format!("Lv{lv}:{wins}%/{rounds}r")
            })
            .collect();
        println!("{stage}: {}", row.join("  "));
    }
}

/// 重算一場要多久（Workers 免費方案每次請求 10 ms CPU）：`cargo test --release -- --ignored --nocapture replay_cost`
#[test]
#[ignore]
fn replay_cost() {
    let cat = Catalog::builtin();
    let s = setup("1-10", 20);
    let log = auto(&cat, &s, 1).log().to_vec();
    let start = std::time::Instant::now();
    for _ in 0..100 {
        simulate(&cat, &s, 1, &log).unwrap();
    }
    println!(
        "魔王戰 {} 個指令，重算一場 {:?}",
        log.len(),
        start.elapsed() / 100
    );
    let start = std::time::Instant::now();
    for _ in 0..100 {
        Catalog::builtin();
    }
    println!("讀資料一次 {:?}", start.elapsed() / 100);
}
