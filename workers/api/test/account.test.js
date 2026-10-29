import { test } from 'node:test';
import assert from 'node:assert/strict';
import { server } from './helpers.js';
import { dayOf } from '../src/app.js';
import { STARTER, DAILY } from '../src/data.js';

test('新帳號拿到起始角色與貨幣', async () => {
  const s = server();
  const acc = await s.newAccount();
  assert.match(acc.id, /^[0-9a-f]{32}$/);
  assert.match(acc.key, /^[0-9a-f]{64}$/);
  assert.deepEqual(acc.profile.costumes.map((c) => c.id).sort(), [...STARTER.costumes].sort());
  assert.equal(acc.profile.stardust, STARTER.stardust);
  // 金鑰只存雜湊
  const row = s.db.raw.prepare('SELECT key_hash FROM users WHERE id = ?').get(acc.id);
  assert.notEqual(row.key_hash, acc.key);
});

test('沒有或錯的金鑰不能用', async () => {
  const s = server();
  const acc = await s.newAccount();
  assert.equal((await s.call('GET', '/v1/me')).status, 401);
  assert.equal((await s.call('GET', '/v1/me', undefined, { id: acc.id, key: 'f'.repeat(64) })).status, 401);
  assert.equal((await s.call('GET', '/v1/me', undefined, acc)).status, 200);
});

test('每日登入獎勵一天一次，台灣時間凌晨 4 點換日', async () => {
  const s = server();
  const acc = await s.newAccount();
  const first = await s.call('GET', '/v1/me', undefined, acc);
  assert.deepEqual(first.body.daily, DAILY);
  assert.equal(first.body.profile.stardust, STARTER.stardust + DAILY.stardust);
  assert.equal((await s.call('GET', '/v1/me', undefined, acc)).body.daily, null);
  s.clock += 24 * 3_600_000;
  assert.deepEqual((await s.call('GET', '/v1/me', undefined, acc)).body.daily, DAILY);
  // 台灣 03:59 和 04:00 是不同天
  const at = (h, m) => Date.UTC(2026, 9, 30, h - 8, m);
  assert.equal(dayOf(at(3, 59)) + 1, dayOf(at(4, 0)));
});

test('繼承碼換裝置：舊金鑰失效、碼只能用一次', async () => {
  const s = server();
  const acc = await s.newAccount();
  const { body } = await s.call('POST', '/v1/account/transfer', {}, acc);
  assert.match(body.code, /^[2-9A-HJ-NP-Z]{12}$/);
  const claimed = await s.call('POST', '/v1/account/claim', { code: body.code.toLowerCase() });
  assert.equal(claimed.status, 200);
  assert.equal(claimed.body.id, acc.id);
  assert.equal((await s.call('GET', '/v1/me', undefined, acc)).status, 401);
  assert.equal((await s.call('GET', '/v1/me', undefined, claimed.body)).status, 200);
  assert.equal((await s.call('POST', '/v1/account/claim', { code: body.code })).status, 404);
});

test('繼承碼過期', async () => {
  const s = server();
  const acc = await s.newAccount();
  const { body } = await s.call('POST', '/v1/account/transfer', {}, acc);
  s.clock += 8 * 24 * 3_600_000;
  assert.equal((await s.call('POST', '/v1/account/claim', { code: body.code })).status, 404);
});
