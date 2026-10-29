//! Server 的重算：用戰鬥票上的隊伍、關卡、種子，把 Client 送來的指令紀錄重跑一次。
//! 指令不合法、跑完還沒結束、結束後還有多的指令，都算驗證失敗，不發獎勵。

use crate::battle::{Battle, BattleSetup, Command, CommandError, Phase};
use crate::data::Catalog;
use crate::stats::SetupError;
use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Outcome {
    pub won: bool,
    /// 我方出手幾次（三星評價看這個）
    pub rounds: u32,
    /// 還站著的我方人數（之後算星數用）
    pub survivors: usize,
    pub digest: u64,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ReplayError {
    Setup(SetupError),
    Command {
        index: usize,
        error: CommandError,
    },
    /// 指令用完了，戰鬥還沒結束
    Unfinished,
    /// 戰鬥結束後還有指令
    Extra {
        index: usize,
    },
}

pub fn outcome(b: &Battle) -> Outcome {
    Outcome {
        won: b.phase() == Phase::Won,
        rounds: b.ally_turns(),
        survivors: b.allies().iter().filter(|a| a.alive()).count(),
        digest: b.digest(),
    }
}

pub fn simulate(
    cat: &Catalog,
    setup: &BattleSetup,
    seed: u64,
    commands: &[Command],
) -> Result<Outcome, ReplayError> {
    let mut b = Battle::new(cat, setup, seed).map_err(ReplayError::Setup)?;
    for (index, cmd) in commands.iter().enumerate() {
        if b.phase() != Phase::Command {
            return Err(ReplayError::Extra { index });
        }
        b.apply(cmd)
            .map_err(|error| ReplayError::Command { index, error })?;
        b.take_events();
    }
    if b.phase() == Phase::Command {
        return Err(ReplayError::Unfinished);
    }
    Ok(outcome(&b))
}
