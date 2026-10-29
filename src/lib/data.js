// 畫面用的遊戲資料：跟引擎、Server 讀同一份 repo 的 data/*.json（數值以 Server 與引擎為準，這裡只拿來顯示）。
// 玩家看到的「角色」＝ 某位系列角色的某個版本（例：小翠・夏日祭典），資料裡叫 costume。
import costumeList from '../../data/costumes.json';
import skillList from '../../data/skills.json';
import stageList from '../../data/stages.json';
import enemyList from '../../data/enemies.json';
import rules from '../../data/rules.json';
import { CHARACTERS } from './characters.js';
import { t } from './i18n.svelte.js';

export const COSTUMES = new Map(costumeList.map((c) => [c.id, c]));
export const SKILLS = new Map(skillList.map((s) => [s.id, s]));
export const STAGES = stageList;
export const ENEMIES = new Map(enemyList.map((e) => [e.id, e]));
export const RULES = rules;

export const ELEMENT_NAME = { blaze: '焰', tide: '潮', gale: '嵐', dawn: '曉', dusk: '暮' };
export const ROLE_NAME = { attacker: '攻擊', guardian: '守護', support: '支援', disruptor: '妨礙' };
export const STAT_NAME = { hp: 'HP', atk: '攻擊', def: '防禦', spd: '速度', crit: '暴擊', crit_dmg: '暴擊傷害', hit: '效果命中', resist: '效果抵抗' };
export const STATUS_NAME = {
  atk_up: '攻擊提升',
  def_up: '防禦提升',
  spd_up: '速度提升',
  crit_up: '暴擊提升',
  immune: '免疫',
  counter: '反擊架勢',
  atk_down: '攻擊降低',
  def_down: '防禦降低',
  spd_down: '速度降低',
  hit_down: '命中降低',
  burn: '灼燒',
  stun: '暈眩',
  taunt: '挑釁',
  silence: '封印',
  no_heal: '阻止回復',
};
export const isBuff = (s) => ['atk_up', 'def_up', 'spd_up', 'crit_up', 'immune', 'counter'].includes(s);
const TARGET_NAME = { enemy: '敵方單體', all_enemies: '敵方全體', ally: '我方單體', all_allies: '我方全體', self: '自己' };
const TO_NAME = { self: '自己', all_allies: '我方全體', all_enemies: '敵方全體' };
const SCALE_NAME = { atk: '攻擊', def: '防禦', max_hp: '最大 HP' };

/** 角色的顯示名稱：「小翠・夏日祭典」 */
export function unitName(costumeId) {
  const c = COSTUMES.get(costumeId);
  return `${t(CHARACTERS.get(c.character).name)}・${t(c.name)}`;
}

export const characterOf = (costumeId) => CHARACTERS.get(COSTUMES.get(costumeId).character);

/** 等級上限看突破（跟 legends-core 的 stats::level_cap 一樣） */
export const levelCap = (ascension) => 20 + 8 * ascension;

const RATES = ['crit', 'crit_dmg', 'hit', 'resist'];
/** 顯示用的能力值（跟 legends-core 的 stats::base_stats 一樣；還沒算裝備） */
export function unitStats(costumeId, level, ascension = 0, bond = 0) {
  const c = COSTUMES.get(costumeId);
  const pct = 100 + 5 * ascension + bond;
  const out = {};
  for (const k of ['hp', 'atk', 'def', 'spd', ...RATES]) {
    const grown = (c.base[k] ?? 0) + (c.growth[k] ?? 0) * (level - 1);
    out[k] = (rules.base[k] ?? 0) + (RATES.includes(k) ? grown : Math.floor((grown * pct) / 100));
  }
  return out;
}

/** 升級花費（跟 Server 的 data.js 一樣） */
export function levelCost(from, to) {
  let gold = 0;
  let notes = 0;
  for (let lv = from; lv < to; lv++) {
    gold += 50 * lv;
    notes += 5 * lv;
  }
  return { gold, notes };
}
export const ascendCost = (asc) => 1000 * (asc + 1);

/** 一個效果寫成一小句 */
function effectText(e) {
  const to = e.to && e.to !== 'target' ? `${t(TO_NAME[e.to])}` : '';
  const scale = e.scale && e.scale !== 'atk' ? t(SCALE_NAME[e.scale]) : '';
  switch (e.type) {
    case 'damage':
      return t('{to}傷害 {scale}{p}%', { to, scale, p: e.power });
    case 'heal':
      return t('{to}回復 {scale}{p}%', { to, scale, p: e.power });
    case 'shield':
      return t('{to}護盾 {scale}{p}%', { to, scale, p: e.power });
    case 'status': {
      const chance = e.chance !== undefined && e.chance < 100 ? t('（{c}%）', { c: e.chance }) : '';
      return t('{to}{s} {n} 回合{chance}', { to, s: t(STATUS_NAME[e.status]), n: e.turns, chance });
    }
    case 'gauge':
      return e.pct > 0 ? t('{to}加速 {p}%', { to, p: e.pct }) : t('{to}延遲 {p}%', { to, p: -e.pct });
    case 'cleanse':
      return t('{to}淨化 {n} 個減益', { to, n: e.count });
    case 'dispel':
      return t('{to}驅散 {n} 個增益', { to, n: e.count });
    case 'cooldown':
      return t('{to}冷卻 −{n}', { to, n: e.amount });
    default:
      return '';
  }
}

/** 技能 level 級的效果（套用強化） */
export function skillAt(skill, level = 1) {
  let s = { ...skill };
  for (const u of (skill.upgrades ?? []).slice(0, level - 1)) {
    if (u.cooldown !== undefined) s.cooldown = u.cooldown;
    if (u.effects) s.effects = u.effects;
  }
  return s;
}

/** 技能效果寫成一句話 */
export function skillText(skill) {
  return `${t(TARGET_NAME[skill.target])}：${skill.effects.map(effectText).join('、')}`;
}

export const cooldownText = (skill) => (skill.cooldown ? t('冷卻 {n}', { n: skill.cooldown }) : t('沒有冷卻'));

const COND_TEXT = {
  target_hp_below: (n) => t('目標 HP 低於 {n}% 時', { n }),
  target_has: (s) => t('目標{s}時', { s: t(STATUS_NAME[s]) }),
  self_hp_below: (n) => t('自己 HP 低於 {n}% 時', { n }),
};
const ON_TEXT = {
  hit: '被打時',
  ally_attack: '隊友攻擊時',
  after_attack: '攻擊之後',
  turn_start: '回合開始時',
  battle_start: '戰鬥開始時',
  kill: '打倒敵人時',
};

/** 被動寫成一句話 */
export function passiveText(p) {
  const cond = p.cond ? Object.entries(p.cond).map(([k, v]) => COND_TEXT[k]?.(v) ?? '')[0] : '';
  const chance = p.chance !== undefined && p.chance < 100 ? t('{c}% ', { c: p.chance }) : '';
  const act = {
    counter: () => t('反擊'),
    link: () => t('連攜'),
    pursuit: () => t('追擊 {p}%', { p: p.act.power }),
    effects: () => p.act.effects.map(effectText).join('、'),
  }[p.act.kind]();
  return `${t(ON_TEXT[p.on])}${cond}${chance}${act}`;
}
