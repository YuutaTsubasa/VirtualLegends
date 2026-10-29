import { test } from 'node:test';
import assert from 'node:assert/strict';
import { server, playAuto } from './helpers.js';
import { STAGES, STARTER } from '../src/data.js';

const TEAM = STARTER.costumes;

async function start(s, acc, stage = '1-1', team = TEAM) {
  return s.call('POST', '/v1/battle/start', { stage, team }, acc);
}

test('打第一關：Server 重算贏了才發獎勵，首通多給星屑', async () => {
  const s = server();
  const acc = await s.newAccount();
  const t = (await start(s, acc)).body;
  assert.equal(t.setup.team.length, 4);
  assert.match(t.seed, /^[0-9a-f]{16}$/);
  const play = playAuto(t);
  assert.equal(play.phase, 'won');
  const r = await s.call('POST', '/v1/battle/finish', { ticket: t.ticket, commands: play.commands, digest: play.digest }, acc);
  assert.equal(r.status, 200);
  assert.equal(r.body.outcome.won, true);
  assert.equal(r.body.desync, false);
  const { gold, notes, first_stardust } = STAGES[0].rewards;
  assert.deepEqual(r.body.rewards, { gold, notes, stardust: first_stardust, first: true });
  assert.equal(r.body.profile.stardust, STARTER.stardust + STAGES[0].rewards.first_stardust);
  assert.equal(r.body.profile.clears['1-1'], 1);

  // 第二次打：沒有首通獎勵
  const t2 = (await start(s, acc)).body;
  const r2 = await s.call('POST', '/v1/battle/finish', { ticket: t2.ticket, commands: playAuto(t2).commands }, acc);
  assert.equal(r2.body.rewards.stardust, 0);
  assert.equal(r2.body.profile.clears['1-1'], 2);
});

test('同一張票只能結算一次（同時送也一樣）', async () => {
  const s = server();
  const acc = await s.newAccount();
  const t = (await start(s, acc)).body;
  const { commands } = playAuto(t);
  const [a, b] = await Promise.all([
    s.call('POST', '/v1/battle/finish', { ticket: t.ticket, commands }, acc),
    s.call('POST', '/v1/battle/finish', { ticket: t.ticket, commands }, acc),
  ]);
  assert.deepEqual([a.status, b.status].sort(), [200, 409]);
  const again = await s.call('POST', '/v1/battle/finish', { ticket: t.ticket, commands }, acc);
  assert.deepEqual([again.status, again.body.error], [409, 'ticket_used']);
  const me = (await s.call('GET', '/v1/me', undefined, acc)).body.profile;
  assert.equal(me.clears['1-1'], 1);
});

test('竄改的指令紀錄：重算失敗、不發獎勵、票作廢', async () => {
  const s = server();
  const acc = await s.newAccount();
  const t = (await start(s, acc)).body;
  const { commands } = playAuto(t);
  const r = await s.call('POST', '/v1/battle/finish', { ticket: t.ticket, commands: commands.slice(0, -1) }, acc);
  assert.deepEqual([r.status, r.body.error], [422, 'replay_failed']);
  const retry = await s.call('POST', '/v1/battle/finish', { ticket: t.ticket, commands }, acc);
  assert.equal(retry.status, 409);
  const me = (await s.call('GET', '/v1/me', undefined, acc)).body.profile;
  assert.equal(me.clears['1-1'], undefined);
});

test('用自己挑的種子打：Server 用票上的種子重算，結果對不上', async () => {
  const s = server();
  const acc = await s.newAccount();
  const t = (await start(s, acc)).body;
  // 試很多種子，挑一個 Client 端打得最快的（作弊）
  const cheat = playAuto({ ...t, seed: 'ffffffffffffffff' });
  const r = await s.call('POST', '/v1/battle/finish', { ticket: t.ticket, commands: cheat.commands, digest: cheat.digest }, acc);
  if (r.status === 200) {
    // 指令剛好在真的種子下也合法：結果以 Server 為準，而且會標出 desync
    assert.equal(r.body.desync, true);
  } else {
    assert.equal(r.status, 422);
  }
});

test('過期的票、別人的票、沒解鎖的關卡、不合法的隊伍', async () => {
  const s = server();
  const acc = await s.newAccount();
  const other = await s.newAccount();
  const t = (await start(s, acc)).body;
  const { commands } = playAuto(t);
  assert.equal((await s.call('POST', '/v1/battle/finish', { ticket: t.ticket, commands }, other)).status, 404);
  s.clock += 31 * 60_000;
  assert.equal((await s.call('POST', '/v1/battle/finish', { ticket: t.ticket, commands }, acc)).status, 410);

  assert.deepEqual((await start(s, acc, '1-2')).body, { error: 'locked' });
  assert.equal((await start(s, acc, '1-1', ['leo.uniform'])).status, 404);
  assert.equal((await start(s, acc, '1-1', [...TEAM, 'aming.uniform'])).body.error, 'bad_team');
  assert.equal((await start(s, acc, '1-1', [])).body.error, 'bad_team');
});

test('打過 1-1 才解鎖 1-2', async () => {
  const s = server();
  const acc = await s.newAccount();
  const t = (await start(s, acc)).body;
  await s.call('POST', '/v1/battle/finish', { ticket: t.ticket, commands: playAuto(t).commands }, acc);
  assert.equal((await start(s, acc, '1-2')).status, 200);
});

test('戰鬥票上的隊伍是 Server 的等級，不是 Client 說的', async () => {
  const s = server();
  const acc = await s.newAccount();
  const r = await s.call('POST', '/v1/battle/start', { stage: '1-1', team: TEAM, levels: [60, 60, 60, 60] }, acc);
  assert.ok(r.body.setup.team.every((m) => m.level === 1));
});
