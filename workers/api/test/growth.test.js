import { test } from 'node:test';
import assert from 'node:assert/strict';
import { server } from './helpers.js';
import { STARTER, levelCost } from '../src/data.js';

test('升級扣金幣與筆記、加羈絆，不能超過等級上限', async () => {
  const s = server();
  const acc = await s.newAccount();
  s.db.raw.prepare('UPDATE users SET gold = 1000000, notes = 100000 WHERE id = ?').run(acc.id);
  const r = await s.call('POST', '/v1/costumes/level', { costume: 'aming.uniform', levels: 19 }, acc);
  assert.equal(r.status, 200);
  let gold = 0;
  for (let lv = 1; lv < 20; lv++) gold += levelCost(lv).gold;
  assert.equal(r.body.spent.gold, gold);
  assert.equal(r.body.profile.costumes.find((c) => c.id === 'aming.uniform').level, 20);
  assert.deepEqual(r.body.profile.bonds.aming, { exp: 190, level: 1 });
  const over = await s.call('POST', '/v1/costumes/level', { costume: 'aming.uniform', levels: 1 }, acc);
  assert.deepEqual([over.status, over.body.error], [400, 'level_cap']);
});

test('錢不夠不能升級、沒有的角色不能升', async () => {
  const s = server();
  const acc = await s.newAccount();
  assert.equal((await s.call('POST', '/v1/costumes/level', { costume: 'aming.uniform', levels: 19 }, acc)).body.error, 'not_enough');
  assert.equal((await s.call('POST', '/v1/costumes/level', { costume: 'leo.uniform', levels: 1 }, acc)).status, 404);
  const me = (await s.call('GET', '/v1/me', undefined, acc)).body.profile;
  assert.equal(me.gold, STARTER.gold + 1000);
});

test('突破：要練滿、要有重複抽到的次數', async () => {
  const s = server();
  const acc = await s.newAccount();
  s.db.raw.prepare('UPDATE users SET gold = 1000000, notes = 100000 WHERE id = ?').run(acc.id);
  assert.equal((await s.call('POST', '/v1/costumes/ascend', { costume: 'aming.uniform' }, acc)).body.error, 'no_dupes');
  s.db.raw.prepare("UPDATE costumes SET dupes = 1 WHERE user_id = ? AND costume_id = 'aming.uniform'").run(acc.id);
  assert.equal((await s.call('POST', '/v1/costumes/ascend', { costume: 'aming.uniform' }, acc)).body.error, 'level_not_capped');
  await s.call('POST', '/v1/costumes/level', { costume: 'aming.uniform', levels: 19 }, acc);
  const r = await s.call('POST', '/v1/costumes/ascend', { costume: 'aming.uniform' }, acc);
  assert.equal(r.status, 200);
  assert.deepEqual(r.body.profile.costumes.find((c) => c.id === 'aming.uniform'), { id: 'aming.uniform', level: 20, ascension: 1, dupes: 0 });
  // 突破後上限 28
  assert.equal((await s.call('POST', '/v1/costumes/level', { costume: 'aming.uniform', levels: 8 }, acc)).status, 200);
});
