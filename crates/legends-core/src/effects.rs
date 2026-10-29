//! Effect System：傷害、治療、護盾、增減益、推條、淨化、驅散、冷卻，以及魔王換階段。
//!
//! 傷害 = 能力值 × 威力% × 200／(200 + 防禦) × 屬性 × 暴擊 × 浮動 95～105% × 護盾格數
//! 減益（含延遲）要過兩關：技能寫的機率，以及 100 + 效果命中 − 效果抵抗（剋的一方再加），夾在上下限之間。

use crate::battle::{Battle, Event};
use crate::data::{Effect, Scale, StatKey, StatusKind, To};
use crate::state::{Side, Status, Who, GAUGE_FULL};

/// 一次命中的結果（觸發系統要看）
#[derive(Clone, Copy, Debug)]
pub(crate) struct Hit {
    pub target: Who,
    pub crit: bool,
    pub killed: bool,
}

impl Battle {
    fn roll(&mut self, pct: i64) -> bool {
        (self.rng.below(100) as i64) < pct
    }

    fn scaled(&self, from: Who, scale: Scale) -> i64 {
        let u = self.unit(from);
        match scale {
            Scale::Atk => u.eff(StatKey::Atk, &self.rules.status),
            Scale::Def => u.eff(StatKey::Def, &self.rules.status),
            Scale::MaxHp => u.max_hp,
        }
    }

    /// 效果的對象
    pub(crate) fn recipients(&self, from: Who, to: To, primary: &[Who]) -> Vec<Who> {
        let alive = |v: Vec<Who>| v.into_iter().filter(|&w| self.unit(w).alive()).collect();
        match to {
            To::Target => alive(primary.to_vec()),
            To::Myself => alive(vec![from]),
            To::AllAllies => self.alive_of(from.0),
            To::AllEnemies => self.alive_of(from.0.other()),
        }
    }

    /// 依序套用效果。擦傷的目標不會再被上減益
    pub(crate) fn apply_effects(
        &mut self,
        from: Who,
        effects: &[Effect],
        primary: &[Who],
        hits: &mut Vec<Hit>,
    ) {
        let mut grazed: Vec<Who> = Vec::new();
        for e in effects {
            for to in self.recipients(from, e.to(), primary) {
                match *e {
                    Effect::Damage { power, scale, .. } => {
                        if let Some((hit, graze)) = self.hit(from, to, power, scale) {
                            if graze {
                                grazed.push(to);
                            }
                            hits.push(hit);
                        }
                    }
                    Effect::Heal { power, scale, .. } => {
                        let amount = self.scaled(from, scale) * power / 100;
                        self.heal(to, amount);
                    }
                    Effect::Shield { power, scale, .. } => {
                        let amount = self.scaled(from, scale) * power / 100;
                        let fresh = self.acting == Some(to);
                        let u = self.unit_mut(to);
                        u.shield = u.shield.max(amount).min(u.max_hp);
                        u.shield_turns = if fresh { 3 } else { 2 };
                        self.events.push(Event::Shield {
                            side: to.0,
                            unit: to.1,
                            amount,
                        });
                    }
                    Effect::Status {
                        status,
                        turns,
                        chance,
                        ..
                    } => {
                        if !grazed.contains(&to) {
                            self.add_status(from, to, status, turns, chance);
                        }
                    }
                    Effect::Gauge { pct, .. } => {
                        if pct < 0
                            && to.0 != from.0
                            && (grazed.contains(&to) || !self.debuff_lands(from, to, 100))
                        {
                            continue;
                        }
                        let u = self.unit_mut(to);
                        u.gauge = (u.gauge + pct * GAUGE_FULL / 100).clamp(0, GAUGE_FULL);
                        let gauge = u.gauge;
                        self.events.push(Event::Gauge {
                            side: to.0,
                            unit: to.1,
                            pct,
                            gauge,
                        });
                    }
                    Effect::Cleanse { count, .. } => self.remove_statuses(to, count, false),
                    Effect::Dispel { count, .. } => self.remove_statuses(to, count, true),
                    Effect::Cooldown { amount, .. } => {
                        for c in &mut self.unit_mut(to).cooldowns {
                            *c = c.saturating_sub(amount);
                        }
                        self.events.push(Event::Cooldown {
                            side: to.0,
                            unit: to.1,
                            amount,
                        });
                    }
                }
            }
        }
    }

