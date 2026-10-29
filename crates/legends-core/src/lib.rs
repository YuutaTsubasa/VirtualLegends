//! 虛擬新傳說的戰鬥引擎。Client 演出與 Server 驗證用的是同一份程式。
//! - `rng`：自己寫的亂數（不靠外部套件，任何平台、任何版本結果都一樣）
//! - `tiles`：牌氣（萬／筒／條／字）、牌山、順子與刻子
//! - `data`：衣裝、技能、敵人、關卡的資料（repo 的 `data/*.json`）
//! - `stats`：隊伍成員（等級、突破、羈絆）換算成能力值
//! - `battle`：戰鬥本體（確定性：同樣的隊伍、關卡、種子、指令，一定得到同樣的結果）
//! - `ai`：自動戰鬥
//! - `replay`：Server 用的重算

pub mod ai;
pub mod battle;
pub mod data;
pub mod replay;
pub mod rng;
pub mod stats;
pub mod tiles;

#[cfg(test)]
mod tests;
