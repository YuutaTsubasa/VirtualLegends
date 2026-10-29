// 畫面用的遊戲資料：跟引擎、Server 讀同一份 repo 的 data/*.json（數值以 Server 與引擎為準，這裡只拿來顯示）。
// 玩家看到的「角色」＝ 某位系列角色的某個版本（例：小翠・夏日祭典），資料裡叫 costume。
import costumeList from '../../data/costumes.json';
import skillList from '../../data/skills.json';
import stageList from '../../data/stages.json';
import enemyList from '../../data/enemies.json';
import { CHARACTERS } from './characters.js';
import { t } from './i18n.svelte.js';

export const COSTUMES = new Map(costumeList.map((c) => [c.id, c]));
export const SKILLS = new Map(skillList.map((s) => [s.id, s]));
export const STAGES = stageList;
export const ENEMIES = new Map(enemyList.map((e) => [e.id, e]));

export const SUIT_NAME = { man: '萬', pin: '筒', sou: '條', honor: '字' };
export const ROLE_NAME = { attacker: '攻擊', guardian: '守護', support: '支援', disruptor: '妨礙' };
const TARGET_NAME = { enemy: '敵方單體', all_enemies: '敵方全體', ally: '我方單體', all_allies: '我方全體', self: '自己' };
const STAT_NAME = { atk: '攻擊', def: '防禦', spd: '速度' };

/** 角色的顯示名稱：「小翠・夏日祭典」 */
export function unitName(costumeId) {
  const c = COSTUMES.get(costumeId);
  return `${t(CHARACTERS.get(c.character).name)}・${t(c.name)}`;
}

export const characterOf = (costumeId) => CHARACTERS.get(COSTUMES.get(costumeId).character);

/** 等級上限看突破（跟 legends-core 的 stats::level_cap 一樣） */
export const levelCap = (ascension) => 20 + 8 * ascension;

/** 顯示用的能力值（跟 legends-core 的 stats::member_stats 一樣） */
export function unitStats(costumeId, level, ascension = 0, bond = 0) {
  const c = COSTUMES.get(costumeId);
  const pct = 100 + 5 * ascension + bond;
  const f = (k) => Math.floor(((c.base[k] + c.growth[k] * (level - 1)) * pct) / 100);
  return { hp: f('hp'), atk: f('atk'), def: f('def'), spd: f('spd') };
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

/** 技能效果寫成一句話 */
export function skillText(skill) {
  const parts = skill.effects.map((e) => {
    switch (e.type) {
      case 'damage':
        return t('傷害 {p}%', { p: e.power });
      case 'heal':
        return t('回復 {p}%', { p: e.power });
      case 'shield':
        return t('護盾 {p}%', { p: e.power });
      case 'buff':
        return t('{stat}{sign}{p}%（{n} 回合）', { stat: t(STAT_NAME[e.stat]), sign: e.pct > 0 ? '+' : '', p: e.pct, n: e.turns });
      case 'steal':
        return t('拿掉 {n} 張牌', { n: e.count });
      case 'draw':
        return t('摸 {n} 張牌', { n: e.count });
      default:
        return '';
    }
  });
  return `${t(TARGET_NAME[skill.target])}：${parts.join('、')}`;
}

export function skillCost(skill) {
  if (skill.kind === 'basic') return t('不花牌，再拿 1 張自己屬性的牌');
  if (skill.kind === 'ultimate') return t('花 3 張組成順子或刻子');
  return t('花 {n} 張自己屬性的牌', { n: skill.cost });
}
