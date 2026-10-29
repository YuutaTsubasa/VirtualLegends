// 虛擬新傳說的 Server：帳號、轉蛋、庫存、養成、戰鬥票與結算。Server 是權威，Client 只送「我想做什麼」。
//
// handle(request, deps)：
//   deps.db           D1（或測試用的 node:sqlite 轉接，見 test/d1.js）
//   deps.engine       { simulate, checkSetup, levelCap }（Rust 引擎的 WebAssembly）
//   deps.now()        毫秒
//   deps.randomBytes(n)
//
// 驗證：Authorization: Bearer <帳號 id>.<裝置金鑰>
// 會改狀態的請求：讀 users（含 rev）→ 算好 → 一個 batch 寫入，users 那句用 rev 當條件並換新 rev，
// 其他句用 EXISTS(rev = 新 rev) 當條件。users 那句沒改到（同時有別的請求）就回 409，Client 重試。
import {
  BANNERS, COSTUMES, DAILY, MAX_ASCENSION, OVERFLOW, STAGES, STAGE_INDEX, STARTER,
  TICKET_MINUTES, TRANSFER_DAYS, ascendCost, bondLevel, levelCost,
} from './data.js';
import { publicRates, pull } from './gacha.js';

const DAY = 86_400_000;
const HOUR = 3_600_000;
/** 台灣時間（UTC+8）凌晨 4 點換日 */
export const dayOf = (ms) => Math.floor((ms + 4 * HOUR) / DAY);

class HttpError extends Error {
  constructor(status, code) {
    super(code);
    this.status = status;
    this.code = code;
  }
}
const fail = (status, code) => {
  throw new HttpError(status, code);
};

const CORS = {
  'access-control-allow-origin': '*',
  'access-control-allow-methods': 'GET, POST, OPTIONS',
  'access-control-allow-headers': 'authorization, content-type',
};
const json = (body, status = 200) =>
  new Response(JSON.stringify(body), { status, headers: { 'content-type': 'application/json; charset=utf-8', ...CORS } });

const hex = (bytes) => [...bytes].map((b) => b.toString(16).padStart(2, '0')).join('');

async function sha256(text) {
  return hex(new Uint8Array(await crypto.subtle.digest('SHA-256', new TextEncoder().encode(text))));
}

/** 0..n-1 的均勻整數（拒絕取樣，沒有取餘數的偏差） */
function makeRandInt(randomBytes) {
  return (n) => {
    const limit = Math.floor(0x1_0000_0000 / n) * n;
    for (;;) {
      const b = randomBytes(4);
      const x = ((b[0] << 24) | (b[1] << 16) | (b[2] << 8) | b[3]) >>> 0;
      if (x < limit) return x % n;
    }
  };
}

// ---- 路由 ----

const routes = {
  'POST /v1/account': createAccount,
  'POST /v1/account/transfer': withUser(issueTransfer),
  'POST /v1/account/claim': claimTransfer,
  'GET /v1/me': withUser(me),
  'GET /v1/banners': banners,
  'POST /v1/gacha': withUser(gacha),
  'POST /v1/costumes/level': withUser(levelUp),
  'POST /v1/costumes/ascend': withUser(ascend),
  'POST /v1/battle/start': withUser(startBattle),
  'POST /v1/battle/finish': withUser(finishBattle),
};

export async function handle(request, deps) {
  if (request.method === 'OPTIONS') return new Response(null, { status: 204, headers: CORS });
  const url = new URL(request.url);
  const route = routes[`${request.method} ${url.pathname}`];
  if (!route) return json({ error: 'not_found' }, 404);
  try {
    let body = {};
    if (request.method === 'POST') {
      const text = await request.text();
      try {
        body = text ? JSON.parse(text) : {};
      } catch {
        fail(400, 'bad_json');
      }
    }
    return json(await route({ request, body, deps }));
  } catch (e) {
    if (e instanceof HttpError) return json({ error: e.code }, e.status);
    console.error(e);
    return json({ error: 'internal' }, 500);
  }
}

