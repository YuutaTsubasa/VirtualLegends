import { test } from 'node:test';
import assert from 'node:assert/strict';
import { randomInt } from 'node:crypto';
import { server } from './helpers.js';
import { BANNERS, COSTUMES } from '../src/data.js';
import { publicRates, pull, rollRarity } from '../src/gacha.js';

const standard = BANNERS.get('standard');

/** 永遠擲出 4294959999（拒絕取樣的上限以下；% 10000 = 9999 → 最低稀有度，% 10 = 9 → 卡池最後一張）的亂數 */
const alwaysLow = (n) => {
  const out = new Uint8Array(n);
  for (let i = 0; i < n; i++) out[i] = [0xff, 0xff, 0xe3, 0x7f][i % 4];
  return out;
};

test('卡池資料：每張卡都存在，公開機率加起來是 100%', () => {
  for (const b of BANNERS.values()) {
    for (const id of b.pool) assert.ok(COSTUMES.has(id), id);
    assert.equal(Object.values(b.rates).reduce((a, x) => a + x, 0), 10000);
    const total = publicRates(b).cards.reduce((a, c) => a + c.rate, 0);
    assert.ok(Math.abs(total - 10000) < 1e-6, total);
  }
});

test('抽 20 萬次，各稀有度的比例接近公開機率', () => {
  const n = 200_000;
  const count = { 3: 0, 4: 0, 5: 0 };
  for (let i = 0; i < n; i++) count[rollRarity(standard, randomInt)]++;
  for (const r of [3, 4, 5]) {
    const expected = standard.rates[r] / 10000;
    assert.ok(Math.abs(count[r] / n - expected) < 0.004, `★${r}: ${count[r] / n} vs ${expected}`);
  }
});

test('十連至少一張 ★4、第 80 抽保底 ★5', () => {
  let pity = 0;
  for (let k = 0; k < 7; k++) {
    const r = pull(standard, 10, pity, () => 9999);
    assert.equal(r.results[9].rarity, 4);
    assert.ok(r.results.slice(0, 9).every((x) => x.rarity === 3));
    pity = r.pity;
  }
  assert.equal(pity, 70);
  const last = pull(standard, 10, pity, () => 9999);
  assert.equal(last.results[9].rarity, 5);
  assert.equal(last.pity, 0);
});

test('Server 扣星屑、寫進庫存與紀錄', async () => {
  const s = server();
  const acc = await s.newAccount();
  const { status, body } = await s.call('POST', '/v1/gacha', { banner: 'standard', count: 10 }, acc);
  assert.equal(status, 200);
  assert.equal(body.results.length, 10);
  assert.equal(body.profile.stardust, 0);
  assert.equal(body.profile.pulls, 10);
  const owned = new Set(body.profile.costumes.map((c) => c.id));
  for (const r of body.results) assert.ok(owned.has(r.costume));
  assert.equal(s.db.raw.prepare('SELECT COUNT(*) AS n FROM pulls WHERE user_id = ?').get(acc.id).n, 10);
  // 沒錢了
  const broke = await s.call('POST', '/v1/gacha', { banner: 'standard', count: 1 }, acc);
  assert.deepEqual([broke.status, broke.body.error], [400, 'not_enough_stardust']);
});

test('重複抽到變成突破次數，滿了換星屑與羈絆', async () => {
  const s = server();
  const acc = await s.newAccount();
  s.db.raw.prepare('UPDATE users SET stardust = 100000 WHERE id = ?').run(acc.id);
  s.random = alwaysLow; // 都抽到 ★3，卡池最後一張 ★3 是阿烈的制服
  const results = [];
  for (let i = 0; i < 7; i++) results.push((await s.call('POST', '/v1/gacha', { banner: 'standard', count: 1 }, acc)).body);
  s.random = null;
  assert.deepEqual(
    results.map((r) => r.results[0].result),
    ['new', 'dupe', 'dupe', 'dupe', 'dupe', 'dupe', 'overflow'],
  );
  const last = results[6].profile;
  assert.equal(last.costumes.find((c) => c.id === 'alie.uniform').dupes, 5);
  assert.equal(last.bonds.alie.exp, 20);
  assert.equal(last.stardust, 100000 - 700 + 10);
});

test('同時送兩次十連，只有一次成功', async () => {
  const s = server();
  const acc = await s.newAccount();
  const [a, b] = await Promise.all([
    s.call('POST', '/v1/gacha', { banner: 'standard', count: 10 }, acc),
    s.call('POST', '/v1/gacha', { banner: 'standard', count: 10 }, acc),
  ]);
  assert.deepEqual([a.status, b.status].sort(), [200, 409]);
  const me = (await s.call('GET', '/v1/me', undefined, acc)).body.profile;
  assert.equal(me.pulls, 10);
  assert.equal(s.db.raw.prepare('SELECT COUNT(*) AS n FROM pulls WHERE user_id = ?').get(acc.id).n, 10);
});

test('公開機率的 API', async () => {
  const s = server();
  const { body } = await s.call('GET', '/v1/banners');
  const b = body.banners.find((x) => x.id === 'standard');
  assert.equal(b.pity, 80);
  assert.equal(b.cards.find((c) => c.costume === 'yuehua.moonlit').rate, 150);
});
