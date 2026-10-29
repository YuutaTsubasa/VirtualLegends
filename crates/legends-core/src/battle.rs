//! 行動條回合制戰鬥：把 Scheduler、Action Resolver、Effect、Trigger、Resource、Battle State 串起來。
//!
//! 流程：Scheduler 決定下一個出手的單位 → 回合開始（冷卻、灼燒、被動）→
//! 敵人由 AI 出手；輪到我方就停下來等指令（`Phase::Command`）。
//! 我方一次送一個 `Command`（哪一招、打誰、要不要接棒），送出時就檢查。
//! 確定性：同樣的隊伍、關卡、種子、指令，一定得到同樣的結果（亂數只來自種子，全部整數運算）。

use crate::data::{Catalog, EnvDef, Rules, SkillDef, StatusKind};
use crate::rng::Rng;
use crate::scheduler::{GaugeScheduler, Scheduler};
use crate::state::{Side, Unit, Who};
use crate::stats::{base_stats, gear_bonus, validate_team, Bonus, Member, SetupError};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// 雙方加起來最多幾個回合，打不完算輸
pub const MAX_TURNS: u32 = 300;
/// 開場行動值的亂數範圍
pub const START_GAUGE: u64 = 200;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BattleSetup {
    pub stage: String,
    pub team: Vec<Member>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    /// 等我方指令
    Command,
    Won,
    Lost,
}

