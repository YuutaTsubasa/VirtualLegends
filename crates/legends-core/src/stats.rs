//! 隊伍成員（Server 庫存裡的一套衣裝）換算成戰鬥能力值。
//! 能力值 =（基礎 + 成長 ×（等級 − 1））×（100 + 突破 × 5 + 羈絆）%

use crate::data::{Catalog, CostumeDef, Stats};
use serde::{Deserialize, Serialize};

pub const TEAM_SIZE: usize = 4;
pub const MAX_ASCENSION: u8 = 5;
pub const MAX_BOND: u8 = 10;

/// 等級上限看突破：0 突破 20 級，每突破一次 +8，滿突破 60 級
pub fn level_cap(ascension: u8) -> u32 {
    20 + 8 * ascension as u32
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Member {
    pub costume: String,
    pub level: u32,
    #[serde(default)]
    pub ascension: u8,
    /// 角色的羈絆等級（同一位角色的所有衣裝共用）
    #[serde(default)]
    pub bond: u8,
}

#[derive(Debug, PartialEq, Eq)]
pub enum SetupError {
    EmptyTeam,
    TeamTooLarge,
    UnknownCostume(String),
    /// 同一位角色只能上一套衣裝
    DuplicateCharacter(String),
    BadLevel(String),
    UnknownStage(String),
}

pub fn member_stats(def: &CostumeDef, m: &Member) -> Stats {
    let lv = m.level as i64 - 1;
    let pct = 100 + 5 * m.ascension as i64 + m.bond as i64;
    let f = |base: i64, g: i64| (base + g * lv) * pct / 100;
    Stats {
        hp: f(def.base.hp, def.growth.hp),
        atk: f(def.base.atk, def.growth.atk),
        def: f(def.base.def, def.growth.def),
        spd: f(def.base.spd, def.growth.spd),
    }
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
    }
    Ok(())
}
