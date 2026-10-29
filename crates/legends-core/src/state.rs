//! Battle State：戰場上每個單位的狀態。整份可以序列化給畫面看，也拿來算 digest。

use crate::data::{
    Element, Passive, PhaseDef, Role, SkillDef, StatKey, Stats, StatusKind, StatusRule,
};
use serde::{Deserialize, Serialize};

/// 行動值滿了就出手
pub const GAUGE_FULL: i64 = 1000;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Side {
    Ally,
    Enemy,
}

impl Side {
    pub fn other(self) -> Side {
        match self {
            Side::Ally => Side::Enemy,
            Side::Enemy => Side::Ally,
        }
    }
}

/// 某一邊的第幾個
pub type Who = (Side, usize);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct Status {
    pub kind: StatusKind,
    /// 還剩幾個自己的回合
    pub turns: u8,
    /// 灼燒的每回合傷害
    pub value: i64,
    /// 誰上的（挑釁要知道打誰）
    pub source: Option<Who>,
    /// 在自己回合上身的：這個回合結束時不扣
    #[serde(skip)]
    pub fresh: bool,
}

/// 敵人蓄力中的大招
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct Charge {
    pub slot: usize,
    /// 還要幾個自己的回合才發動（0＝下一次輪到就發動）
    pub turns: u8,
}

#[derive(Clone, Debug, Serialize)]
pub struct Unit {
    /// 角色 id 或敵人 id
    pub id: String,
    /// 系列角色 id（只有我方）
    pub character: Option<String>,
    pub element: Element,
    pub role: Option<Role>,
    pub boss: bool,
    pub max_hp: i64,
    pub hp: i64,
    pub shield: i64,
    pub shield_turns: u8,
    /// 能力值（含裝備、共鳴、環境；不含增減益）
    pub stats: Stats,
    pub gauge: i64,
    /// 技能 id（畫面用）與冷卻
    pub skill_ids: Vec<String>,
    pub cooldowns: Vec<u8>,
    pub statuses: Vec<Status>,
    pub toughness: u32,
    pub max_toughness: u32,
    pub charge: Option<Charge>,
    /// 已經進到第幾個階段（0＝還沒換）
    pub phase: usize,
    /// 已經套用強化的技能
    #[serde(skip)]
    pub skills: Vec<SkillDef>,
    #[serde(skip)]
    pub passives: Vec<Passive>,
    #[serde(skip)]
    pub phases: Vec<PhaseDef>,
}

impl Unit {
    pub fn alive(&self) -> bool {
        self.hp > 0
    }

    pub fn has(&self, k: StatusKind) -> bool {
        self.statuses.iter().any(|s| s.kind == k)
    }

    pub fn hp_pct(&self) -> i64 {
        self.hp * 100 / self.max_hp.max(1)
    }

    /// 算上增減益後的能力值
    pub fn eff(&self, k: StatKey, r: &StatusRule) -> i64 {
        let base = self.stats.get(k);
        let mut pct = 100;
        let mut add = 0;
        for s in &self.statuses {
            match (s.kind, k) {
                (StatusKind::AtkUp, StatKey::Atk) => pct += r.atk_up,
                (StatusKind::AtkDown, StatKey::Atk) => pct -= r.atk_down,
                (StatusKind::DefUp, StatKey::Def) => pct += r.def_up,
                (StatusKind::DefDown, StatKey::Def) => pct -= r.def_down,
                (StatusKind::SpdUp, StatKey::Spd) => pct += r.spd_up,
                (StatusKind::SpdDown, StatKey::Spd) => pct -= r.spd_down,
                (StatusKind::CritUp, StatKey::Crit) => add += r.crit_up,
                (StatusKind::HitDown, StatKey::Hit) => add -= r.hit_down,
                _ => {}
            }
        }
        if k.is_rate() {
            base + add
        } else {
            (base * pct.max(10) / 100).max(1)
        }
    }

    /// 這一招現在能不能用
    pub fn can_use(&self, slot: usize) -> bool {
        slot < self.skills.len()
            && self.cooldowns[slot] == 0
            && (slot == 0 || !self.has(StatusKind::Silence))
    }

    /// 挑釁它的人
    pub fn taunted_by(&self) -> Option<Who> {
        self.statuses
            .iter()
            .find(|s| s.kind == StatusKind::Taunt)
            .and_then(|s| s.source)
    }
}