/// 我方一個指令。actor 必須是現在輪到的人（多寫一次，避免畫面和引擎對不上）
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Command {
    pub actor: usize,
    /// 第幾招（0～2）
    pub skill: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<usize>,
    /// 花聲援讓這位隊友緊接著出手
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relay: Option<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub enum CommandError {
    Over,
    NotYourTurn {
        current: Option<usize>,
    },
    NoSkill,
    Cooldown,
    Silenced,
    NeedTarget,
    BadTarget,
    /// 被挑釁了，只能打挑釁的人
    Taunted {
        target: usize,
    },
    BadRelay,
    NoCheer,
}

/// 觸發來源
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Via {
    Counter,
    Link,
    Pursuit,
    Passive,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    Wave {
        wave: usize,
    },
    Turn {
        side: Side,
        unit: usize,
    },
    Skill {
        side: Side,
        unit: usize,
        skill: String,
        slot: usize,
        target_side: Side,
        targets: Vec<usize>,
        via: Option<Via>,
    },
    Damage {
        side: Side,
        unit: usize,
        amount: i64,
        crit: bool,
        adv: bool,
        graze: bool,
        dot: bool,
        hp: i64,
        shield: i64,
    },
    Heal {
        side: Side,
        unit: usize,
        amount: i64,
        hp: i64,
    },
    Shield {
        side: Side,
        unit: usize,
        amount: i64,
    },
    Status {
        side: Side,
        unit: usize,
        status: StatusKind,
        turns: u8,
    },
    Resisted {
        side: Side,
        unit: usize,
    },
    Expired {
        side: Side,
        unit: usize,
        status: StatusKind,
    },
    Removed {
        side: Side,
        unit: usize,
        status: StatusKind,
    },
    Gauge {
        side: Side,
        unit: usize,
        pct: i64,
        gauge: i64,
    },
    Cooldown {
        side: Side,
        unit: usize,
        amount: u8,
    },
    Charge {
        unit: usize,
        skill: String,
        turns: u8,
    },
    Charging {
        unit: usize,
        turns: u8,
    },
    Interrupted {
        unit: usize,
    },
    Break {
        unit: usize,
    },
    Phase {
        unit: usize,
        phase: usize,
        name: String,
    },
    Stunned {
        side: Side,
        unit: usize,
    },
    Down {
        side: Side,
        unit: usize,
    },
    Trigger {
        side: Side,
        unit: usize,
        via: Via,
        name: String,
    },
    Cheer {
        value: u8,
    },
    Relay {
        from: usize,
        to: usize,
    },
    Won,
    Lost,
}

#[derive(Clone, Debug)]
pub struct Battle {
    pub(crate) rules: Rules,
    pub(crate) book: BTreeMap<String, SkillDef>,
    pub(crate) rng: Rng,
    pub(crate) allies: Vec<Unit>,
    pub(crate) enemies: Vec<Unit>,
    pub(crate) waves: Vec<Vec<Unit>>,
    pub(crate) wave: usize,
    pub(crate) env: Option<EnvDef>,
    /// 我方羈絆組合（隊伍裡的位置）
    pub(crate) bonds: Vec<(usize, usize)>,
    pub(crate) phase: Phase,
    pub(crate) current: Option<usize>,
    /// 正在出手的單位（自己回合上的狀態不在這回合結束時扣）
    pub(crate) acting: Option<Who>,
    pub(crate) cheer: u8,
    pub(crate) relay: Option<usize>,
    pub(crate) turns: u32,
    pub(crate) ally_turns: u32,
    pub(crate) log: Vec<Command>,
    pub(crate) events: Vec<Event>,
}

impl Battle {
    pub fn new(cat: &Catalog, setup: &BattleSetup, seed: u64) -> Result<Battle, SetupError> {
        validate_team(cat, &setup.team)?;
        let stage = cat
            .stages
            .get(&setup.stage)
            .ok_or_else(|| SetupError::UnknownStage(setup.stage.clone()))?;
        let rules = cat.rules.clone();
        let env = stage.env.clone();
        let env_mods = |side: Side, el, bonus: &mut Bonus| {
            for m in env.iter().flat_map(|e| &e.mods) {
                if m.side.is_none_or(|s| s == side) && m.element.is_none_or(|e| e == el) {
                    bonus.push(&m.stat);
                }
            }
        };

        let defs: Vec<_> = setup
            .team
            .iter()
            .map(|m| &cat.costumes[&m.costume])
            .collect();
        let mut allies = Vec::new();
        for (m, def) in setup.team.iter().zip(&defs) {
            let mut bonus = Bonus::default();
            let mut passives = def.passives.clone();
            passives.extend(gear_bonus(cat, m, &mut bonus));
            for r in &rules.resonance {
                if r.element == def.element
                    && defs.iter().filter(|d| d.element == def.element).count() >= r.count
                {
                    bonus.push(&r.stat);
                }
            }
            env_mods(Side::Ally, def.element, &mut bonus);
            let stats = bonus.apply(base_stats(cat, def, m));
            let skills: Vec<SkillDef> = def
                .skills
                .iter()
                .zip(m.skills)
                .map(|(id, lv)| cat.skills[id].at_level(lv))
                .collect();
            allies.push(Unit {
                id: def.id.clone(),
                character: Some(def.character.clone()),
                element: def.element,
                role: Some(def.role),
                boss: false,
                max_hp: stats.hp,
                hp: stats.hp,
                shield: 0,
                shield_turns: 0,
                stats,
                gauge: 0,
                skill_ids: def.skills.to_vec(),
                cooldowns: vec![0; 3],
                statuses: Vec::new(),
                toughness: 0,
                max_toughness: 0,
                charge: None,
                phase: 0,
                skills,
                passives,
                phases: Vec::new(),
            });
        }
        let mut bonds = Vec::new();
        for i in 0..defs.len() {
            for j in i + 1..defs.len() {
                if cat.bonded(&defs[i].character, &defs[j].character) {
                    bonds.push((i, j));
                }
            }
        }

        let waves = stage
            .waves
            .iter()
            .map(|w| {
                w.iter()
                    .map(|id| {
                        let e = &cat.enemies[id];
                        let mut base = e.stats;
                        for k in [
                            crate::data::StatKey::Hp,
                            crate::data::StatKey::Atk,
                            crate::data::StatKey::Def,
                        ] {
                            *base.get_mut(k) = base.get(k) * stage.scale / 100;
                        }
                        for k in crate::data::Stats::KEYS {
                            *base.get_mut(k) += rules.base.get(k);
                        }
                        let mut bonus = Bonus::default();
                        env_mods(Side::Enemy, e.element, &mut bonus);
                        let stats = bonus.apply(base);
                        Unit {
                            id: e.id.clone(),
                            character: None,
                            element: e.element,
                            role: None,
                            boss: e.boss,
                            max_hp: stats.hp,
                            hp: stats.hp,
                            shield: 0,
                            shield_turns: 0,
                            stats,
                            gauge: 0,
                            skill_ids: e.skills.clone(),
                            cooldowns: vec![0; e.skills.len()],
                            statuses: Vec::new(),
                            toughness: e.toughness,
                            max_toughness: e.toughness,
                            charge: None,
                            phase: 0,
                            skills: e.skills.iter().map(|s| cat.skills[s].clone()).collect(),
                            passives: e.passives.clone(),
                            phases: e.phases.clone(),
                        }
                    })
                    .collect()
            })
            .collect();

        let mut b = Battle {
            rules,
            book: cat.skills.clone(),
            rng: Rng::new(seed),
            allies,
            enemies: Vec::new(),
            waves,
            wave: 0,
            env,
            bonds,
            phase: Phase::Command,
            current: None,
            acting: None,
            cheer: 0,
            relay: None,
            turns: 0,
            ally_turns: 0,
            log: Vec::new(),
            events: Vec::new(),
        };
        for u in &mut b.allies {
            u.gauge = b.rng.below(START_GAUGE) as i64;
        }
        b.start_wave(0);
        b.advance();
        Ok(b)
    }

    // ---- 給畫面與 Server 看的 ----

    pub fn phase(&self) -> Phase {
        self.phase
    }
    /// 現在輪到我方第幾個（等指令時）
    pub fn current(&self) -> Option<usize> {
        self.current
    }
    pub fn allies(&self) -> &[Unit] {
        &self.allies
    }
    pub fn enemies(&self) -> &[Unit] {
        &self.enemies
    }
    pub fn wave(&self) -> usize {
        self.wave
    }
    pub fn wave_count(&self) -> usize {
        self.waves.len()
    }
    pub fn cheer(&self) -> u8 {
        self.cheer
    }
    pub fn rules(&self) -> &Rules {
        &self.rules
    }
    pub fn env(&self) -> Option<&EnvDef> {
        self.env.as_ref()
    }
    /// 我方出手過幾次（三星評價用）
    pub fn ally_turns(&self) -> u32 {
        self.ally_turns
    }
    pub fn turns(&self) -> u32 {
        self.turns
    }
    pub fn log(&self) -> &[Command] {
        &self.log
    }
    pub fn take_events(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.events)
    }
    pub fn unit(&self, w: Who) -> &Unit {
        match w.0 {
            Side::Ally => &self.allies[w.1],
            Side::Enemy => &self.enemies[w.1],
        }
    }
    pub(crate) fn unit_mut(&mut self, w: Who) -> &mut Unit {
        match w.0 {
            Side::Ally => &mut self.allies[w.1],
            Side::Enemy => &mut self.enemies[w.1],
        }
    }
    pub(crate) fn side(&self, s: Side) -> &[Unit] {
        match s {
            Side::Ally => &self.allies,
            Side::Enemy => &self.enemies,
        }
    }
    pub(crate) fn alive_of(&self, s: Side) -> Vec<Who> {
        self.side(s)
            .iter()
            .enumerate()
            .filter(|(_, u)| u.alive())
            .map(|(i, _)| (s, i))
            .collect()
    }

    /// 接下來 n 個出手的人（時間軸）。現在輪到的人排第一
    pub fn timeline(&self, n: usize) -> Vec<Who> {
        let mut out: Vec<Who> = self.current.map(|i| (Side::Ally, i)).into_iter().collect();
        out.extend(GaugeScheduler.preview(
            &self.allies,
            &self.enemies,
            self.relay,
            &self.rules.status,
            n.saturating_sub(out.len()),
        ));
        out
    }

    /// 狀態摘要：Client 與 Server 跑出來的結果要一樣
    pub fn digest(&self) -> u64 {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        let mut put = |bytes: &[u8]| {
            for &b in bytes {
                h ^= b as u64;
                h = h.wrapping_mul(0x0000_0100_0000_01b3);
            }
        };
        put(&self.rng.state().to_le_bytes());
        put(&[self.wave as u8, self.cheer, self.phase as u8]);
        put(&self.turns.to_le_bytes());
        put(serde_json::to_string(&self.allies).unwrap().as_bytes());
        put(serde_json::to_string(&self.enemies).unwrap().as_bytes());
        h
    }

    // ---- 指令 ----

    pub fn validate(&self, cmd: &Command) -> Result<(), CommandError> {
        if self.phase != Phase::Command {
            return Err(CommandError::Over);
        }
        if self.current != Some(cmd.actor) {
            return Err(CommandError::NotYourTurn {
                current: self.current,
            });
        }
        let me = &self.allies[cmd.actor];
        let skill = me.skills.get(cmd.skill).ok_or(CommandError::NoSkill)?;
        if me.cooldowns[cmd.skill] > 0 {
            return Err(CommandError::Cooldown);
        }
        if cmd.skill > 0 && me.has(StatusKind::Silence) {
            return Err(CommandError::Silenced);
        }
        if skill.target.single() {
            let t = cmd.target.ok_or(CommandError::NeedTarget)?;
            let side = if skill.target.hostile() {
                Side::Enemy
            } else {
                Side::Ally
            };
            if !self.side(side).get(t).is_some_and(|u| u.alive()) {
                return Err(CommandError::BadTarget);
            }
            if skill.target.hostile() {
                if let Some((Side::Enemy, src)) = me.taunted_by() {
                    if self.enemies[src].alive() && src != t {
                        return Err(CommandError::Taunted { target: src });
                    }
                }
            }
        } else if cmd.target.is_some() {
            return Err(CommandError::BadTarget);
        }
        if let Some(r) = cmd.relay {
            if r == cmd.actor || !self.allies.get(r).is_some_and(|u| u.alive()) {
                return Err(CommandError::BadRelay);
            }
            if self.cheer < self.rules.relay_cost {
                return Err(CommandError::NoCheer);
            }
        }
        Ok(())
    }

    pub fn apply(&mut self, cmd: &Command) -> Result<(), CommandError> {
        self.validate(cmd)?;
        self.log.push(cmd.clone());
        let me = (Side::Ally, cmd.actor);
        self.current = None;
        if cmd.relay.is_some() {
            self.cheer -= self.rules.relay_cost;
        }
        self.use_skill(me, cmd.skill, cmd.target, None);
        self.cheer = (self.cheer + 1).min(self.rules.cheer_max);
        self.events.push(Event::Cheer { value: self.cheer });
        if let Some(r) = cmd.relay {
            self.relay = Some(r);
            self.events.push(Event::Relay {
                from: cmd.actor,
                to: r,
            });
        }
        self.end_turn(me);
        self.settle();
        self.advance();
        Ok(())
    }

    // ---- 流程 ----

    fn start_wave(&mut self, w: usize) {
        self.wave = w;
        self.enemies = self.waves[w].clone();
        for u in &mut self.enemies {
            u.gauge = self.rng.below(START_GAUGE) as i64;
        }
        self.events.push(Event::Wave { wave: w });
        self.battle_start_passives();
    }

    /// 打倒一整波、全滅、超過回合數
    fn settle(&mut self) {
        if self.phase != Phase::Command {
            return;
        }
        if self.allies.iter().all(|u| !u.alive()) || self.turns >= MAX_TURNS {
            self.phase = Phase::Lost;
            self.current = None;
            self.events.push(Event::Lost);
        } else if self.enemies.iter().all(|u| !u.alive()) {
            if self.wave + 1 < self.waves.len() {
                self.start_wave(self.wave + 1);
            } else {
                self.phase = Phase::Won;
                self.current = None;
                self.events.push(Event::Won);
            }
        }
    }

    /// 跑到下一次輪到我方（或結束）
    fn advance(&mut self) {
        while self.phase == Phase::Command && self.current.is_none() {
            let next = GaugeScheduler.next(
                &mut self.allies,
                &mut self.enemies,
                &mut self.relay,
                &self.rules.status,
            );
            let Some(w) = next else {
                self.settle();
                return;
            };
            self.turns += 1;
            self.events.push(Event::Turn {
                side: w.0,
                unit: w.1,
            });
            self.begin_turn(w);
            if self.unit(w).alive() {
                if self.unit(w).has(StatusKind::Stun) {
                    self.events.push(Event::Stunned {
                        side: w.0,
                        unit: w.1,
                    });
                    self.end_turn(w);
                } else if w.0 == Side::Ally {
                    self.ally_turns += 1;
                    self.current = Some(w.1);
                    self.acting = Some(w);
                } else {
                    self.enemy_turn(w.1);
                    self.end_turn(w);
                }
            }
            self.settle();
        }
    }
}
