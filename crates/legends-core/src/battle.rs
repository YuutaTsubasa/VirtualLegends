//! 牌氣回合制戰鬥。
//!
//! 每回合開始：我方從牌山摸 3 張進共用手牌（最多 8 張），每隻敵人各摸 1 張（最多 3 張，滿了丟最舊的）。
//! 敵人三張牌組成順子或刻子時「聽牌」，輪到它就放大招——玩家可以先用妨礙技拿掉它的牌。
//!
//! 我方每人每回合行動一次，**順序由玩家決定**：一次送一個 `Command`（誰、做什麼）。
//! 速度不決定我方順序，而是決定敵人插在哪：輪到某位我方角色前，所有比他快、這回合還沒動的敵人先動。
//! 我方全員動完後，剩下的敵人再動，接著看合技、減少增益回合，進下一回合。
//! 指令送出時就檢查；若輪到他之前，比他快的敵人偷走了要花的牌，這次行動退回普攻（`Event::Fizzle`）。
//!
//! - 普攻：不花牌，打完多拿 1 張自己屬性的牌。
//! - 技能：花 cost 張自己屬性的牌（字牌可以代替任何花色；字屬性的衣裝可以花任何牌）。
//! - 大招：花 3 張組成順子或刻子的牌，威力見 `Meld::power`。
//! - 合技：同一回合我方花掉的牌湊得出一順＋一刻，回合結束時全隊對所有敵人追加攻擊。

