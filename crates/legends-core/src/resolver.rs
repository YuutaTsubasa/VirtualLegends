//! Action Resolver：把「誰用哪一招打誰」解析成目標與效果，以及敵人怎麼選技能、選目標、蓄力。
//! 我方指令的合法性檢查在 `battle::validate`。

use crate::battle::{Battle, Event, Via};
use crate::data::{Role, Target};
use crate::effects::Hit;
use crate::state::{Charge, Side, Who};

impl Battle {
    /// 技能的主要目標
    fn primary(&self, who: Who, target: Target, picked: Option<usize>) -> Vec<Who> {
        let me = who.0;
        match target {
            Target::Enemy => picked.map(|t| (me.other(), t)).into_iter().collect(),
            Target::Ally => picked.map(|t| (me, t)).into_iter().collect(),
            Target::AllEnemies => self.alive_of(me.other()),
            Target::AllAllies => self.alive_of(me),
            Target::Myself => vec![who],
        }
    }

    /// 用第 slot 招。via 是觸發來源（反擊、連攜）；主動使用才進冷卻、才會引發連攜與追擊
    pub(crate) fn use_skill(
        &mut self,
        who: Who,
        slot: usize,
        picked: Option<usize>,
        via: Option<Via>,
    ) {
        let skill = self.unit(who).skills[slot].clone();
        if via.is_none() {
            self.unit_mut(who).cooldowns[slot] = skill.cooldown;
        }
        let primary = self.primary(who, skill.target, picked);
        let target_side = if skill.target.hostile() {
            who.0.other()
        } else {
            who.0
        };
        self.events.push(Event::Skill {
            side: who.0,
            unit: who.1,
            skill: skill.id.clone(),
            slot,
            target_side,
            targets: primary.iter().map(|w| w.1).collect(),
            via,
        });
        let mut hits: Vec<Hit> = Vec::new();
        self.apply_effects(who, &skill.effects, &primary, &mut hits);
        let main = if skill.target.single() {
            primary.first().copied()
        } else {
            None
        };
        self.after_skill(who, &hits, main, via);
    }

    /// 敵人單體技能的目標：被挑釁就打挑釁的人；否則守護型比較常被選中
    fn enemy_pick(&mut self, who: Who, target: Target) -> Option<usize> {
        let foes = self.alive_of(who.0.other());
        match target {
            Target::Enemy => {
                if let Some(src) = self.unit(who).taunted_by() {
                    if src.0 != who.0 && self.unit(src).alive() {
                        return Some(src.1);
                    }
                }
                let guards: Vec<Who> = foes
                    .iter()
                    .copied()
                    .filter(|&w| self.unit(w).role == Some(Role::Guardian))
                    .collect();
                if !guards.is_empty() && (self.rng.below(100) as i64) < self.rules.guardian_aggro {
                    let i = self.rng.below(guards.len() as u64) as usize;
                    return Some(guards[i].1);
                }
                if foes.is_empty() {
                    return None;
                }
                let i = self.rng.below(foes.len() as u64) as usize;
                Some(foes[i].1)
            }
            Target::Ally => self
                .alive_of(who.0)
                .into_iter()
                .min_by_key(|&w| (self.unit(w).hp_pct(), w.1))
                .map(|w| w.1),
            _ => None,
        }
    }

    pub(crate) fn enemy_turn(&mut self, i: usize) {
        let who = (Side::Enemy, i);
        // 蓄力中：時間到了才發動
        if let Some(c) = self.unit(who).charge {
            if c.turns > 0 {
                self.unit_mut(who).charge = Some(Charge {
                    turns: c.turns - 1,
                    ..c
                });
                self.events.push(Event::Charging {
                    unit: i,
                    turns: c.turns - 1,
                });
                return;
            }
            self.unit_mut(who).charge = None;
            let target = self.unit(who).skills[c.slot].target;
            let t = self.enemy_pick(who, target);
            self.use_skill(who, c.slot, t, None);
            return;
        }
        let u = self.unit(who);
        let slot = (0..u.skills.len())
            .rev()
            .find(|&s| u.can_use(s))
            .unwrap_or(0);
        let skill = &u.skills[slot];
        if skill.charge > 0 {
            let (id, turns, cd) = (skill.id.clone(), skill.charge, skill.cooldown);
            let u = self.unit_mut(who);
            u.cooldowns[slot] = cd;
            u.charge = Some(Charge {
                slot,
                turns: turns - 1,
            });
            self.events.push(Event::Charge {
                unit: i,
                skill: id,
                turns,
            });
            return;
        }
        let target = skill.target;
        let t = self.enemy_pick(who, target);
        if target.single() && t.is_none() {
            return;
        }
        self.use_skill(who, slot, t, None);
    }
}