function withUser(handler) {
  return async (ctx) => {
    const auth = ctx.request.headers.get('authorization') ?? '';
    const m = /^Bearer ([0-9a-f]{32})\.([0-9a-f]{64})$/.exec(auth);
    if (!m) fail(401, 'unauthorized');
    const user = await ctx.deps.db.prepare('SELECT * FROM users WHERE id = ?').bind(m[1]).first();
    if (!user || user.key_hash !== (await sha256(m[2]))) fail(401, 'unauthorized');
    return handler({ ...ctx, user });
  };
}

// ---- 讀取 ----

async function profile(db, userId) {
  const [user, costumes, bonds, clears] = await Promise.all([
    db.prepare('SELECT stardust, gold, notes, pity, pulls, daily_day FROM users WHERE id = ?').bind(userId).first(),
    db.prepare('SELECT costume_id, level, ascension, dupes FROM costumes WHERE user_id = ? ORDER BY obtained_at, costume_id').bind(userId).all(),
    db.prepare('SELECT character_id, exp FROM bonds WHERE user_id = ? ORDER BY character_id').bind(userId).all(),
    db.prepare('SELECT stage, wins FROM clears WHERE user_id = ?').bind(userId).all(),
  ]);
  return {
    id: userId,
    stardust: user.stardust,
    gold: user.gold,
    notes: user.notes,
    pity: user.pity,
    pulls: user.pulls,
    costumes: costumes.results.map((c) => ({ id: c.costume_id, level: c.level, ascension: c.ascension, dupes: c.dupes })),
    bonds: Object.fromEntries(bonds.results.map((b) => [b.character_id, { exp: b.exp, level: bondLevel(b.exp) }])),
    clears: Object.fromEntries(clears.results.map((c) => [c.stage, c.wins])),
  };
}

/** 新的 rev 與「這批寫入只在 rev 換成功時生效」的條件 */
function revision(deps, user) {
  const rev = hex(deps.randomBytes(8));
  return {
    rev,
    guard: 'EXISTS (SELECT 1 FROM users WHERE id = ? AND rev = ?)',
    args: [user.id, rev],
  };
}

async function commit(db, statements) {
  const results = await db.batch(statements);
  if (results[0].meta.changes !== 1) fail(409, 'conflict');
  return results;
}

// ---- 帳號 ----

async function createAccount({ deps }) {
  const id = hex(deps.randomBytes(16));
  const key = hex(deps.randomBytes(32));
  const now = deps.now();
  const { db } = deps;
  await db.batch([
    db
      .prepare('INSERT INTO users (id, key_hash, rev, created_at, stardust, gold, notes) VALUES (?, ?, ?, ?, ?, ?, ?)')
      .bind(id, await sha256(key), hex(deps.randomBytes(8)), now, STARTER.stardust, STARTER.gold, STARTER.notes),
    ...STARTER.costumes.map((c) =>
      db.prepare('INSERT INTO costumes (user_id, costume_id, level, obtained_at) VALUES (?, ?, 1, ?)').bind(id, c, now),
    ),
  ]);
  return { id, key, profile: await profile(db, id) };
}

/** 換裝置用的繼承碼：12 個字（不含容易看錯的 0 O 1 I），7 天有效，一次只有一組 */
async function issueTransfer({ user, deps }) {
  const alphabet = '23456789ABCDEFGHJKLMNPQRSTUVWXYZ';
  const randInt = makeRandInt(deps.randomBytes);
  const code = Array.from({ length: 12 }, () => alphabet[randInt(alphabet.length)]).join('');
  const expires = deps.now() + TRANSFER_DAYS * DAY;
  await deps.db
    .prepare('UPDATE users SET transfer_hash = ?, transfer_expires = ? WHERE id = ?')
    .bind(await sha256(code), expires, user.id)
    .run();
  return { code, expires_at: expires };
}

