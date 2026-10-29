//! Scheduler：決定誰能行動。
//!
//! 第一個（目前唯一的）戰鬥規則是「行動條」：每個單位的行動值每一刻加上自己的速度，
//! 先到 `GAUGE_FULL` 的出手，出手時歸零。同時滿的話，行動值高的先；再一樣就速度快的先、我方先、前排先。
//! 接棒（聲援）指定的隊友插在最前面。之後要加別的規則（時間軸、卡牌…）就另外實作 `Scheduler`。

use crate::data::{StatKey, StatusRule};
use crate::state::{Side, Unit, Who, GAUGE_FULL};

pub trait Scheduler {
    /// 推進到下一個出手的單位並把他的行動值歸零；雙方都沒有活著的單位時回傳 None
    fn next(
        &self,
        allies: &mut [Unit],
        enemies: &mut [Unit],
        relay: &mut Option<usize>,
        r: &StatusRule,
    ) -> Option<Who>;

    /// 預測接下來 n 個出手的單位（不改變狀態，給時間軸用）
    fn preview(
        &self,
        allies: &[Unit],
        enemies: &[Unit],
        relay: Option<usize>,
        r: &StatusRule,
        n: usize,
    ) -> Vec<Who>;
}

pub struct GaugeScheduler;

fn pick(allies: &[Unit], enemies: &[Unit], r: &StatusRule) -> Option<Who> {
    let units = allies
        .iter()
        .enumerate()
        .map(|(i, u)| ((Side::Ally, i), u))
        .chain(
            enemies
                .iter()
                .enumerate()
                .map(|(i, u)| ((Side::Enemy, i), u)),
        );
    units
        .filter(|(_, u)| u.alive() && u.gauge >= GAUGE_FULL)
        .max_by(|(a, ua), (b, ub)| {
            ua.gauge
                .cmp(&ub.gauge)
                .then(ua.eff(StatKey::Spd, r).cmp(&ub.eff(StatKey::Spd, r)))
                // 我方先、前面的先（max_by 取最大，所以反過來比）
                .then(b.0.cmp(&a.0))
                .then(b.1.cmp(&a.1))
        })
        .map(|(w, _)| w)
}

/// 讓時間走到有人滿為止
fn tick(allies: &mut [Unit], enemies: &mut [Unit], r: &StatusRule) -> bool {
    let need = allies
        .iter()
        .chain(enemies.iter())
        .filter(|u| u.alive())
        .map(|u| {
            let spd = u.eff(StatKey::Spd, r).max(1);
            ((GAUGE_FULL - u.gauge).max(0) + spd - 1) / spd
        })
        .min();
    let Some(t) = need else { return false };
    for u in allies.iter_mut().chain(enemies.iter_mut()) {
        if u.alive() {
            u.gauge += u.eff(StatKey::Spd, r).max(1) * t;
        }
    }
    true
}

impl Scheduler for GaugeScheduler {
    fn next(
        &self,
        allies: &mut [Unit],
        enemies: &mut [Unit],
        relay: &mut Option<usize>,
        r: &StatusRule,
    ) -> Option<Who> {
        if let Some(i) = relay.take() {
            if allies.get(i).is_some_and(|u| u.alive()) {
                allies[i].gauge = 0;
                return Some((Side::Ally, i));
            }
        }
        loop {
            if let Some(w) = pick(allies, enemies, r) {
                match w.0 {
                    Side::Ally => allies[w.1].gauge = 0,
                    Side::Enemy => enemies[w.1].gauge = 0,
                }
                return Some(w);
            }
            if !tick(allies, enemies, r) {
                return None;
            }
        }
    }

    fn preview(
        &self,
        allies: &[Unit],
        enemies: &[Unit],
        relay: Option<usize>,
        r: &StatusRule,
        n: usize,
    ) -> Vec<Who> {
        let mut a = allies.to_vec();
        let mut e = enemies.to_vec();
        let mut relay = relay;
        (0..n)
            .map_while(|_| self.next(&mut a, &mut e, &mut relay, r))
            .collect()
    }
}
