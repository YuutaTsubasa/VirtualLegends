use crate::ai;
use crate::battle::{Battle, BattleSetup, Command, CommandError, Event, Phase, Via};
use crate::data::{
    Catalog, Cond, Effect, EnemyDef, PassiveAct, PhaseDef, Slot, StageDef, StatKey, StatMod, Stats,
    StatusKind, To,
};
use crate::replay::{outcome, simulate, ReplayError};
use crate::state::{Charge, Side, GAUGE_FULL};
use crate::stats::{member_stats, Gear, Member, SetupError};

/// 內建資料＋一隻打不死、幾乎不動的木樁（關卡 "test"）
fn cat() -> Catalog {
    let mut cat = Catalog::builtin();
    let mut s1 = cat.skills["slime.s1"].clone();
    s1.id = "dummy.s1".into();
    s1.effects = vec![Effect::Damage {
        power: 1,
        scale: Default::default(),
        to: To::Target,
    }];
    cat.skills.insert(s1.id.clone(), s1);
    cat.enemies.insert(
        "dummy".into(),
        EnemyDef {
            id: "dummy".into(),
            name: "木樁".into(),
            element: crate::data::Element::Tide,
            stats: Stats {
                hp: 1_000_000,
                atk: 1,
                def: 0,
                spd: 1,
                ..Default::default()
            },
            skills: vec!["dummy.s1".into()],
            passives: vec![],
            toughness: 0,
            phases: vec![],
            boss: false,
        },
    );
    cat.stages.insert(
        "test".into(),
        StageDef {
            id: "test".into(),
            name: "測試".into(),
            scale: 100,
            waves: vec![vec!["dummy".into()]],
            env: None,
            star_turns: 0,
        },
    );
    cat
}

fn setup(stage: &str, team: &[&str], level: u32) -> BattleSetup {
    BattleSetup {
        stage: stage.into(),
        team: team
            .iter()
            .map(|id| {
                let mut m = Member::new(id, level);
                m.ascension = if level > 20 {
                    (level - 20).div_ceil(8) as u8
                } else {
                    0
                };
                m
            })
            .collect(),
    }
}

fn battle(cat: &Catalog, team: &[&str]) -> Battle {
    let mut b = Battle::new(cat, &setup("test", team, 1), 7).unwrap();
    b.take_events();
    b
}

/// 現在輪到的人用第 skill 招
fn act(b: &mut Battle, skill: usize, target: Option<usize>, relay: Option<usize>) -> Vec<Event> {
    let actor = b.current().unwrap();
    b.apply(&Command {
        actor,
        skill,
        target,
        relay,
    })
    .unwrap();
    b.take_events()
}

/// 等到輪到我方第 i 個
fn until(b: &mut Battle, i: usize) {
    for _ in 0..50 {
        if b.current() == Some(i) {
            return;
        }
        act(b, 0, Some(0), None);
    }
    panic!("一直沒輪到 {i}");
}

fn auto(cat: &Catalog, s: &BattleSetup, seed: u64) -> Battle {
    let mut b = Battle::new(cat, s, seed).unwrap();
    while let Some(cmd) = ai::choose(&b) {
        b.apply(&cmd).unwrap();
        b.take_events();
    }
    b
}

const STARTER: [&str; 4] = [
    "xiaocui.daily",
    "aming.daily",
    "tangtang.daily",
    "lanjie.daily",
];

#[test]
fn builtin_data_loads() {
    let cat = Catalog::builtin();
    assert_eq!(cat.costumes.len(), 15);
    assert_eq!(cat.stages.len(), 10);
    for c in cat.costumes.values() {
        for id in &c.skills {
            let s = &cat.skills[id];
            // 每一級都還是合法的技能
            for lv in 1..=s.max_level() {
                assert!(!s.at_level(lv).effects.is_empty(), "{id} lv{lv}");
            }
        }
    }
}