/** 用繼承碼取回帳號：發新的裝置金鑰（舊裝置的金鑰失效），繼承碼用掉 */
async function claimTransfer({ body, deps }) {
  const code = String(body.code ?? '').toUpperCase().replace(/[^0-9A-Z]/g, '');
  if (code.length !== 12) fail(400, 'bad_code');
  const user = await deps.db.prepare('SELECT id, transfer_expires FROM users WHERE transfer_hash = ?').bind(await sha256(code)).first();
  if (!user || user.transfer_expires < deps.now()) fail(404, 'bad_code');
  const key = hex(deps.randomBytes(32));
  await deps.db
    .prepare('UPDATE users SET key_hash = ?, transfer_hash = NULL, transfer_expires = NULL WHERE id = ?')
    .bind(await sha256(key), user.id)
    .run();
  return { id: user.id, key, profile: await profile(deps.db, user.id) };
}

/** 個人資料；當天第一次讀時發每日登入獎勵 */
async function me({ user, deps }) {
  const today = dayOf(deps.now());
  let daily = null;
  if (user.daily_day < today) {
    const r = await deps.db
      .prepare('UPDATE users SET stardust = stardust + ?, gold = gold + ?, daily_day = ? WHERE id = ? AND daily_day < ?')
      .bind(DAILY.stardust, DAILY.gold, today, user.id, today)
      .run();
    if (r.meta.changes === 1) daily = DAILY;
  }
  return { daily, profile: await profile(deps.db, user.id) };
}

// ---- 轉蛋 ----

function banners() {
  return { banners: [...BANNERS.values()].map(publicRates) };
}

async function gacha({ user, body, deps }) {
  const banner = BANNERS.get(body.banner);
  if (!banner) fail(400, 'bad_banner');
  const count = body.count;
  if (count !== 1 && count !== 10) fail(400, 'bad_count');
  const cost = count === 10 ? banner.ten_cost : banner.cost;
  if (user.stardust < cost) fail(400, 'not_enough_stardust');

  const { db } = deps;
  const now = deps.now();
  const owned = new Map(
    (await db.prepare('SELECT costume_id, ascension, dupes FROM costumes WHERE user_id = ?').bind(user.id).all()).results.map(
      (c) => [c.costume_id, { ...c, isNew: false }],
    ),
  );
  const { results, pity } = pull(banner, count, user.pity, makeRandInt(deps.randomBytes));

  // 套到庫存：新的加進去；重複的變成突破次數；已經滿突破的換成星屑與羈絆經驗
  let refund = 0;
  const bondGain = new Map();
  const shown = results.map(({ costume, rarity }) => {
    const c = owned.get(costume);
    if (!c) {
      owned.set(costume, { costume_id: costume, ascension: 0, dupes: 0, isNew: true, touched: true });
      return { costume, rarity, result: 'new' };
    }
    c.touched = true;
    if (c.ascension + c.dupes < MAX_ASCENSION) {
      c.dupes += 1;
      return { costume, rarity, result: 'dupe' };
    }
    const o = OVERFLOW[rarity];
    refund += o.stardust;
    const ch = COSTUMES.get(costume).character;
    bondGain.set(ch, (bondGain.get(ch) ?? 0) + o.bond);
    return { costume, rarity, result: 'overflow', stardust: o.stardust, bond: o.bond };
  });

  const g = revision(deps, user);
  const statements = [
    db
      .prepare('UPDATE users SET stardust = stardust - ? + ?, pity = ?, pulls = pulls + ?, rev = ? WHERE id = ? AND rev = ?')
      .bind(cost, refund, pity, count, g.rev, user.id, user.rev),
  ];
  for (const c of owned.values()) {
    if (!c.touched) continue;
    statements.push(
      c.isNew
        ? db
            .prepare(`INSERT INTO costumes (user_id, costume_id, level, dupes, obtained_at) SELECT ?, ?, 1, ?, ? WHERE ${g.guard}`)
            .bind(user.id, c.costume_id, c.dupes, now, ...g.args)
        : db.prepare(`UPDATE costumes SET dupes = ? WHERE user_id = ? AND costume_id = ? AND ${g.guard}`).bind(c.dupes, user.id, c.costume_id, ...g.args),
    );
  }
  for (const [ch, exp] of bondGain) statements.push(addBond(db, g, user.id, ch, exp));
  for (const r of results) {
    statements.push(
      db
        .prepare(`INSERT INTO pulls (user_id, banner, costume_id, rarity, created_at) SELECT ?, ?, ?, ?, ? WHERE ${g.guard}`)
        .bind(user.id, banner.id, r.costume, r.rarity, now, ...g.args),
    );
  }
  await commit(db, statements);
  return { results: shown, profile: await profile(db, user.id) };
}

