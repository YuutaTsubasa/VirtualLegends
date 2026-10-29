//! 虛擬新傳說的戰鬥引擎。Client 演出與 Server 驗證用的是同一份程式。
//!
//! 架構（`docs/plan.md` §4.6）：
//! - Combat Ruleset：目前是「行動條」（`scheduler::GaugeScheduler`）
//! - `scheduler`：Scheduler，決定誰能行動
//! - `resolver`：Action Resolver，解析技能與目標、敵人 AI
//! - `effects`：Effect System，傷害、治療、增減益、推條
//! - `triggers`：Trigger System，反擊、連攜、追擊、被動
//! - `resources`：Resource System，冷卻、狀態回合數（聲援在 `battle`）
//! - `state`：Battle State，角色與戰場狀態
//! - `battle`：把上面串起來；`rng`、`replay` 是 Deterministic Simulation Core
//! - `data`：遊戲資料（repo 的 `data/*.json`）；`stats`：等級、突破、羈絆、裝備換算成能力值
//! - `ai`：我方的自動戰鬥

pub mod ai;
pub mod battle;
pub mod data;
mod effects;
pub mod replay;
mod resolver;
mod resources;
pub mod rng;
pub mod scheduler;
pub mod state;
pub mod stats;
mod triggers;

#[cfg(test)]
mod tests;
