//! Resource System：冷卻、增減益的回合數、護盾的回合數、回合開始的灼燒。聲援（接棒）的增減在 `battle::apply`。
//!
//! 「回合」都是單位自己的回合：冷卻在自己回合開始時減 1，增減益與護盾在自己回合結束時減 1。
//! 在自己回合上身的狀態，這個回合結束時不扣（所以「2 回合」的增益會撐過接下來兩個自己的回合）。

use crate::battle::{Battle, Event};
use crate::data::{On, StatusKind};
use crate::state::Who;

impl Battle {
    pub(crate) fn begin_turn(&mut self, w: Who) {
        self.acting = Some(w);
        let u = self.unit_mut(w);
        for c in &mut u.cooldowns {
            *c = c.saturating_sub(1);
        }
        let burn: i64 = u
            .statuses
            .iter()
            .filter(|s| s.kind == StatusKind::Burn)
            .map(|s| s.value)
            .sum();
        if burn > 0 {
            self.direct_damage(w, burn);
        }
        if self.unit(w).alive() {
            self.passives_on_self(w, On::TurnStart);
        }
    }

    pub(crate) fn end_turn(&mut self, w: Who) {
        let u = self.unit_mut(w);
        let mut expired = Vec::new();
        for s in u.statuses.iter_mut() {
            if s.fresh {
                s.fresh = false;
            } else {
                s.turns = s.turns.saturating_sub(1);
            }
            if s.turns == 0 {
                expired.push(s.kind);
            }
        }
        u.statuses.retain(|s| s.turns > 0);
        if u.shield > 0 {
            u.shield_turns = u.shield_turns.saturating_sub(1);
            if u.shield_turns == 0 {
                u.shield = 0;
            }
        }
        for status in expired {
            self.events.push(Event::Expired {
                side: w.0,
                unit: w.1,
                status,
            });
        }
        self.acting = None;
    }

    pub(crate) fn battle_start_passives(&mut self) {
        let all: Vec<Who> = self
            .alive_of(crate::state::Side::Ally)
            .into_iter()
            .chain(self.alive_of(crate::state::Side::Enemy))
            .collect();
        for w in all {
            self.passives_on_self(w, On::BattleStart);
        }
    }
}