function addBond(db, g, userId, character, exp) {
  return db
    .prepare(
      `INSERT INTO bonds (user_id, character_id, exp) SELECT ?, ?, ? WHERE ${g.guard}
       ON CONFLICT (user_id, character_id) DO UPDATE SET exp = exp + excluded.exp`,
    )
    .bind(userId, character, exp, ...g.args);
}

// ---- 養成 ----

async function ownedCostume(db, userId, costumeId) {
  const c = await db.prepare('SELECT * FROM costumes WHERE user_id = ? AND costume_id = ?').bind(userId, costumeId).first();
  if (!c) fail(404, 'no_costume');
  return c;
}

/** 升 levels 級（不能超過突破的等級上限）；每升一級，角色羈絆 +10 */
async function levelUp({ user, body, deps }) {
  const { db, engine } = deps;
  const levels = body.levels;
  if (!Number.isInteger(levels) || levels < 1) fail(400, 'bad_levels');
  const c = await ownedCostume(db, user.id, body.costume);
  const target = c.level + levels;
  if (target > engine.levelCap(c.ascension)) fail(400, 'level_cap');
  let gold = 0;
  let notes = 0;
  for (let lv = c.level; lv < target; lv++) {
    gold += levelCost(lv).gold;
    notes += levelCost(lv).notes;
  }
  if (user.gold < gold || user.notes < notes) fail(400, 'not_enough');
  const g = revision(deps, user);
  await commit(db, [
    db.prepare('UPDATE users SET gold = gold - ?, notes = notes - ?, rev = ? WHERE id = ? AND rev = ?').bind(gold, notes, g.rev, user.id, user.rev),
    db.prepare(`UPDATE costumes SET level = ? WHERE user_id = ? AND costume_id = ? AND level = ? AND ${g.guard}`).bind(target, user.id, c.costume_id, c.level, ...g.args),
    addBond(db, g, user.id, COSTUMES.get(c.costume_id).character, 10 * levels),
  ]);
  return { spent: { gold, notes }, profile: await profile(db, user.id) };
}

/** 突破：要先練到目前的等級上限，花一次重複抽到的次數和金幣 */
async function ascend({ user, body, deps }) {
  const { db, engine } = deps;
  const c = await ownedCostume(db, user.id, body.costume);
  if (c.ascension >= MAX_ASCENSION) fail(400, 'max_ascension');
  if (c.dupes < 1) fail(400, 'no_dupes');
  if (c.level < engine.levelCap(c.ascension)) fail(400, 'level_not_capped');
  const gold = ascendCost(c.ascension);
  if (user.gold < gold) fail(400, 'not_enough');
  const g = revision(deps, user);
  await commit(db, [
    db.prepare('UPDATE users SET gold = gold - ?, rev = ? WHERE id = ? AND rev = ?').bind(gold, g.rev, user.id, user.rev),
    db
      .prepare(`UPDATE costumes SET ascension = ascension + 1, dupes = dupes - 1 WHERE user_id = ? AND costume_id = ? AND ascension = ? AND ${g.guard}`)
      .bind(user.id, c.costume_id, c.ascension, ...g.args),
  ]);
  return { spent: { gold }, profile: await profile(db, user.id) };
}

// ---- 戰鬥 ----

