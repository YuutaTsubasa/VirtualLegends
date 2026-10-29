//! Trigger System：反擊、連攜、追擊與被動。
//!
//! 主動技能打完之後依序：被打的人反擊 → 自己追擊 → 隊友連攜 → 打倒敵人的被動。
//! 防止無限連鎖的規則：
//! - 觸發出來的行動（反擊、連攜、追擊）不會再引發連攜與追擊；
//! - 反擊不會被反擊；
//! - 每個被打的人一次技能最多反擊一次，每位隊友一次技能最多連攜一次。

use crate::battle::{Battle, Event, Via};
use crate::data::{Cond, On, Passive, PassiveAct, Scale};
use crate::effects::Hit;
use crate::state::{Side, Who};

impl Battle {
    fn cond_ok(&self, cond: Option<Cond>, me: Who, target: Option<Who>, crit: bool) -> bool {
        match cond {
            None => true,
            Some(Cond::Crit) => crit,
            Some(Cond::SelfHpBelow(p)) => self.unit(me).hp_pct() < p,
            Some(Cond::TargetHpBelow(p)) => {
                target.is_some_and(|t| self.unit(t).alive() && self.unit(t).hp_pct() < p)
            }
            Some(Cond::TargetHas(k)) => target.is_some_and(|t| self.unit(t).has(k)),
        }
    }

    /// 這個單位在 on 時機、條件成立而且擲中機率的被動
    fn fire(&mut self, me: Who, on: On, target: Option<Who>, crit: bool) -> Vec<Passive> {
        let ps: Vec<Passive> = self
            .unit(me)
            .passives
            .iter()
            .filter(|p| p.on == on)
            .cloned()
            .collect();
        ps.into_iter()
            .filter(|p| {
                self.cond_ok(p.cond, me, target, crit) && (self.rng.below(100) as i64) < p.chance
            })
            .collect()
    }

    /// 能不能做觸發的行動（活著、沒被暈）
    fn ready(&self, w: Who) -> bool {
        let u = self.unit(w);
        u.alive() && !u.has(crate::data::StatusKind::Stun)
    }

    fn passive_effects(&mut self, me: Who, p: &Passive, target: Who) {
        if let PassiveAct::Effects { effects } = &p.act {
            self.events.push(Event::Trigger {
                side: me.0,
                unit: me.1,
                via: Via::Passive,
                name: p.name.clone(),
            });
            let mut hits = Vec::new();
            self.apply_effects(me, effects, &[target], &mut hits);
        }
    }

    /// 回合開始、戰鬥開始這類只對自己的被動
    pub(crate) fn passives_on_self(&mut self, me: Who, on: On) {
        for p in self.fire(me, on, Some(me), false) {
            self.passive_effects(me, &p, me);
        }
    }

    pub(crate) fn after_skill(
        &mut self,
        who: Who,
        hits: &[Hit],
        main: Option<Who>,
        via: Option<Via>,
    ) {
        if hits.is_empty() {
            return;
        }
        let crit = hits.iter().any(|h| h.crit);

        // 反擊
        if via != Some(Via::Counter) {
            let mut seen: Vec<Who> = Vec::new();
            for h in hits {
                let t = h.target;
                if t.0 == who.0 || seen.contains(&t) || !self.ready(t) {
                    continue;
                }
                seen.push(t);
                let mut counter = self.unit(t).has(crate::data::StatusKind::Counter);
                for p in self.fire(t, On::Hit, Some(who), crit) {
                    match p.act {
                        PassiveAct::Counter => counter = true,
                        _ => self.passive_effects(t, &p, who),
                    }
                }
                if counter && self.unit(who).alive() && self.ready(t) {
                    self.events.push(Event::Trigger {
                        side: t.0,
                        unit: t.1,
                        via: Via::Counter,
                        name: String::new(),
                    });
                    self.use_skill(t, 0, Some(who.1), Some(Via::Counter));
                }
            }
        }
        if via.is_some() {
            return;
        }

        // 追擊：對主要目標（全體技能就是第一個被打的人）
        let focus = main
            .or(hits.first().map(|h| h.target))
            .filter(|&t| t.0 != who.0);
        if let Some(t) = focus {
            for p in self.fire(who, On::AfterAttack, Some(t), crit) {
                match p.act {
                    PassiveAct::Pursuit { power } if self.unit(t).alive() && self.ready(who) => {
                        self.events.push(Event::Trigger {
                            side: who.0,
                            unit: who.1,
                            via: Via::Pursuit,
                            name: p.name.clone(),
                        });
                        if let Some((hit, _)) = self.hit(who, t, power, Scale::Atk) {
                            if hit.killed {
                                self.on_kill(who);
                            }
                        }
                    }
                    PassiveAct::Effects { .. } => self.passive_effects(who, &p, t),
                    _ => {}
                }
            }
        }

        // 連攜：被動寫的機率＋羈絆組合
        if let Some(t0) = focus {
            let mates: Vec<Who> = self
                .alive_of(who.0)
                .into_iter()
                .filter(|&w| w != who)
                .collect();
            for m in mates {
                let mut chance = 0;
                let mut effects = Vec::new();
                let ps: Vec<Passive> = self
                    .unit(m)
                    .passives
                    .iter()
                    .filter(|p| p.on == On::AllyAttack)
                    .cloned()
                    .collect();
                for p in ps {
                    if !self.cond_ok(p.cond, m, Some(t0), crit) {
                        continue;
                    }
                    match p.act {
                        PassiveAct::Link => chance += p.chance,
                        _ => effects.push(p),
                    }
                }
                if who.0 == Side::Ally
                    && self
                        .bonds
                        .iter()
                        .any(|&(a, b)| (a, b) == (who.1.min(m.1), who.1.max(m.1)))
                {
                    chance += self.rules.bond_link;
                }
                for p in effects {
                    if (self.rng.below(100) as i64) < p.chance {
                        self.passive_effects(m, &p, t0);
                    }
                }
                if chance <= 0 || !self.ready(m) || (self.rng.below(100) as i64) >= chance {
                    continue;
                }
                // 原本的目標倒了就打同一邊第一個還活著的
                let Some(t) = std::iter::once(t0)
                    .chain(self.alive_of(t0.0))
                    .find(|&w| self.unit(w).alive())
                else {
                    break;
                };
                self.events.push(Event::Trigger {
                    side: m.0,
                    unit: m.1,
                    via: Via::Link,
                    name: String::new(),
                });
                self.use_skill(m, 0, Some(t.1), Some(Via::Link));
            }
        }

        if hits.iter().any(|h| h.killed) {
            self.on_kill(who);
        }
    }

    fn on_kill(&mut self, who: Who) {
        if self.unit(who).alive() {
            self.passives_on_self(who, On::Kill);
        }
    }
}
