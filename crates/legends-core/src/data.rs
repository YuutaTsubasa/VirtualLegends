//! 遊戲資料：角色、技能、敵人、關卡、裝備套裝、規則數字。正本是 repo 的 `data/*.json`，Client 與 Server 編進同一份。
//! 技能的目標是「相對施放者」的：Enemy 是對手那一邊，Ally 是自己那一邊。

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// 屬性：焰 → 嵐 → 潮 → 焰 循環相剋；曉、暮互剋
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Element {
    Blaze,
    Tide,
    Gale,
    Dawn,
    Dusk,
}

impl Element {
    /// 自己剋不剋 o
    pub fn beats(self, o: Element) -> bool {
        use Element::*;
        matches!(
            (self, o),
            (Blaze, Gale) | (Gale, Tide) | (Tide, Blaze) | (Dawn, Dusk) | (Dusk, Dawn)
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// 攻擊：收尾與追擊
    Attacker,
    /// 守護：比較常被打，帶反擊與護盾
    Guardian,
    /// 支援：推條、治療、淨化
    Support,
    /// 妨礙：延遲、減益
    Disruptor,
}

/// 能力值的種類。暴擊、暴擊傷害、效果命中、效果抵抗是百分點
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StatKey {
    Hp,
    Atk,
    Def,
    Spd,
    Crit,
    CritDmg,
    Hit,
    Resist,
}

impl StatKey {
    /// 這幾種的「%」直接當百分點加（暴擊 +10% 就是 +10）
    pub fn is_rate(self) -> bool {
        matches!(
            self,
            StatKey::Crit | StatKey::CritDmg | StatKey::Hit | StatKey::Resist
        )
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stats {
    #[serde(default)]
    pub hp: i64,
    #[serde(default)]
    pub atk: i64,
    #[serde(default)]
    pub def: i64,
    #[serde(default)]
    pub spd: i64,
    #[serde(default)]
    pub crit: i64,
    #[serde(default)]
    pub crit_dmg: i64,
    #[serde(default)]
    pub hit: i64,
    #[serde(default)]
    pub resist: i64,
}

impl Stats {
    pub fn get(&self, k: StatKey) -> i64 {
        match k {
            StatKey::Hp => self.hp,
            StatKey::Atk => self.atk,
            StatKey::Def => self.def,
            StatKey::Spd => self.spd,
            StatKey::Crit => self.crit,
            StatKey::CritDmg => self.crit_dmg,
            StatKey::Hit => self.hit,
            StatKey::Resist => self.resist,
        }
    }

    pub fn get_mut(&mut self, k: StatKey) -> &mut i64 {
        match k {
            StatKey::Hp => &mut self.hp,
            StatKey::Atk => &mut self.atk,
            StatKey::Def => &mut self.def,
            StatKey::Spd => &mut self.spd,
            StatKey::Crit => &mut self.crit,
            StatKey::CritDmg => &mut self.crit_dmg,
            StatKey::Hit => &mut self.hit,
            StatKey::Resist => &mut self.resist,
        }
    }

    pub const KEYS: [StatKey; 8] = [
        StatKey::Hp,
        StatKey::Atk,
        StatKey::Def,
        StatKey::Spd,
        StatKey::Crit,
        StatKey::CritDmg,
        StatKey::Hit,
        StatKey::Resist,
    ];
}

/// 能力值的加成：add 是固定值，pct 是百分比（暴擊這類 is_rate 的直接加百分點）
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatMod {
    pub stat: StatKey,
    #[serde(default)]
    pub add: i64,
    #[serde(default)]
    pub pct: i64,
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

impl Target {
    /// 要不要指定一個目標
    pub fn single(self) -> bool {
        matches!(self, Target::Enemy | Target::Ally)
    }

    pub fn hostile(self) -> bool {
        matches!(self, Target::Enemy | Target::AllEnemies)
    }
}

/// 效果作用在誰身上：預設是技能的目標
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum To {
    #[default]
    Target,
    #[serde(rename = "self")]
    Myself,
    AllAllies,
    AllEnemies,
}

/// 傷害、治療、護盾用施放者的哪個能力值算
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Scale {
    #[default]
    Atk,
    Def,
    MaxHp,
}

/// 增益與減益。數值在 `rules.json` 的 `status`
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StatusKind {
    AtkUp,
    DefUp,
    SpdUp,
    CritUp,
    /// 不會被上減益
    Immune,
    /// 反擊架勢：被打必定反擊
    Counter,
    AtkDown,
    DefDown,
    SpdDown,
    HitDown,
    /// 灼燒：自己回合開始時受傷（施放者攻擊力的 rules.status.burn%）
    Burn,
    /// 暈眩：跳過回合；蓄力中被暈會中斷
    Stun,
    /// 挑釁：單體技能只能打施放者
    Taunt,
    /// 封印：只能用第一招
    Silence,
    /// 阻止回復
    NoHeal,
}

impl StatusKind {
    pub fn is_buff(self) -> bool {
        use StatusKind::*;
        matches!(self, AtkUp | DefUp | SpdUp | CritUp | Immune | Counter)
    }
}

fn hundred() -> i64 {
    100
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Effect {
    /// 傷害：scale 能力值 × power%
    Damage {
        power: i64,
        #[serde(default)]
        scale: Scale,
        #[serde(default)]
        to: To,
    },
    Heal {
        power: i64,
        #[serde(default)]
        scale: Scale,
        #[serde(default)]
        to: To,
    },
    /// 護盾：持續兩回合，最多到最大 HP
    Shield {
        power: i64,
        #[serde(default)]
        scale: Scale,
        #[serde(default)]
        to: To,
    },
    /// 上增益（給自己人）或減益（給對手，要過命中與抵抗）
    Status {
        status: StatusKind,
        turns: u8,
        #[serde(default = "hundred")]
        chance: i64,
        #[serde(default)]
        to: To,
    },
    /// 推條：正的是加速（自己人），負的是延遲（對手，要過命中與抵抗）
    Gauge {
        pct: i64,
        #[serde(default)]
        to: To,
    },
    /// 拿掉 count 個減益
    Cleanse {
        count: u8,
        #[serde(default)]
        to: To,
    },
    /// 拿掉對手 count 個增益
    Dispel {
        count: u8,
        #[serde(default)]
        to: To,
    },
    /// 冷卻減 amount 回合
    Cooldown {
        amount: u8,
        #[serde(default)]
        to: To,
    },
}

impl Effect {
    pub fn to(&self) -> To {
        match *self {
            Effect::Damage { to, .. }
            | Effect::Heal { to, .. }
            | Effect::Shield { to, .. }
            | Effect::Status { to, .. }
            | Effect::Gauge { to, .. }
            | Effect::Cleanse { to, .. }
            | Effect::Dispel { to, .. }
            | Effect::Cooldown { to, .. } => to,
        }
    }
}

/// 技能強化：第 n 級（從 2 開始）換掉冷卻或效果，沒寫的沿用前一級
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Upgrade {
    /// 畫面上的說明（例：「冷卻 4 → 3」）
    #[serde(default)]
    pub note: String,
    #[serde(default)]
    pub cooldown: Option<u8>,
    #[serde(default)]
    pub effects: Option<Vec<Effect>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillDef {
    pub id: String,
    pub name: String,
    /// 用完要等幾個自己的回合（0＝沒有冷卻）
    #[serde(default)]
    pub cooldown: u8,
    pub target: Target,
    pub effects: Vec<Effect>,
    /// 只有敵人用：蓄力幾個自己的回合才發動（掛在時間軸上，暈眩會中斷）
    #[serde(default)]
    pub charge: u8,
    #[serde(default)]
    pub upgrades: Vec<Upgrade>,
}

impl SkillDef {
    pub fn max_level(&self) -> u8 {
        1 + self.upgrades.len() as u8
    }

    /// 套用強化到 level 級之後的技能
    pub fn at_level(&self, level: u8) -> SkillDef {
        let mut s = self.clone();
        for u in self.upgrades.iter().take(level.saturating_sub(1) as usize) {
            if let Some(cd) = u.cooldown {
                s.cooldown = cd;
            }
            if let Some(e) = &u.effects {
                s.effects = e.clone();
            }
        }
        s.upgrades.clear();
        s
    }

    pub fn deals_damage(&self) -> bool {
        self.effects
            .iter()
            .any(|e| matches!(e, Effect::Damage { .. }))
    }
}

/// 被動在什麼時候觸發
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum On {
    /// 被打（還活著）：反擊
    Hit,
    /// 隊友用主動技能打人：連攜
    AllyAttack,
    /// 自己用主動技能打人之後：追擊
    AfterAttack,
    /// 自己的回合開始
    TurnStart,
    /// 戰鬥開始（每一波）
    BattleStart,
    /// 打倒敵人
    Kill,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Cond {
    /// 目標剩不到 n% HP
    TargetHpBelow(i64),
    /// 目標身上有這個狀態
    TargetHas(StatusKind),
    /// 自己剩不到 n% HP
    SelfHpBelow(i64),
    /// 這一擊暴擊
    Crit,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PassiveAct {
    /// 用第一招打攻擊者
    Counter,
    /// 用第一招打同一個目標
    Link,
    /// 對目標追加一擊
    Pursuit { power: i64 },
    /// 套用效果：Hit 的目標是攻擊者；AllyAttack／AfterAttack 是被打的人；其他是自己
    Effects { effects: Vec<Effect> },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Passive {
    pub on: On,
    #[serde(default = "hundred")]
    pub chance: i64,
    #[serde(default)]
    pub cond: Option<Cond>,
    pub act: PassiveAct,
    /// 畫面上的名字與說明
    #[serde(default)]
    pub name: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CostumeDef {
    pub id: String,
    /// 系列共通的角色 id（xiaocui、yuehua…）
    pub character: String,
    pub name: String,
    pub rarity: u8,
    pub element: Element,
    pub role: Role,
    pub base: Stats,
    /// 每升一級加多少
    pub growth: Stats,
    /// 三招：第一招沒有冷卻
    pub skills: [String; 3],
    #[serde(default)]
    pub passives: Vec<Passive>,
}

/// 魔王的階段：HP 低於 hp_below% 時換技能、套用效果
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhaseDef {
    pub hp_below: i64,
    #[serde(default)]
    pub skills: Option<Vec<String>>,
    #[serde(default)]
    pub effects: Vec<Effect>,
    #[serde(default)]
    pub name: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EnemyDef {
    pub id: String,
    pub name: String,
    pub element: Element,
    pub stats: Stats,
    /// 第一招沒有冷卻；AI 會用冷卻好了的最後一招
    pub skills: Vec<String>,
    #[serde(default)]
    pub passives: Vec<Passive>,
    /// 護盾格數：還有格數時受到的傷害減半，每被打一下少一格，打破時暈眩一回合
    #[serde(default)]
    pub toughness: u32,
    #[serde(default)]
    pub phases: Vec<PhaseDef>,
    #[serde(default)]
    pub boss: bool,
}

/// 環境條件的一條：side 不寫是雙方，element 不寫是所有屬性
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnvMod {
    #[serde(default)]
    pub side: Option<crate::state::Side>,
    #[serde(default)]
    pub element: Option<Element>,
    #[serde(flatten)]
    pub stat: StatMod,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnvDef {
    pub name: String,
    #[serde(default)]
    pub text: String,
    pub mods: Vec<EnvMod>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StageDef {
    pub id: String,
    pub name: String,
    /// 敵人能力值的倍率（百分比）
    #[serde(default = "hundred")]
    pub scale: i64,
    pub waves: Vec<Vec<String>>,
    #[serde(default)]
    pub env: Option<EnvDef>,
    /// 三星評價：幾個我方回合內打完
    #[serde(default)]
    pub star_turns: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Slot {
    Weapon,
    Armor,
    Accessory,
    Badge,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GearSetDef {
    pub id: String,
    pub name: String,
    /// 幾件觸發（2 或 4）
    pub pieces: u8,
    #[serde(default)]
    pub stats: Vec<StatMod>,
    #[serde(default)]
    pub passive: Option<Passive>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatusRule {
    pub atk_up: i64,
    pub def_up: i64,
    pub spd_up: i64,
    pub crit_up: i64,
    pub atk_down: i64,
    pub def_down: i64,
    pub spd_down: i64,
    pub hit_down: i64,
    pub burn: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Resonance {
    pub element: Element,
    /// 同屬性至少幾人
    pub count: usize,
    #[serde(flatten)]
    pub stat: StatMod,
}

/// 規則數字（`data/rules.json`）
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Rules {
    /// 所有單位都有的基礎值（暴擊、暴擊傷害…）
    pub base: Stats,
    /// 剋的一方傷害 ×adv%、效果命中 +adv_hit
    pub adv: i64,
    pub adv_hit: i64,
    /// 被剋的一方：graze% 機率擦傷，傷害 ×graze_dmg%、不附帶減益
    pub disadv: i64,
    pub graze: i64,
    pub graze_dmg: i64,
    /// 減益命中率的下限與上限（%）
    pub hit_floor: i64,
    pub hit_cap: i64,
    pub status: StatusRule,
    /// 聲援上限、每次出手加多少、接棒花多少
    pub cheer_max: u8,
    pub relay_cost: u8,
    /// 羈絆組合同隊時的連攜機率
    pub bond_link: i64,
    pub bonds: Vec<[String; 2]>,
    pub resonance: Vec<Resonance>,
    /// 守護型被敵人單體技能選中的機率（%）
    pub guardian_aggro: i64,
    /// 有護盾格數時受到的傷害（%）
    pub toughness_dmg: i64,
}

#[derive(Clone, Debug)]
pub struct Catalog {
    pub costumes: BTreeMap<String, CostumeDef>,
    pub skills: BTreeMap<String, SkillDef>,
    pub enemies: BTreeMap<String, EnemyDef>,
    pub stages: BTreeMap<String, StageDef>,
    pub sets: BTreeMap<String, GearSetDef>,
    pub rules: Rules,
}

#[derive(Debug, PartialEq, Eq)]
pub enum DataError {
    Parse(String),
    Duplicate(String),
    Missing(String),
    Invalid(String),
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

pub struct Sources<'a> {
    pub costumes: &'a str,
    pub skills: &'a str,
    pub enemies: &'a str,
    pub stages: &'a str,
    pub sets: &'a str,
    pub rules: &'a str,
}

impl Catalog {
    pub fn from_json(src: Sources) -> Result<Self, DataError> {
        let cat = Catalog {
            costumes: by_id(src.costumes, |c: &CostumeDef| &c.id)?,
            skills: by_id(src.skills, |s: &SkillDef| &s.id)?,
            enemies: by_id(src.enemies, |e: &EnemyDef| &e.id)?,
            stages: by_id(src.stages, |s: &StageDef| &s.id)?,
            sets: by_id(src.sets, |s: &GearSetDef| &s.id)?,
            rules: serde_json::from_str(src.rules).map_err(|e| DataError::Parse(e.to_string()))?,
        };
        cat.check()?;
        Ok(cat)
    }

    /// repo 的 `data/` 編進來的那一份
    pub fn builtin() -> Self {
        Self::from_json(Sources {
            costumes: include_str!("../../../data/costumes.json"),
            skills: include_str!("../../../data/skills.json"),
            enemies: include_str!("../../../data/enemies.json"),
            stages: include_str!("../../../data/stages.json"),
            sets: include_str!("../../../data/gear_sets.json"),
            rules: include_str!("../../../data/rules.json"),
        })
        .expect("data/*.json 有錯")
    }

    fn check(&self) -> Result<(), DataError> {
        let skill = |id: &str| {
            self.skills
                .get(id)
                .ok_or_else(|| DataError::Missing(format!("skill {id}")))
        };
        for c in self.costumes.values() {
            for (i, id) in c.skills.iter().enumerate() {
                let s = skill(id)?;
                if i == 0 && s.cooldown != 0 {
                    return Err(DataError::Invalid(format!("{id}: 第一招不能有冷卻")));
                }
            }
        }
        for e in self.enemies.values() {
            if e.skills.is_empty() {
                return Err(DataError::Missing(format!("skills of {}", e.id)));
            }
            let lists =
                std::iter::once(&e.skills).chain(e.phases.iter().filter_map(|p| p.skills.as_ref()));
            for list in lists {
                for id in list {
                    skill(id)?;
                }
                if skill(&list[0])?.cooldown != 0 || skill(&list[0])?.charge != 0 {
                    return Err(DataError::Invalid(format!(
                        "{}: 第一招不能有冷卻或蓄力",
                        e.id
                    )));
                }
            }
        }
        for s in self.stages.values() {
            if s.waves.is_empty() || s.waves.iter().any(|w| w.is_empty() || w.len() > 5) {
                return Err(DataError::Invalid(format!("waves of {}", s.id)));
            }
            for e in s.waves.iter().flatten() {
                if !self.enemies.contains_key(e) {
                    return Err(DataError::Missing(format!("enemy {e}")));
                }
            }
        }
        for [a, b] in &self.rules.bonds {
            if a == b {
                return Err(DataError::Invalid(format!("bond {a}")));
            }
        }
        Ok(())
    }

    /// 兩位系列角色是不是羈絆組合
    pub fn bonded(&self, a: &str, b: &str) -> bool {
        self.rules
            .bonds
            .iter()
            .any(|[x, y]| (x == a && y == b) || (x == b && y == a))
    }
}