/** 發戰鬥票：用 Server 上的等級、突破、羈絆組隊伍，發種子 */
async function startBattle({ user, body, deps }) {
  const { db, engine } = deps;
  const index = STAGE_INDEX.get(body.stage);
  if (index === undefined) fail(400, 'bad_stage');
  if (index > 0) {
    const prev = await db.prepare('SELECT 1 FROM clears WHERE user_id = ? AND stage = ?').bind(user.id, STAGES[index - 1].id).first();
    if (!prev) fail(403, 'locked');
  }
  const ids = body.team;
  if (!Array.isArray(ids) || ids.length < 1 || ids.length > 4 || new Set(ids).size !== ids.length) fail(400, 'bad_team');
  const team = [];
  for (const id of ids) {
    const c = await ownedCostume(db, user.id, id);
    const ch = COSTUMES.get(id).character;
    const b = await db.prepare('SELECT exp FROM bonds WHERE user_id = ? AND character_id = ?').bind(user.id, ch).first();
    team.push({ costume: id, level: c.level, ascension: c.ascension, bond: bondLevel(b?.exp ?? 0) });
  }
  const setup = { stage: body.stage, team };
  try {
    engine.checkSetup(JSON.stringify(setup));
  } catch {
    fail(400, 'bad_team');
  }
  const ticket = hex(deps.randomBytes(16));
  const seed = hex(deps.randomBytes(8));
  const now = deps.now();
  const expires = now + TICKET_MINUTES * 60_000;
  await db
    .prepare('INSERT INTO tickets (id, user_id, stage, setup, seed, created_at, expires_at) VALUES (?, ?, ?, ?, ?, ?, ?)')
    .bind(ticket, user.id, body.stage, JSON.stringify(setup), seed, now, expires)
    .run();
  return { ticket, setup, seed, expires_at: expires };
}

/** 結算：重算 Client 送來的指令紀錄，贏了才發獎勵；一張票只能結算一次 */
async function finishBattle({ user, body, deps }) {
  const { db, engine } = deps;
  const t = await db.prepare('SELECT * FROM tickets WHERE id = ? AND user_id = ?').bind(String(body.ticket ?? ''), user.id).first();
  if (!t) fail(404, 'no_ticket');
  if (t.status !== 'open') fail(409, 'ticket_used');
  if (t.expires_at < deps.now()) fail(410, 'ticket_expired');

  let outcome;
  try {
    outcome = JSON.parse(engine.simulate(t.setup, t.seed, JSON.stringify(body.commands ?? [])));
  } catch {
    await db.prepare("UPDATE tickets SET status = 'rejected' WHERE id = ? AND status = 'open'").bind(t.id).run();
    fail(422, 'replay_failed');
  }

  const stage = STAGES[STAGE_INDEX.get(t.stage)];
  const first = outcome.won && !(await db.prepare('SELECT 1 FROM clears WHERE user_id = ? AND stage = ?').bind(user.id, t.stage).first());
  const rewards = outcome.won
    ? { gold: stage.rewards.gold, notes: stage.rewards.notes, stardust: first ? stage.rewards.first_stardust : 0, first }
    : { gold: 0, notes: 0, stardust: 0, first: false };

  const g = revision(deps, user);
  const statements = [
    // 票還是 open 才發（另一個請求剛把它標成 rejected 時，這裡就不生效）
    db
      .prepare(
        `UPDATE users SET gold = gold + ?, notes = notes + ?, stardust = stardust + ?, rev = ?
         WHERE id = ? AND rev = ? AND EXISTS (SELECT 1 FROM tickets WHERE id = ? AND status = 'open')`,
      )
      .bind(rewards.gold, rewards.notes, rewards.stardust, g.rev, user.id, user.rev, t.id),
    // 票的狀態也在同一個交易裡換掉：同一張票同時送兩次，只有一次會發獎勵
    db
      .prepare(`UPDATE tickets SET status = ?, digest = ? WHERE id = ? AND status = 'open' AND ${g.guard}`)
      .bind(outcome.won ? 'won' : 'lost', outcome.digest, t.id, ...g.args),
  ];
  if (outcome.won) {
    statements.push(
      db
        .prepare(
          `INSERT INTO clears (user_id, stage, first_at) SELECT ?, ?, ? WHERE ${g.guard}
           ON CONFLICT (user_id, stage) DO UPDATE SET wins = wins + 1`,
        )
        .bind(user.id, t.stage, deps.now(), ...g.args),
    );
  }
  const results = await commit(db, statements);
  if (results[1].meta.changes !== 1) fail(409, 'ticket_used');
  return {
    outcome,
    // Client 自己算的 digest 跟 Server 不一樣：代表兩邊版本不同或有 bug（結果仍以 Server 為準）
    desync: body.digest !== undefined && body.digest !== outcome.digest,
    rewards,
    profile: await profile(db, user.id),
  };
}
