//! 隊伍成員（Server 庫存裡的一個角色＋穿的裝備）換算成戰鬥能力值。
//!
//! 基礎 =（基礎 + 成長 ×（等級 − 1））×（100 + 突破 × 5 + 羈絆）% ＋ 規則的基礎值（暴擊等）
//! 最終 =（基礎 + 固定加成）×（100 + 百分比加成）%；暴擊這類「率」的百分比直接加百分點。
//! 加成來自裝備主副屬性、套裝、屬性共鳴、環境條件。

use crate::data::{Catalog, CostumeDef, Passive, Slot, StatMod, Stats};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const TEAM_SIZE: usize = 4;
pub const MAX_ASCENSION: u8 = 5;
pub const MAX_BOND: u8 = 10;

/// 等級上限看突破：0 突破 20 級，每突破一次 +8，滿突破 60 級
pub fn level_cap(ascension: u8) -> u32 {
    20 + 8 * ascension as u32
}

/// 一件裝備。數值是 Server 依強化等級算好的最終值，引擎直接用
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Gear {
    pub slot: Slot,
    pub set: String,
    pub main: StatMod,
    #[serde(default)]
    pub subs: Vec<StatMod>,
}

fn one() -> [u8; 3] {
    [1, 1, 1]
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Member {
    pub costume: String,
    pub level: u32,
    #[serde(default)]
    pub ascension: u8,
    /// 角色的羈絆等級（同一位角色的所有版本共用）
    #[serde(default)]
    pub bond: u8,
    /// 三招的強化等級（從 1 開始）
    #[serde(default = "one")]
    pub skills: [u8; 3],
    #[serde(default)]
    pub gear: Vec<Gear>,
}

impl Member {
    pub fn new(costume: &str, level: u32) -> Self {
        Member {
            costume: costume.into(),
            level,
            ascension: 0,
            bond: 0,
            skills: one(),
            gear: Vec::new(),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum SetupError {
    EmptyTeam,
    TeamTooLarge,
    UnknownCostume(String),
    /// 同一位角色只能上一個版本
    DuplicateCharacter(String),
    BadLevel(String),
    BadSkillLevel(String),
    BadGear(String),
    UnknownStage(String),
}

/// 累積的加成
#[derive(Clone, Debug, Default)]
pub struct Bonus {
    add: Stats,
    pct: Stats,
}

impl Bonus {
    pub fn push(&mut self, m: &StatMod) {
        if m.stat.is_rate() {
            *self.add.get_mut(m.stat) += m.add + m.pct;
        } else {
            *self.add.get_mut(m.stat) += m.add;
            *self.pct.get_mut(m.stat) += m.pct;
        }
    }

    pub fn apply(&self, base: Stats) -> Stats {
        let mut out = Stats::default();
        for k in Stats::KEYS {
            let v = base.get(k) + self.add.get(k);
            *out.get_mut(k) = if k.is_rate() {
                v
            } else {
                v * (100 + self.pct.get(k)).max(10) / 100
            };
        }
        out
    }
}

/// 等級、突破、羈絆算出的基礎能力值（還沒有裝備）
pub fn base_stats(cat: &Catalog, def: &CostumeDef, m: &Member) -> Stats {
    let lv = m.level as i64 - 1;
    let pct = 100 + 5 * m.ascension as i64 + m.bond as i64;
    let mut s = Stats::default();
    for k in Stats::KEYS {
        let grown = def.base.get(k) + def.growth.get(k) * lv;
        // 率不吃等級倍率
        *s.get_mut(k) = cat.rules.base.get(k)
            + if k.is_rate() {
                grown
            } else {
                grown * pct / 100
            };
    }
    s
}

/// 裝備主副屬性與套裝的加成，以及套裝帶來的被動
pub fn gear_bonus(cat: &Catalog, m: &Member, bonus: &mut Bonus) -> Vec<Passive> {
    let mut count: BTreeMap<&str, u8> = BTreeMap::new();
    for g in &m.gear {
        bonus.push(&g.main);
        for s in &g.subs {
            bonus.push(s);
        }
        *count.entry(&g.set).or_default() += 1;
    }
    let mut passives = Vec::new();
    for (id, n) in count {
        let Some(set) = cat.sets.get(id) else {
            continue;
        };
        // 2 件套湊 4 件給兩次
        for _ in 0..n / set.pieces {
            for s in &set.stats {
                bonus.push(s);
            }
            if let Some(p) = &set.passive {
                passives.push(p.clone());
            }
        }
    }
    passives
}

/// 畫面用：只看自己（等級、裝備）的最終能力值，不含共鳴與環境
pub fn member_stats(cat: &Catalog, m: &Member) -> Option<Stats> {
    let def = cat.costumes.get(&m.costume)?;
    let mut bonus = Bonus::default();
    gear_bonus(cat, m, &mut bonus);
    Some(bonus.apply(base_stats(cat, def, m)))
}

pub fn validate_team(cat: &Catalog, team: &[Member]) -> Result<(), SetupError> {
    if team.is_empty() {
        return Err(SetupError::EmptyTeam);
    }
    if team.len() > TEAM_SIZE {
        return Err(SetupError::TeamTooLarge);
    }
    let mut seen = Vec::new();
    for m in team {
        let def = cat
            .costumes
            .get(&m.costume)
            .ok_or_else(|| SetupError::UnknownCostume(m.costume.clone()))?;
        if seen.contains(&def.character) {
            return Err(SetupError::DuplicateCharacter(def.character.clone()));
        }
        seen.push(def.character.clone());
        if m.level == 0
            || m.ascension > MAX_ASCENSION
            || m.bond > MAX_BOND
            || m.level > level_cap(m.ascension)
        {
            return Err(SetupError::BadLevel(m.costume.clone()));
        }
        for (i, &lv) in m.skills.iter().enumerate() {
            if lv == 0 || lv > cat.skills[&def.skills[i]].max_level() {
                return Err(SetupError::BadSkillLevel(m.costume.clone()));
            }
        }
        let mut slots = Vec::new();
        for g in &m.gear {
            if slots.contains(&g.slot) || !cat.sets.contains_key(&g.set) || g.subs.len() > 4 {
                return Err(SetupError::BadGear(m.costume.clone()));
            }
            slots.push(g.slot);
        }
    }
    Ok(())
}
