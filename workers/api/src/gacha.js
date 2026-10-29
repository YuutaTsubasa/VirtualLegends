// 轉蛋：機率以萬分之一為單位（rates 加起來 10000）。亂數由呼叫端給（Server 用 crypto.getRandomValues）。
// - 保底：距離上一個 ★5 第 pity 抽必出 ★5
// - 十連：十張裡沒有 ★4 以上時，最後一張換成 ★4（從 ★4 卡池抽）
import { COSTUMES } from './data.js';

/** 卡池依稀有度分組 */
export function poolByRarity(banner) {
  const groups = {};
  for (const id of banner.pool) {
    const r = COSTUMES.get(id).rarity;
    (groups[r] ??= []).push(id);
  }
  return groups;
}

/** 遊戲內公開的機率：每張卡的機率（萬分之一，可能有小數）＋保底說明 */
export function publicRates(banner) {
  const groups = poolByRarity(banner);
  const cards = banner.pool.map((id) => {
    const c = COSTUMES.get(id);
    return { costume: id, rarity: c.rarity, rate: banner.rates[c.rarity] / groups[c.rarity].length };
  });
  return {
    id: banner.id,
    name: banner.name,
    cost: banner.cost,
    ten_cost: banner.ten_cost,
    rates: banner.rates,
    pity: banner.pity,
    ten_min_rarity: banner.ten_min_rarity,
    cards,
  };
}

/** randInt(n) 回傳 0..n-1 的整數 */
export function rollRarity(banner, randInt) {
  let x = randInt(10000);
  for (const r of Object.keys(banner.rates).map(Number).sort((a, b) => b - a)) {
    x -= banner.rates[r];
    if (x < 0) return r;
  }
  return Math.min(...Object.keys(banner.rates).map(Number));
}

/**
 * 抽 count 次。pity 是抽之前距離上一個 ★5 的次數。
 * 回傳 { results: [{ costume, rarity }], pity: 抽完後的 pity }
 */
export function pull(banner, count, pity, randInt) {
  const groups = poolByRarity(banner);
  const pick = (r) => groups[r][randInt(groups[r].length)];
  const results = [];
  for (let i = 0; i < count; i++) {
    let rarity = pity + 1 >= banner.pity ? 5 : rollRarity(banner, randInt);
    if (count === 10 && i === 9 && rarity < banner.ten_min_rarity && results.every((r) => r.rarity < banner.ten_min_rarity)) {
      rarity = banner.ten_min_rarity;
    }
    pity = rarity === 5 ? 0 : pity + 1;
    results.push({ costume: pick(rarity), rarity });
  }
  return { results, pity };
}
