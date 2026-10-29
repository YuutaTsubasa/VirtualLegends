//! 把戰鬥引擎包成 WebAssembly。進出都是 JSON 字串，種子是 16 位的十六進位字串（Server 發的）。
//!
//! Client（戰鬥畫面）：
//!   const b = new BattleSession(JSON.stringify(ticket.setup), ticket.seed);
//!   b.apply(JSON.stringify({ actor: 0, skill: 1, target: 0, relay: 2 })); // 回傳這一步的事件（JSON 陣列）
//!   const cmd = b.auto();      // 自動戰鬥的下一個指令（JSON），打完了是 undefined
//!   JSON.parse(b.state());     // 雙方、輪到誰、時間軸、聲援、波次、環境、勝負
//!   b.log();                   // 指令紀錄，打完送給 Server
//!
//! Server（結算）：
//!   simulate(setupJson, seed, commandsJson) // 成功回傳 Outcome 的 JSON，失敗丟出錯誤字串

use legends_core::ai;
use legends_core::battle::{Battle, BattleSetup, Command};
use legends_core::data::Catalog;
use legends_core::replay;
use serde_json::json;
use std::sync::OnceLock;
use wasm_bindgen::prelude::*;

/// 時間軸顯示幾個
const TIMELINE: usize = 8;

fn catalog() -> &'static Catalog {
    static CAT: OnceLock<Catalog> = OnceLock::new();
    CAT.get_or_init(Catalog::builtin)
}

fn parse_seed(seed: &str) -> Result<u64, JsError> {
    u64::from_str_radix(seed, 16).map_err(|_| JsError::new("bad seed"))
}

fn parse<T: serde::de::DeserializeOwned>(s: &str, what: &str) -> Result<T, JsError> {
    serde_json::from_str(s).map_err(|e| JsError::new(&format!("bad {what}: {e}")))
}

#[wasm_bindgen]
pub struct BattleSession {
    battle: Battle,
}

#[wasm_bindgen]
impl BattleSession {
    #[wasm_bindgen(constructor)]
    pub fn new(setup: &str, seed: &str) -> Result<BattleSession, JsError> {
        let setup: BattleSetup = parse(setup, "setup")?;
        let battle = Battle::new(catalog(), &setup, parse_seed(seed)?)
            .map_err(|e| JsError::new(&format!("{e:?}")))?;
        Ok(BattleSession { battle })
    }

    /// 套用一個指令，回傳這一步產生的事件；指令不合法時丟出錯誤（狀態不變）
    pub fn apply(&mut self, command: &str) -> Result<String, JsError> {
        let cmd: Command = parse(command, "command")?;
        self.battle
            .apply(&cmd)
            .map_err(|e| JsError::new(&format!("{e:?}")))?;
        Ok(serde_json::to_string(&self.battle.take_events()).unwrap())
    }

    /// 這個指令現在合不合法（給畫面把不能按的按鈕變灰）
    pub fn check(&self, command: &str) -> Result<(), JsError> {
        let cmd: Command = parse(command, "command")?;
        self.battle
            .validate(&cmd)
            .map_err(|e| JsError::new(&format!("{e:?}")))
    }

    /// 開場（建立戰鬥時）的事件
    pub fn events(&mut self) -> String {
        serde_json::to_string(&self.battle.take_events()).unwrap()
    }

    /// 自動戰鬥的下一個指令
    pub fn auto(&self) -> Option<String> {
        ai::choose(&self.battle).map(|c| serde_json::to_string(&c).unwrap())
    }

    pub fn state(&self) -> String {
        let b = &self.battle;
        let timeline: Vec<_> = b
            .timeline(TIMELINE)
            .into_iter()
            .map(|(side, unit)| json!({ "side": side, "unit": unit }))
            .collect();
        json!({
            "phase": b.phase(),
            "current": b.current(),
            "turns": b.ally_turns(),
            "wave": b.wave(),
            "waves": b.wave_count(),
            "cheer": b.cheer(),
            "cheer_max": b.rules().cheer_max,
            "relay_cost": b.rules().relay_cost,
            "env": b.env(),
            "allies": b.allies(),
            "enemies": b.enemies(),
            "timeline": timeline,
        })
        .to_string()
    }

    pub fn log(&self) -> String {
        serde_json::to_string(self.battle.log()).unwrap()
    }

    pub fn digest(&self) -> String {
        format!("{:016x}", self.battle.digest())
    }
}

/// Server 的重算：成功回傳 `{ won, rounds, survivors, digest }`（digest 是十六進位字串）
#[wasm_bindgen]
pub fn simulate(setup: &str, seed: &str, commands: &str) -> Result<String, JsError> {
    let setup: BattleSetup = parse(setup, "setup")?;
    let commands: Vec<Command> = parse(commands, "commands")?;
    let o = replay::simulate(catalog(), &setup, parse_seed(seed)?, &commands)
        .map_err(|e| JsError::new(&format!("{e:?}")))?;
    Ok(json!({
        "won": o.won,
        "rounds": o.rounds,
        "survivors": o.survivors,
        "digest": format!("{:016x}", o.digest),
    })
    .to_string())
}

/// 隊伍的檢查（Server 發戰鬥票前用）：合法回傳 undefined，不合法丟出錯誤
#[wasm_bindgen(js_name = checkSetup)]
pub fn check_setup(setup: &str) -> Result<(), JsError> {
    let setup: BattleSetup = parse(setup, "setup")?;
    Battle::new(catalog(), &setup, 0)
        .map(|_| ())
        .map_err(|e| JsError::new(&format!("{e:?}")))
}

/// 一個隊伍成員（等級、突破、羈絆、技能等級、裝備）的能力值，給角色畫面顯示
#[wasm_bindgen(js_name = memberStats)]
pub fn member_stats(member: &str) -> Result<String, JsError> {
    let m: legends_core::stats::Member = parse(member, "member")?;
    let s = legends_core::stats::member_stats(catalog(), &m)
        .ok_or_else(|| JsError::new("unknown costume"))?;
    Ok(serde_json::to_string(&s).unwrap())
}

/// 等級上限（看突破），Server 的養成用同一份規則
#[wasm_bindgen(js_name = levelCap)]
pub fn level_cap(ascension: u8) -> u32 {
    legends_core::stats::level_cap(ascension)
}