use crate::data::{BuffStat, Catalog, Effect, EnemyDef, Role, SkillDef, Target};
use crate::rng::Rng;
use crate::stats::{member_stats, validate_team, Member, SetupError};
use crate::tiles::{has_combo, meld_of, Suit, Tile, Wall};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const MAX_ROUNDS: u32 = 30;
pub const HAND_LIMIT: usize = 8;
pub const DRAW_PER_ROUND: usize = 3;
pub const ENEMY_HAND: usize = 3;
/// 合技的威力（全隊平均攻擊力的百分比）
pub const COMBO_POWER: i64 = 150;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BattleSetup {
    pub stage: String,
    pub team: Vec<Member>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Side {
    Ally,
    Enemy,
}

impl Side {
    fn other(self) -> Side {
        match self {
            Side::Ally => Side::Enemy,
            Side::Enemy => Side::Ally,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct Buff {
    pub stat: BuffStat,
    pub pct: i64,
    pub turns: u8,
}

#[derive(Clone, Debug, Serialize)]
pub struct Fighter {
    /// 衣裝 id 或敵人 id
    pub id: String,
    pub suit: Suit,
    pub role: Option<Role>,
    pub max_hp: i64,
    pub hp: i64,
    pub shield: i64,
    pub atk: i64,
    pub def: i64,
    pub spd: i64,
    pub buffs: Vec<Buff>,
    pub skills: Vec<String>,
    /// 只有敵人用：自己的牌
    pub hand: Vec<Tile>,
    pub acted: bool,
}

impl Fighter {
    pub fn alive(&self) -> bool {
        self.hp > 0
    }

    /// 算上增益後的能力值（最低剩 10%）
    pub fn stat(&self, s: BuffStat) -> i64 {
        let base = match s {
            BuffStat::Atk => self.atk,
            BuffStat::Def => self.def,
            BuffStat::Spd => self.spd,
        };
        let pct: i64 = 100
            + self
                .buffs
                .iter()
                .filter(|b| b.stat == s)
                .map(|b| b.pct)
                .sum::<i64>();
        base * pct.max(10) / 100
    }

    /// 敵人的牌組成順子或刻子：輪到它時放大招
    pub fn ready(&self) -> bool {
        self.skills.len() > 1
            && self.hand.len() == ENEMY_HAND
            && meld_of([self.hand[0], self.hand[1], self.hand[2]]).is_some()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Action {
    Basic {
        target: Option<usize>,
    },
    /// tiles 是共用手牌的位置
    Skill {
        target: Option<usize>,
        tiles: Vec<usize>,
    },
    Ultimate {
        target: Option<usize>,
        tiles: [usize; 3],
    },
}

impl Action {
    pub fn target(&self) -> Option<usize> {
        match self {
            Action::Basic { target }
            | Action::Skill { target, .. }
            | Action::Ultimate { target, .. } => *target,
        }
    }

    fn slot(&self) -> usize {
        match self {
            Action::Basic { .. } => 0,
            Action::Skill { .. } => 1,
            Action::Ultimate { .. } => 2,
        }
    }

    fn tiles(&self) -> Vec<usize> {
        match self {
            Action::Basic { .. } => Vec::new(),
            Action::Skill { tiles, .. } => tiles.clone(),
            Action::Ultimate { tiles, .. } => tiles.to_vec(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Command {
    pub actor: usize,
    pub action: Action,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum CommandError {
    Finished,
    NoSuchActor,
    ActorDown,
    AlreadyActed,
    BadTiles,
    WrongSuit,
    WrongCost,
    NotMeld,
    BadTarget,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Phase {
    /// 等玩家送下一個指令
    Command,
    Won,
    Lost,
}

/// 給 Client 演出用的事件，`take_events` 取走
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    Round {
        n: u32,
    },
    Wave {
        index: usize,
    },
    Draw {
        tiles: Vec<Tile>,
    },
    EnemyDraw {
        slot: usize,
        tile: Tile,
    },
    /// 敵人聽牌：輪到它時會放大招
    Ready {
        slot: usize,
    },
    Act {
        side: Side,
        slot: usize,
        skill: String,
    },
    Spend {
        tiles: Vec<Tile>,
    },
    Gain {
        tile: Tile,
    },
    Damage {
        side: Side,
        slot: usize,
        amount: i64,
        hp: i64,
    },
    Heal {
        side: Side,
        slot: usize,
        amount: i64,
        hp: i64,
    },
    Shield {
        side: Side,
        slot: usize,
        amount: i64,
    },
    Buff {
        side: Side,
        slot: usize,
        stat: BuffStat,
        pct: i64,
        turns: u8,
    },
    Steal {
        side: Side,
        slot: usize,
        tiles: Vec<Tile>,
    },
    Down {
        side: Side,
        slot: usize,
    },
    /// 要花的牌在他出手前被偷走了，改成普攻
    Fizzle {
        slot: usize,
    },
    Combo,
    Won,
    Lost,
}

#[derive(Clone, Debug)]
pub struct Battle {
    skills: BTreeMap<String, SkillDef>,
    waves: Vec<Vec<EnemyDef>>,
    scale: i64,
    wave: usize,
    rng: Rng,
    wall: Wall,
    hand: Vec<Tile>,
    allies: Vec<Fighter>,
    enemies: Vec<Fighter>,
    round: u32,
    phase: Phase,
    /// 這回合我方花掉的牌（看合技）
    spent: Vec<Tile>,
    events: Vec<Event>,
    log: Vec<Command>,
}

impl Battle {
    pub fn new(cat: &Catalog, setup: &BattleSetup, seed: u64) -> Result<Self, SetupError> {
        let stage = cat
            .stages
            .get(&setup.stage)
            .ok_or_else(|| SetupError::UnknownStage(setup.stage.clone()))?;
        validate_team(cat, &setup.team)?;
        // Catalog::from_json 已經檢查過參照都存在
        let mut skills = BTreeMap::new();
        let mut keep = |id: &String| {
            skills.insert(id.clone(), cat.skills[id].clone());
        };
        let allies = setup
            .team
            .iter()
            .map(|m| {
                let def = &cat.costumes[&m.costume];
                def.skills.iter().for_each(&mut keep);
                let s = member_stats(def, m);
                Fighter {
                    id: def.id.clone(),
                    suit: def.suit,
                    role: Some(def.role),
                    max_hp: s.hp,
                    hp: s.hp,
                    shield: 0,
                    atk: s.atk,
                    def: s.def,
                    spd: s.spd,
                    buffs: Vec::new(),
                    skills: def.skills.to_vec(),
                    hand: Vec::new(),
                    acted: false,
                }
            })
            .collect();
        let waves: Vec<Vec<EnemyDef>> = stage
            .waves
            .iter()
            .map(|w| w.iter().map(|id| cat.enemies[id].clone()).collect())
            .collect();
        waves
            .iter()
            .flatten()
            .flat_map(|e| e.skills.iter())
            .for_each(&mut keep);

        let mut rng = Rng::new(seed);
        let wall = Wall::new(&mut rng);
        let mut b = Battle {
            skills,
            waves,
            scale: stage.scale,
            wave: 0,
            rng,
            wall,
            hand: Vec::new(),
            allies,
            enemies: Vec::new(),
            round: 0,
            phase: Phase::Command,
            spent: Vec::new(),
            events: Vec::new(),
            log: Vec::new(),
        };
        b.spawn_wave(0);
        b.start_round();
        Ok(b)
    }

    // ---- 讀取 ----

    pub fn phase(&self) -> Phase {
        self.phase
    }
    pub fn round(&self) -> u32 {
        self.round
    }
    pub fn wave(&self) -> usize {
        self.wave
    }
    pub fn wave_count(&self) -> usize {
        self.waves.len()
    }
    pub fn hand(&self) -> &[Tile] {
        &self.hand
    }
    pub fn allies(&self) -> &[Fighter] {
        &self.allies
    }
    pub fn enemies(&self) -> &[Fighter] {
        &self.enemies
    }
    pub fn skill(&self, id: &str) -> &SkillDef {
        &self.skills[id]
    }
    pub fn log(&self) -> &[Command] {
        &self.log
    }
    pub fn take_events(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.events)
    }
    /// 這回合還能行動的我方
    pub fn pending(&self) -> Vec<usize> {
        (0..self.allies.len())
            .filter(|&i| self.allies[i].alive() && !self.allies[i].acted)
            .collect()
    }

    /// 狀態的雜湊。Client 與 Server 跑同一場，每一步的 digest 都要一樣。
    pub fn digest(&self) -> u64 {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        let mut put = |x: u64| {
            for b in x.to_le_bytes() {
                h ^= b as u64;
                h = h.wrapping_mul(0x0000_0100_0000_01b3);
            }
        };
        put(self.rng.state());
        put(self.round as u64);
        put(self.wave as u64);
        put(self.phase as u64);
        put(self.wall.len() as u64);
        for t in &self.hand {
            put(((t.suit as u64) << 8) | t.rank as u64);
        }
        for f in self.allies.iter().chain(&self.enemies) {
            put(f.hp as u64);
            put(f.shield as u64);
            put(f.buffs.len() as u64);
            put(f.hand.len() as u64);
        }
        h
    }

    // ---- 指令 ----

    pub fn validate(&self, cmd: &Command) -> Result<(), CommandError> {
        if self.phase != Phase::Command {
            return Err(CommandError::Finished);
        }
        let a = self
            .allies
            .get(cmd.actor)
            .ok_or(CommandError::NoSuchActor)?;
        if !a.alive() {
            return Err(CommandError::ActorDown);
        }
        if a.acted {
            return Err(CommandError::AlreadyActed);
        }
        let skill = &self.skills[&a.skills[cmd.action.slot()]];
        let tiles = cmd.action.tiles();
        let mut sorted = tiles.clone();
        sorted.sort_unstable();
        sorted.dedup();
        if sorted.len() != tiles.len() || tiles.iter().any(|&i| i >= self.hand.len()) {
            return Err(CommandError::BadTiles);
        }
        match &cmd.action {
            Action::Basic { .. } => {}
            Action::Skill { .. } => {
                if tiles.len() != skill.cost as usize {
                    return Err(CommandError::WrongCost);
                }
                let ok =
                    |t: Tile| a.suit == Suit::Honor || t.suit == Suit::Honor || t.suit == a.suit;
                if !tiles.iter().all(|&i| ok(self.hand[i])) {
                    return Err(CommandError::WrongSuit);
                }
            }
            Action::Ultimate { tiles, .. } => {
                meld_of(tiles.map(|i| self.hand[i])).ok_or(CommandError::NotMeld)?;
            }
        }
        let alive_at = |side: &[Fighter], i: Option<usize>| {
            i.and_then(|i| side.get(i)).is_some_and(|f| f.alive())
        };
        let ok = match skill.target {
            Target::Enemy => alive_at(&self.enemies, cmd.action.target()),
            Target::Ally => alive_at(&self.allies, cmd.action.target()),
            _ => true,
        };
        if !ok {
            return Err(CommandError::BadTarget);
        }
        Ok(())
    }

    pub fn apply(&mut self, cmd: &Command) -> Result<(), CommandError> {
        self.validate(cmd)?;
        self.log.push(cmd.clone());
        let spd = self.allies[cmd.actor].stat(BuffStat::Spd);
        self.enemies_act(Some(spd));
        if self.phase != Phase::Command {
            return Ok(());
        }
        // 被比他快的敵人打倒了：這次行動就沒了
        if self.allies[cmd.actor].alive() {
            // 比他快的敵人偷走了要花的牌：退回普攻
            let fallback;
            let still_ok = self.validate(cmd).is_ok();
            self.allies[cmd.actor].acted = true;
            let cmd = if still_ok {
                cmd
            } else {
                self.events.push(Event::Fizzle { slot: cmd.actor });
                fallback = Command {
                    actor: cmd.actor,
                    action: Action::Basic {
                        target: cmd.action.target(),
                    },
                };
                &fallback
            };
            self.ally_act(cmd);
            self.check_end();
        } else {
            self.allies[cmd.actor].acted = true;
        }
        if self.phase == Phase::Command && self.pending().is_empty() {
            self.end_round();
        }
        Ok(())
    }

    // ---- 回合 ----

    fn spawn_wave(&mut self, index: usize) {
        let scale = self.scale;
        self.enemies = self.waves[index]
            .iter()
            .map(|e| {
                let s = |v: i64| v * scale / 100;
                Fighter {
                    id: e.id.clone(),
                    suit: e.suit,
                    role: None,
                    max_hp: s(e.stats.hp),
                    hp: s(e.stats.hp),
                    shield: 0,
                    atk: s(e.stats.atk),
                    def: s(e.stats.def),
                    spd: e.stats.spd,
                    buffs: Vec::new(),
                    skills: e.skills.clone(),
                    hand: Vec::new(),
                    // 中途進場的這回合不動
                    acted: self.round > 0,
                }
            })
            .collect();
        self.events.push(Event::Wave { index });
    }

    fn start_round(&mut self) {
        if self.round >= MAX_ROUNDS {
            self.finish(false);
            return;
        }
        self.round += 1;
        self.spent.clear();
        self.events.push(Event::Round { n: self.round });
        for f in self.allies.iter_mut().chain(self.enemies.iter_mut()) {
            f.acted = false;
        }
        let mut drawn = Vec::new();
        for _ in 0..DRAW_PER_ROUND {
            if self.hand.len() < HAND_LIMIT {
                let t = self.wall.draw(&mut self.rng);
                self.hand.push(t);
                drawn.push(t);
            }
        }
        self.events.push(Event::Draw { tiles: drawn });
        for slot in 0..self.enemies.len() {
            if !self.enemies[slot].alive() {
                continue;
            }
            let tile = self.wall.draw(&mut self.rng);
            let e = &mut self.enemies[slot];
            if e.hand.len() >= ENEMY_HAND {
                e.hand.remove(0);
            }
            e.hand.push(tile);
            self.events.push(Event::EnemyDraw { slot, tile });
            if self.enemies[slot].ready() {
                self.events.push(Event::Ready { slot });
            }
        }
    }

    fn end_round(&mut self) {
        self.enemies_act(None);
        if self.phase != Phase::Command {
            return;
        }
        if has_combo(&self.spent) {
            self.events.push(Event::Combo);
            let alive: Vec<&Fighter> = self.allies.iter().filter(|a| a.alive()).collect();
            let atk = alive.iter().map(|a| a.stat(BuffStat::Atk)).sum::<i64>()
                / alive.len().max(1) as i64;
            for slot in 0..self.enemies.len() {
                if self.enemies[slot].alive() {
                    self.hit_raw(atk, Suit::Honor, Side::Enemy, slot, COMBO_POWER);
                }
            }
            self.check_end();
            if self.phase != Phase::Command {
                return;
            }
        }
        for f in self.allies.iter_mut().chain(self.enemies.iter_mut()) {
            f.buffs.iter_mut().for_each(|b| b.turns -= 1);
            f.buffs.retain(|b| b.turns > 0);
        }
        self.start_round();
    }

    /// faster_than = Some(spd)：只讓比 spd 快的敵人動；None：剩下的全動。快的先動，同速看位置。
    fn enemies_act(&mut self, faster_than: Option<i64>) {
        while self.phase == Phase::Command {
            let next = (0..self.enemies.len())
                .filter(|&i| {
                    let e = &self.enemies[i];
                    e.alive() && !e.acted && faster_than.is_none_or(|s| e.stat(BuffStat::Spd) > s)
                })
                .max_by_key(|&i| (self.enemies[i].stat(BuffStat::Spd), std::cmp::Reverse(i)));
            let Some(slot) = next else { break };
            self.enemy_act(slot);
        }
    }

    fn enemy_act(&mut self, slot: usize) {
        self.enemies[slot].acted = true;
        let e = &self.enemies[slot];
        let (skill, mult) = if e.ready() {
            let m = meld_of([e.hand[0], e.hand[1], e.hand[2]]).expect("ready 已經檢查過");
            let s = (e.skills[1].clone(), m.power());
            self.enemies[slot].hand.clear();
            s
        } else {
            (e.skills[0].clone(), 100)
        };
        // 有守護型活著時，一半的機率打他
        let alive: Vec<usize> = (0..self.allies.len())
            .filter(|&i| self.allies[i].alive())
            .collect();
        let guards: Vec<usize> = alive
            .iter()
            .copied()
            .filter(|&i| self.allies[i].role == Some(Role::Guardian))
            .collect();
        let pool = if !guards.is_empty() && self.rng.below(2) == 0 {
            guards
        } else {
            alive
        };
        let target = pool[self.rng.below(pool.len() as u64) as usize];
        self.events.push(Event::Act {
            side: Side::Enemy,
            slot,
            skill: skill.clone(),
        });
        self.use_skill(Side::Enemy, slot, &skill, Some(target), mult);
        self.check_end();
    }

    fn ally_act(&mut self, cmd: &Command) {
        let actor = cmd.actor;
        let skill = self.allies[actor].skills[cmd.action.slot()].clone();
        let mult = match &cmd.action {
            Action::Ultimate { tiles, .. } => meld_of(tiles.map(|i| self.hand[i]))
                .expect("validate 已經檢查過")
                .power(),
            _ => 100,
        };
        let mut idx = cmd.action.tiles();
        if !idx.is_empty() {
            idx.sort_unstable_by(|a, b| b.cmp(a));
            let mut spent: Vec<Tile> = idx.iter().map(|&i| self.hand.remove(i)).collect();
            spent.reverse();
            self.spent.extend(&spent);
            self.events.push(Event::Spend { tiles: spent });
        }
        self.events.push(Event::Act {
            side: Side::Ally,
            slot: actor,
            skill: skill.clone(),
        });
        self.use_skill(Side::Ally, actor, &skill, cmd.action.target(), mult);
        if matches!(cmd.action, Action::Basic { .. }) && self.hand.len() < HAND_LIMIT {
            let suit = self.allies[actor].suit;
            let tile = Tile::new(suit, 1 + self.rng.below(suit.ranks() as u64) as u8);
            self.hand.push(tile);
            self.events.push(Event::Gain { tile });
        }
    }

    // ---- 效果 ----

    fn side(&self, side: Side) -> &Vec<Fighter> {
        match side {
            Side::Ally => &self.allies,
            Side::Enemy => &self.enemies,
        }
    }

    fn side_mut(&mut self, side: Side) -> &mut Vec<Fighter> {
        match side {
            Side::Ally => &mut self.allies,
            Side::Enemy => &mut self.enemies,
        }
    }

    fn targets(
        &self,
        side: Side,
        slot: usize,
        target: Target,
        chosen: Option<usize>,
    ) -> Vec<(Side, usize)> {
        let alive = |s: Side| -> Vec<(Side, usize)> {
            (0..self.side(s).len())
                .filter(|&i| self.side(s)[i].alive())
                .map(|i| (s, i))
                .collect()
        };
        let single = |s: Side| -> Vec<(Side, usize)> {
            match chosen {
                Some(i) if self.side(s).get(i).is_some_and(|f| f.alive()) => vec![(s, i)],
                _ => alive(s).into_iter().take(1).collect(),
            }
        };
        match target {
            Target::Enemy => single(side.other()),
            Target::AllEnemies => alive(side.other()),
            Target::Ally => single(side),
            Target::AllAllies => alive(side),
            Target::Myself => vec![(side, slot)],
        }
    }

    fn use_skill(
        &mut self,
        side: Side,
        slot: usize,
        skill: &str,
        chosen: Option<usize>,
        mult: i64,
    ) {
        let def = self.skills[skill].clone();
        let targets = self.targets(side, slot, def.target, chosen);
        let atk = self.side(side)[slot].stat(BuffStat::Atk);
        let suit = self.side(side)[slot].suit;
        for eff in &def.effects {
            match *eff {
                Effect::Damage { power } => {
                    for &(ts, ti) in &targets {
                        if self.side(ts)[ti].alive() {
                            self.hit_raw(atk, suit, ts, ti, power * mult / 100);
                        }
                    }
                }
                Effect::Heal { power } => {
                    for &(ts, ti) in &targets {
                        let f = &mut self.side_mut(ts)[ti];
                        if f.alive() {
                            let amount = (atk * power * mult / 10000).min(f.max_hp - f.hp);
                            f.hp += amount;
                            let hp = f.hp;
                            self.events.push(Event::Heal {
                                side: ts,
                                slot: ti,
                                amount,
                                hp,
                            });
                        }
                    }
                }
                Effect::Shield { power } => {
                    for &(ts, ti) in &targets {
                        let f = &mut self.side_mut(ts)[ti];
                        if f.alive() {
                            let amount =
                                (atk * power * mult / 10000).min(f.max_hp - f.shield).max(0);
                            f.shield += amount;
                            self.events.push(Event::Shield {
                                side: ts,
                                slot: ti,
                                amount,
                            });
                        }
                    }
                }
                Effect::Buff { stat, pct, turns } => {
                    for &(ts, ti) in &targets {
                        let f = &mut self.side_mut(ts)[ti];
                        if f.alive() {
                            // 同一項、同方向的增益不疊加，改成刷新
                            f.buffs
                                .retain(|b| !(b.stat == stat && (b.pct > 0) == (pct > 0)));
                            f.buffs.push(Buff { stat, pct, turns });
                            self.events.push(Event::Buff {
                                side: ts,
                                slot: ti,
                                stat,
                                pct,
                                turns,
                            });
                        }
                    }
                }
                Effect::Steal { count } => match side {
                    Side::Ally => {
                        for &(ts, ti) in &targets {
                            if ts == Side::Enemy {
                                let hand = &mut self.enemies[ti].hand;
                                let n = (count as usize).min(hand.len());
                                let tiles = hand.split_off(hand.len() - n);
                                self.events.push(Event::Steal {
                                    side: ts,
                                    slot: ti,
                                    tiles,
                                });
                            }
                        }
                    }
                    Side::Enemy => {
                        let n = (count as usize).min(self.hand.len());
                        let tiles = self.hand.split_off(self.hand.len() - n);
                        self.events.push(Event::Steal {
                            side: Side::Ally,
                            slot,
                            tiles,
                        });
                    }
                },
                Effect::Draw { count } => {
                    if side == Side::Ally {
                        let mut tiles = Vec::new();
                        for _ in 0..count {
                            if self.hand.len() < HAND_LIMIT {
                                let t = self.wall.draw(&mut self.rng);
                                self.hand.push(t);
                                tiles.push(t);
                            }
                        }
                        self.events.push(Event::Draw { tiles });
                    }
                }
            }
        }
    }

    /// 傷害 = 攻擊 × 威力% × 200／(200 + 防禦) × 屬性倍率 × 浮動 95～105%，先扣護盾
    fn hit_raw(&mut self, atk: i64, suit: Suit, ts: Side, ti: usize, power: i64) {
        let roll = 95 + self.rng.below(11) as i64;
        let f = &self.side(ts)[ti];
        let def = f.stat(BuffStat::Def);
        let aff = Suit::affinity(suit, f.suit);
        let amount = (atk * power / 100 * 200 / (200 + def) * aff / 100 * roll / 100).max(1);
        let f = &mut self.side_mut(ts)[ti];
        let absorbed = amount.min(f.shield);
        f.shield -= absorbed;
        f.hp = (f.hp - (amount - absorbed)).max(0);
        let hp = f.hp;
        self.events.push(Event::Damage {
            side: ts,
            slot: ti,
            amount,
            hp,
        });
        if hp == 0 {
            self.events.push(Event::Down { side: ts, slot: ti });
        }
    }

    fn check_end(&mut self) {
        if self.phase != Phase::Command {
            return;
        }
        if self.allies.iter().all(|a| !a.alive()) {
            self.finish(false);
        } else if self.enemies.iter().all(|e| !e.alive()) {
            if self.wave + 1 < self.waves.len() {
                self.wave += 1;
                self.spawn_wave(self.wave);
            } else {
                self.finish(true);
            }
        }
    }

    fn finish(&mut self, won: bool) {
        self.phase = if won { Phase::Won } else { Phase::Lost };
        self.events.push(if won { Event::Won } else { Event::Lost });
    }

    #[cfg(test)]
    pub(crate) fn set_hand(&mut self, hand: Vec<Tile>) {
        self.hand = hand;
    }

    #[cfg(test)]
    pub(crate) fn set_enemy_hand(&mut self, slot: usize, hand: Vec<Tile>) {
        self.enemies[slot].hand = hand;
    }
}