    /// 打一下。目標已經倒下就不算（回傳 None）；回傳（結果, 是否擦傷）
    pub(crate) fn hit(
        &mut self,
        from: Who,
        to: Who,
        power: i64,
        scale: Scale,
    ) -> Option<(Hit, bool)> {
        if !self.unit(to).alive() {
            return None;
        }
        let st = self.rules.status;
        let (r_adv, r_graze, r_graze_dmg, r_disadv, r_tough) = (
            self.rules.adv,
            self.rules.graze,
            self.rules.graze_dmg,
            self.rules.disadv,
            self.rules.toughness_dmg,
        );
        let raw = self.scaled(from, scale) * power / 100;
        let (ae, de) = (self.unit(from).element, self.unit(to).element);
        let def = self.unit(to).eff(StatKey::Def, &st);
        let mut dmg = raw * 200 / (200 + def);
        let adv = ae.beats(de);
        let mut graze = false;
        if adv {
            dmg = dmg * r_adv / 100;
        } else if de.beats(ae) {
            if self.roll(r_graze) {
                graze = true;
                dmg = dmg * r_graze_dmg / 100;
            } else {
                dmg = dmg * r_disadv / 100;
            }
        }
        let att = self.unit(from);
        let (crit_rate, crit_dmg) = (att.eff(StatKey::Crit, &st), att.eff(StatKey::CritDmg, &st));
        let crit = !graze && self.roll(crit_rate);
        if crit {
            dmg = dmg * crit_dmg / 100;
        }
        dmg = dmg * (95 + self.rng.below(11) as i64) / 100;
        let mut broke = false;
        {
            let u = self.unit_mut(to);
            if u.toughness > 0 {
                dmg = dmg * r_tough / 100;
                u.toughness -= 1;
                broke = u.toughness == 0;
            }
        }
        let killed = self.take_damage(to, dmg.max(1), crit, adv, graze, false);
        if broke && self.unit(to).alive() {
            self.events.push(Event::Break { unit: to.1 });
            self.put_status(to, StatusKind::Stun, 1, 0, Some(from));
        }
        Some((
            Hit {
                target: to,
                crit,
                killed,
            },
            graze,
        ))
    }

    /// 灼燒這類不看防禦的傷害
    pub(crate) fn direct_damage(&mut self, to: Who, amount: i64) {
        self.take_damage(to, amount.max(1), false, false, false, true);
    }

    /// 扣護盾再扣 HP；倒下回傳 true
    fn take_damage(
        &mut self,
        to: Who,
        amount: i64,
        crit: bool,
        adv: bool,
        graze: bool,
        dot: bool,
    ) -> bool {
        let u = self.unit_mut(to);
        let absorbed = amount.min(u.shield);
        u.shield -= absorbed;
        u.hp = (u.hp - (amount - absorbed)).max(0);
        let (hp, shield) = (u.hp, u.shield);
        self.events.push(Event::Damage {
            side: to.0,
            unit: to.1,
            amount,
            crit,
            adv,
            graze,
            dot,
            hp,
            shield,
        });
        if hp == 0 {
            let u = self.unit_mut(to);
            u.statuses.clear();
            u.shield = 0;
            u.charge = None;
            u.gauge = 0;
            self.events.push(Event::Down {
                side: to.0,
                unit: to.1,
            });
            return true;
        }
        if to.0 == Side::Enemy {
            self.check_phase(to);
        }
        false
    }

    pub(crate) fn heal(&mut self, to: Who, amount: i64) {
        let u = self.unit_mut(to);
        if !u.alive() {
            return;
        }
        let amount = if u.has(StatusKind::NoHeal) {
            0
        } else {
            amount.min(u.max_hp - u.hp)
        };
        u.hp += amount;
        let hp = u.hp;
        self.events.push(Event::Heal {
            side: to.0,
            unit: to.1,
            amount,
            hp,
        });
    }