#[test]
fn deterministic() {
    let cat = cat();
    let s = setup("1-3", &STARTER, 15);
    let a = auto(&cat, &s, 42);
    let b = auto(&cat, &s, 42);
    assert_eq!(a.digest(), b.digest());
    assert_eq!(a.log(), b.log());
    let c = auto(&cat, &s, 43);
    assert_ne!(a.digest(), c.digest());
}

#[test]
fn replay_matches_and_rejects_bad_logs() {
    let cat = cat();
    let s = setup("1-1", &STARTER, 5);
    let b = auto(&cat, &s, 1);
    assert_eq!(b.phase(), Phase::Won);
    let log = b.log().to_vec();
    assert_eq!(simulate(&cat, &s, 1, &log), Ok(outcome(&b)));
    assert_eq!(
        simulate(&cat, &s, 1, &log[..log.len() - 1]),
        Err(ReplayError::Unfinished)
    );
    let mut extra = log.clone();
    extra.push(log[0].clone());
    assert_eq!(
        simulate(&cat, &s, 1, &extra),
        Err(ReplayError::Extra { index: log.len() })
    );
    // 換一個種子，同樣的指令幾乎一定在某一步不合法或結果不同
    let other = simulate(&cat, &s, 2, &log);
    assert!(other.is_err() || other.unwrap().digest != outcome(&b).digest);
}

#[test]
fn faster_units_act_more_often() {
    let mut cat = cat();
    cat.enemies.get_mut("dummy").unwrap().stats.spd = 100;
    cat.costumes.get_mut("xiaocui.daily").unwrap().base.spd = 200;
    let b = battle(&cat, &["xiaocui.daily"]);
    let line = b.timeline(30);
    let mine = line.iter().filter(|w| w.0 == Side::Ally).count();
    let theirs = line.len() - mine;
    assert!(
        mine >= theirs * 2 - 1 && mine <= theirs * 2 + 1,
        "{mine} vs {theirs}"
    );
}

#[test]
fn push_and_delay() {
    let cat = cat();
    let mut b = battle(&cat, &["tangtang.idol"]);
    let ev = act(&mut b, 1, None, None);
    assert!(ev.contains(&Event::Gauge {
        side: Side::Ally,
        unit: 0,
        pct: 20,
        gauge: GAUGE_FULL / 5
    }));

    let mut b = battle(&cat, &["yuehua.daily"]);
    b.enemies[0].gauge = 500;
    let ev = act(&mut b, 0, Some(0), None);
    assert!(
        ev.contains(&Event::Gauge {
            side: Side::Enemy,
            unit: 0,
            pct: -10,
            gauge: 400
        }) || ev.iter().any(|e| matches!(e, Event::Resisted { .. }))
    );
}

#[test]
fn relay_lets_an_ally_act_next() {
    let cat = cat();
    let mut b = battle(&cat, &["xiaocui.daily", "aming.daily"]);
    let first = b.current().unwrap();
    let other = 1 - first;
    assert_eq!(
        b.validate(&Command {
            actor: first,
            skill: 0,
            target: Some(0),
            relay: Some(other)
        }),
        Err(CommandError::NoCheer)
    );
    while b.cheer() < 3 {
        act(&mut b, 0, Some(0), None);
    }
    let me = b.current().unwrap();
    let mate = 1 - me;
    assert_eq!(
        b.validate(&Command {
            actor: me,
            skill: 0,
            target: Some(0),
            relay: Some(me)
        }),
        Err(CommandError::BadRelay)
    );
    let ev = act(&mut b, 0, Some(0), Some(mate));
    assert!(ev.contains(&Event::Relay { from: me, to: mate }));
    assert_eq!(b.current(), Some(mate));
    assert_eq!(b.cheer(), 1);
}

#[test]
fn cooldowns_count_own_turns() {
    let cat = cat();
    let mut b = battle(&cat, &["xiaocui.daily"]);
    act(&mut b, 1, None, None);
    let cmd = |b: &Battle| Command {
        actor: b.current().unwrap(),
        skill: 1,
        target: None,
        relay: None,
    };
    // 冷卻 3：用完之後的兩個自己回合不能用，第三個可以
    assert_eq!(b.validate(&cmd(&b)), Err(CommandError::Cooldown));
    act(&mut b, 0, Some(0), None);
    assert_eq!(b.validate(&cmd(&b)), Err(CommandError::Cooldown));
    act(&mut b, 0, Some(0), None);
    assert_eq!(b.validate(&cmd(&b)), Ok(()));
}

