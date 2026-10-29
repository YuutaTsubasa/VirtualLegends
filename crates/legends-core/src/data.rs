//! 遊戲資料：衣裝、技能、敵人、關卡。正本是 repo 的 `data/*.json`，Client 與 Server 編進同一份。
//! 技能的目標是「相對施放者」的：Enemy 是對手那一邊，Ally 是自己那一邊。

use crate::tiles::Suit;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stats {
    pub hp: i64,
    pub atk: i64,
    pub def: i64,
    pub spd: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// 攻擊
    Attacker,
    /// 守護：敵人一半的機率先打他
    Guardian,
    /// 支援
    Support,
    /// 妨礙：拿掉敵人的牌、降能力
    Disruptor,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CostumeDef {
    pub id: String,
    /// 系列共通的角色 id（xiaocui、yuehua…）
    pub character: String,
    pub name: String,
    pub rarity: u8,
    pub suit: Suit,
    pub role: Role,
    pub base: Stats,
    /// 每升一級加多少
    pub growth: Stats,
    /// [普攻, 技能, 大招]
    pub skills: [String; 3],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SkillKind {
    Basic,
    Skill,
    Ultimate,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Target {
    Enemy,
    AllEnemies,
    Ally,
    AllAllies,
    #[serde(rename = "self")]
    Myself,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BuffStat {
    Atk,
    Def,
    Spd,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Effect {
    /// 傷害，power 是攻擊力的百分比
    Damage { power: i64 },
    /// 回復施放者攻擊力的 power%
    Heal { power: i64 },
    /// 護盾施放者攻擊力的 power%（最多到最大 HP）
    Shield { power: i64 },
    /// 能力增減 pct%，持續 turns 回合（負數是減益）
    Buff { stat: BuffStat, pct: i64, turns: u8 },
    /// 拿掉對手的牌：我方用是拿掉目標敵人手上的牌，敵人用是拿掉我方共用手牌
    Steal { count: u8 },
    /// 我方從牌山多摸幾張
    Draw { count: u8 },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SkillDef {
    pub id: String,
    pub name: String,
    pub kind: SkillKind,
    /// 技能要花幾張牌（普攻、大招不看這個）
    #[serde(default)]
    pub cost: u8,
    pub target: Target,
    pub effects: Vec<Effect>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EnemyDef {
    pub id: String,
    pub name: String,
    pub suit: Suit,
    pub stats: Stats,
    /// [普攻] 或 [普攻, 大招]；手上三張牌組成順子或刻子時放大招
    pub skills: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StageDef {
    pub id: String,
    pub name: String,
    /// 敵人能力值的倍率（百分比）
    #[serde(default = "hundred")]
    pub scale: i64,
    pub waves: Vec<Vec<String>>,
}

fn hundred() -> i64 {
    100
}

#[derive(Clone, Debug, Default)]
pub struct Catalog {
    pub costumes: BTreeMap<String, CostumeDef>,
    pub skills: BTreeMap<String, SkillDef>,
    pub enemies: BTreeMap<String, EnemyDef>,
    pub stages: BTreeMap<String, StageDef>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum DataError {
    Parse(String),
    Duplicate(String),
    Missing(String),
}

fn by_id<T: serde::de::DeserializeOwned>(
    json: &str,
    id: impl Fn(&T) -> &str,
) -> Result<BTreeMap<String, T>, DataError> {
    let list: Vec<T> = serde_json::from_str(json).map_err(|e| DataError::Parse(e.to_string()))?;
    let mut map = BTreeMap::new();
    for item in list {
        let key = id(&item).to_string();
        if map.insert(key.clone(), item).is_some() {
            return Err(DataError::Duplicate(key));
        }
    }
    Ok(map)
}

impl Catalog {
    pub fn from_json(
        costumes: &str,
        skills: &str,
        enemies: &str,
        stages: &str,
    ) -> Result<Self, DataError> {
        let cat = Catalog {
            costumes: by_id(costumes, |c: &CostumeDef| &c.id)?,
            skills: by_id(skills, |s: &SkillDef| &s.id)?,
            enemies: by_id(enemies, |e: &EnemyDef| &e.id)?,
            stages: by_id(stages, |s: &StageDef| &s.id)?,
        };
        cat.check()?;
        Ok(cat)
    }

    /// repo 的 `data/` 編進來的那一份
    pub fn builtin() -> Self {
        Self::from_json(
            include_str!("../../../data/costumes.json"),
            include_str!("../../../data/skills.json"),
            include_str!("../../../data/enemies.json"),
            include_str!("../../../data/stages.json"),
        )
        .expect("data/*.json 有錯")
    }

    /// 參照都要存在、技能種類要放對位置
    fn check(&self) -> Result<(), DataError> {
        let skill = |id: &str, kind: Option<SkillKind>| match self.skills.get(id) {
            Some(s) if kind.is_none_or(|k| s.kind == k) => Ok(()),
            _ => Err(DataError::Missing(format!("skill {id}"))),
        };
        for c in self.costumes.values() {
            skill(&c.skills[0], Some(SkillKind::Basic))?;
            skill(&c.skills[1], Some(SkillKind::Skill))?;
            skill(&c.skills[2], Some(SkillKind::Ultimate))?;
            if self.skills[&c.skills[1]].cost == 0 {
                return Err(DataError::Missing(format!("cost of {}", c.skills[1])));
            }
        }
        for e in self.enemies.values() {
            if e.skills.is_empty() {
                return Err(DataError::Missing(format!("skills of {}", e.id)));
            }
            for s in &e.skills {
                skill(s, None)?;
            }
        }
        for s in self.stages.values() {
            if s.waves.is_empty() || s.waves.iter().any(|w| w.is_empty()) {
                return Err(DataError::Missing(format!("waves of {}", s.id)));
            }
            for e in s.waves.iter().flatten() {
                if !self.enemies.contains_key(e) {
                    return Err(DataError::Missing(format!("enemy {e}")));
                }
            }
        }
        Ok(())
    }
}