    /// 減益會不會中：先看技能的機率，再看免疫與命中／抵抗。沒中而且是因為抵抗時發 Resisted
    pub(crate) fn debuff_lands(&mut self, from: Who, to: Who, chance: i64) -> bool {
        if !self.roll(chance) {
            return false;
        }
        let r = &self.rules;
        let (a, d) = (self.unit(from), self.unit(to));
        let mut p = 100 + a.eff(StatKey::Hit, &r.status) - d.eff(StatKey::Resist, &r.status);
        if a.element.beats(d.element) {
            p += r.adv_hit;
        }
        let p = p.clamp(r.hit_floor, r.hit_cap);
        let immune = d.has(StatusKind::Immune);
        if immune || !self.roll(p) {
            self.events.push(Event::Resisted {
                side: to.0,
                unit: to.1,
            });
            return false;
        }
        true
    }

    pub(crate) fn add_status(
        &mut self,
        from: Who,
        to: Who,
        kind: StatusKind,
        turns: u8,
        chance: i64,
    ) {
        if !self.unit(to).alive() {
            return;
        }
        if !kind.is_buff() && to.0 != from.0 && !self.debuff_lands(from, to, chance) {
            return;
        }
        let value = if kind == StatusKind::Burn {
            self.unit(from).eff(StatKey::Atk, &self.rules.status) * self.rules.status.burn / 100
        } else {
            0
        };
        self.put_status(to, kind, turns, value, Some(from));
    }

    /// 直接上狀態（不檢查命中）。同種類的狀態取較長的回合數
    pub(crate) fn put_status(
        &mut self,
        to: Who,
        kind: StatusKind,
        turns: u8,
        value: i64,
        source: Option<Who>,
    ) {
        let fresh = self.acting == Some(to);
        let u = self.unit_mut(to);
        match u.statuses.iter_mut().find(|s| s.kind == kind) {
            Some(s) => {
                s.turns = s.turns.max(turns);
                s.value = s.value.max(value);
                s.source = source.or(s.source);
                s.fresh |= fresh;
            }
            None => u.statuses.push(Status {
                kind,
                turns,
                value,
                source,
                fresh,
            }),
        }
        self.events.push(Event::Status {
            side: to.0,
            unit: to.1,
            status: kind,
            turns,
        });
        if kind == StatusKind::Stun && self.unit(to).charge.is_some() {
            self.unit_mut(to).charge = None;
            self.events.push(Event::Interrupted { unit: to.1 });
        }
    }

    /// 淨化（拿掉減益）或驅散（拿掉增益），從最新的開始
    fn remove_statuses(&mut self, to: Who, count: u8, buffs: bool) {
        let mut removed = Vec::new();
        let u = self.unit_mut(to);
        for _ in 0..count {
            let Some(i) = u.statuses.iter().rposition(|s| s.kind.is_buff() == buffs) else {
                break;
            };
            removed.push(u.statuses.remove(i).kind);
        }
        for status in removed {
            self.events.push(Event::Removed {
                side: to.0,
                unit: to.1,
                status,
            });
        }
    }

    /// 魔王 HP 掉到門檻以下：換技能、套用效果
    fn check_phase(&mut self, w: Who) {
        loop {
            let u = self.unit(w);
            let Some(p) = u.phases.get(u.phase) else {
                return;
            };
            if u.hp_pct() >= p.hp_below {
                return;
            }
            let p = p.clone();
            let u = self.unit_mut(w);
            u.phase += 1;
            let phase = u.phase;
            if let Some(ids) = &p.skills {
                let skills: Vec<_> = ids.iter().map(|id| self.book[id].clone()).collect();
                let u = self.unit_mut(w);
                u.skills = skills;
                u.skill_ids = ids.clone();
                u.cooldowns = vec![0; ids.len()];
                u.charge = None;
            }
            self.events.push(Event::Phase {
                unit: w.1,
                phase,
                name: p.name.clone(),
            });
            let mut hits = Vec::new();
            self.apply_effects(w, &p.effects, &[w], &mut hits);
        }
    }
}