#[test]
fn skill_upgrades_apply() {
    let cat = cat();
    let mut s = setup("test", &["xiaocui.daily"], 1);
    s.team[0].skills = [1, 3, 1];
    let mut b = Battle::new(&cat, &s, 7).unwrap();
    b.take_events();
    act(&mut b, 1, None, None);
    act(&mut b, 0, Some(0), None);
    // 強化到 3 級：冷卻 3 → 2
    assert_eq!(
        b.validate(&Command {
            actor: 0,
            skill: 1,
            target: None,
            relay: None
        }),
        Ok(())
    );
    s.team[0].skills = [1, 4, 1];
    assert!(matches!(
        Battle::new(&cat, &s, 7),
        Err(SetupError::BadSkillLevel(_))
    ));
}

#[test]
fn counters_are_not_countered() {
    let mut cat = cat();
    let always = |cat: &mut Catalog, id: &str| {
        let c = cat.costumes.get_mut(id).unwrap();
        c.passives[0].chance = 100;
    };
    always(&mut cat, "alie.daily");
    let dummy = cat.enemies.get_mut("dummy").unwrap();
    dummy.passives = cat.costumes["alie.daily"].passives.clone();
    let mut b = battle(&cat, &["alie.daily"]);
    let ev = act(&mut b, 0, Some(0), None);
    let counters = ev
        .iter()
        .filter(|e| {
            matches!(
                e,
                Event::Trigger {
                    via: Via::Counter,
                    ..
                }
            )
        })
        .count();
    assert_eq!(counters, 1, "{ev:?}");
}

#[test]
fn links_follow_but_do_not_chain() {
    let mut cat = cat();
    cat.rules.bond_link = 100;
    let mut b = battle(&cat, &["xiaocui.daily", "yuehua.daily"]);
    let ev = act(&mut b, 0, Some(0), None);
    let links: Vec<_> = ev
        .iter()
        .filter(|e| matches!(e, Event::Trigger { via: Via::Link, .. }))
        .collect();
    assert_eq!(links.len(), 1, "{ev:?}");
}

#[test]
fn pursuit_after_attack() {
    let mut cat = cat();
    let p = &mut cat.costumes.get_mut("aming.daily").unwrap().passives[0];
    p.cond = Some(Cond::TargetHpBelow(101));
    assert!(matches!(p.act, PassiveAct::Pursuit { .. }));
    let mut b = battle(&cat, &["aming.daily"]);
    let ev = act(&mut b, 0, Some(0), None);
    assert_eq!(
        ev.iter()
            .filter(|e| matches!(
                e,
                Event::Trigger {
                    via: Via::Pursuit,
                    ..
                }
            ))
            .count(),
        1
    );
}

#[test]
fn stun_interrupts_charge() {
    let mut cat = cat();
    let s = cat.skills.get_mut("alie.daily.s2").unwrap();
    s.effects = vec![Effect::Status {
        status: StatusKind::Stun,
        turns: 1,
        chance: 100,
        to: To::Target,
    }];
    let mut b = battle(&cat, &["alie.daily"]);
    b.enemies[0].charge = Some(Charge { slot: 0, turns: 1 });
    let ev = act(&mut b, 1, Some(0), None);
    assert!(ev.contains(&Event::Interrupted { unit: 0 }), "{ev:?}");
    assert!(b.enemies[0].charge.is_none());
}

#[test]
fn enemy_charges_then_fires() {
    let cat = cat();
    let mut b = Battle::new(&cat, &setup("1-5", &STARTER, 20), 3).unwrap();
    let mut charged = false;
    let mut fired = false;
    while let Some(cmd) = ai::choose(&b) {
        b.apply(&cmd).unwrap();
        for e in b.take_events() {
            match e {
                Event::Charge { .. } => charged = true,
                Event::Skill {
                    side: Side::Enemy,
                    ref skill,
                    ..
                } if skill == "sentinel.s2" => fired = true,
                _ => {}
            }
        }
    }
    assert!(charged && fired);
}

#[test]
fn breaking_toughness_stuns() {
    let mut cat = cat();
    cat.enemies.get_mut("dummy").unwrap().toughness = 1;
    let mut b = battle(&cat, &["aming.daily"]);
    let ev = act(&mut b, 0, Some(0), None);
    assert!(ev.contains(&Event::Break { unit: 0 }));
    assert!(b.enemies[0].has(StatusKind::Stun));
}

#[test]
fn boss_changes_phase() {
    let mut cat = cat();
    cat.enemies.get_mut("dummy").unwrap().phases = vec![PhaseDef {
        hp_below: 50,
        skills: None,
        effects: vec![Effect::Status {
            status: StatusKind::AtkUp,
            turns: 9,
            chance: 100,
            to: To::Myself,
        }],
        name: "過載".into(),
    }];
    let mut b = battle(&cat, &["aming.daily"]);
    b.enemies[0].hp = b.enemies[0].max_hp / 2;
    let ev = act(&mut b, 0, Some(0), None);
    assert!(ev
        .iter()
        .any(|e| matches!(e, Event::Phase { phase: 1, .. })));
    assert!(b.enemies[0].has(StatusKind::AtkUp));
}

#[test]
fn immune_resists_debuffs() {
    let cat = cat();
    let mut b = battle(&cat, &["cangya.daily"]);
    b.enemies[0].statuses.push(crate::state::Status {
        kind: StatusKind::Immune,
        turns: 9,
        value: 0,
        source: None,
        fresh: false,
    });
    let ev = act(&mut b, 1, Some(0), None);
    assert!(ev.iter().any(|e| matches!(e, Event::Resisted { .. })));
    assert!(!b.enemies[0].has(StatusKind::DefDown));
}

#[test]
fn self_buff_lasts_its_turns() {
    let cat = cat();
    let mut b = battle(&cat, &["jinye.daily"]);
    act(&mut b, 1, None, None); // 攻擊提升 3 回合
    for _ in 0..3 {
        assert!(b.allies[0].has(StatusKind::AtkUp));
        act(&mut b, 0, Some(0), None);
    }
    assert!(!b.allies[0].has(StatusKind::AtkUp));
}

#[test]
fn taunt_and_silence_restrict_commands() {
    let cat = cat();
    let mut s = setup("test", &["aming.daily"], 1);
    s.stage = "1-3".into();
    let mut b = Battle::new(&cat, &s, 5).unwrap();
    b.take_events();
    let me = b.current().unwrap();
    b.allies[me].statuses.push(crate::state::Status {
        kind: StatusKind::Taunt,
        turns: 2,
        value: 0,
        source: Some((Side::Enemy, 1)),
        fresh: false,
    });
    assert_eq!(
        b.validate(&Command {
            actor: me,
            skill: 0,
            target: Some(0),
            relay: None
        }),
        Err(CommandError::Taunted { target: 1 })
    );
    b.allies[me].statuses.push(crate::state::Status {
        kind: StatusKind::Silence,
        turns: 2,
        value: 0,
        source: None,
        fresh: false,
    });
    assert_eq!(
        b.validate(&Command {
            actor: me,
            skill: 1,
            target: Some(1),
            relay: None
        }),
        Err(CommandError::Silenced)
    );
    assert_eq!(
        b.validate(&Command {
            actor: me,
            skill: 0,
            target: Some(9),
            relay: None
        }),
        Err(CommandError::BadTarget)
    );
    assert_eq!(
        b.validate(&Command {
            actor: me,
            skill: 0,
            target: None,
            relay: None
        }),
        Err(CommandError::NeedTarget)
    );
    assert!(matches!(
        b.validate(&Command {
            actor: me + 1,
            skill: 0,
            target: Some(1),
            relay: None
        }),
        Err(CommandError::NotYourTurn { .. })
    ));
}

#[test]
fn resonance_environment_and_gear() {
    let cat = cat();
    let atk = |team: &[&str], stage: &str| {
        let b = Battle::new(&cat, &setup(stage, team, 1), 1).unwrap();
        b.allies[0].stats
    };
    let alone = atk(&["aming.daily"], "test");
    assert_eq!(
        atk(&["aming.daily", "alie.daily"], "test").atk,
        alone.atk * 108 / 100
    );
    assert_eq!(atk(&["aming.daily"], "1-2").spd, alone.spd + 15);

    let mut m = Member::new("aming.daily", 1);
    let piece = |slot, set: &str| Gear {
        slot,
        set: set.into(),
        main: StatMod {
            stat: StatKey::Atk,
            add: 0,
            pct: 0,
        },
        subs: vec![],
    };
    m.gear = vec![piece(Slot::Weapon, "gale"), piece(Slot::Armor, "gale")];
    assert_eq!(member_stats(&cat, &m).unwrap().spd, alone.spd * 110 / 100);
    m.gear.push(piece(Slot::Armor, "gale"));
    let bad = BattleSetup {
        stage: "test".into(),
        team: vec![m],
    };
    assert!(matches!(
        Battle::new(&cat, &bad, 1),
        Err(SetupError::BadGear(_))
    ));
}

#[test]
fn unison_set_adds_link() {
    let mut cat = cat();
    cat.sets
        .get_mut("unison")
        .unwrap()
        .passive
        .as_mut()
        .unwrap()
        .chance = 100;
    let mut s = setup("test", &["aming.daily", "leo.daily"], 1);
    s.team[1].gear = [Slot::Weapon, Slot::Armor, Slot::Accessory, Slot::Badge]
        .into_iter()
        .map(|slot| Gear {
            slot,
            set: "unison".into(),
            main: StatMod {
                stat: StatKey::Hp,
                add: 0,
                pct: 0,
            },
            subs: vec![],
        })
        .collect();
    let mut b = Battle::new(&cat, &s, 7).unwrap();
    b.take_events();
    until(&mut b, 0);
    let ev = act(&mut b, 0, Some(0), None);
    assert!(
        ev.iter().any(|e| matches!(
            e,
            Event::Trigger {
                side: Side::Ally,
                unit: 1,
                via: Via::Link,
                ..
            }
        )),
        "{ev:?}"
    );
}

#[test]
fn starter_team_clears_first_stage() {
    let cat = Catalog::builtin();
    let wins = (0..50)
        .filter(|&seed| auto(&cat, &setup("1-1", &STARTER, 1), seed).phase() == Phase::Won)
        .count();
    assert!(wins >= 45, "{wins}/50");
}

/// 平衡用：`cargo test --release -- --ignored --nocapture balance_table`
#[test]
#[ignore]
fn balance_table() {
    let cat = Catalog::builtin();
    for stage in cat.stages.keys() {
        let row: Vec<String> = [1, 5, 10, 15, 20, 30, 40]
            .iter()
            .map(|&lv| {
                let runs: Vec<Battle> = (0..100)
                    .map(|seed| auto(&cat, &setup(stage, &STARTER, lv), seed))
                    .collect();
                let wins = runs.iter().filter(|b| b.phase() == Phase::Won).count();
                let turns = runs.iter().map(|b| b.ally_turns()).sum::<u32>() / 100;
                format!("Lv{lv}:{wins:>3}%/{turns:>2}t")
            })
            .collect();
        println!("{stage:>5}: {}", row.join("  "));
    }
}

/// 重算一場要多久（Workers 免費方案每次請求 10 ms CPU）：`cargo test --release -- --ignored --nocapture replay_cost`
#[test]
#[ignore]
fn replay_cost() {
    let cat = Catalog::builtin();
    let s = setup("1-10", &STARTER, 30);
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
