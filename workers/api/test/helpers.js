// 測試用的 Server：記憶體 D1＋真的 wasm 引擎＋可以快轉的時間、可以指定的亂數
import { readFileSync } from 'node:fs';
import { randomBytes as nodeRandom } from 'node:crypto';
import { initSync, simulate, checkSetup, levelCap, BattleSession } from '../../../pkg/engine/legends_wasm.js';
import { handle } from '../src/app.js';
import { memoryD1 } from './d1.js';

initSync({ module: readFileSync(new URL('../../../pkg/engine/legends_wasm_bg.wasm', import.meta.url)) });

export { BattleSession };

export function server() {
  const s = {
    db: memoryD1(),
    clock: Date.UTC(2026, 9, 30, 12),
    /** 設成函式就用它產生亂數（測保底用） */
    random: null,
  };
  const deps = {
    db: s.db,
    engine: { simulate, checkSetup, levelCap },
    now: () => s.clock,
    randomBytes: (n) => (s.random ? s.random(n) : new Uint8Array(nodeRandom(n))),
  };
  s.call = async (method, path, body, auth) => {
    const headers = { 'content-type': 'application/json' };
    if (auth) headers.authorization = `Bearer ${auth.id}.${auth.key}`;
    const res = await handle(
      new Request(`https://api.test${path}`, { method, headers, body: body === undefined ? undefined : JSON.stringify(body) }),
      deps,
    );
    return { status: res.status, body: await res.json() };
  };
  s.newAccount = async () => (await s.call('POST', '/v1/account')).body;
  return s;
}

/** 用 Client 那份 wasm 把戰鬥票自動打完，回傳指令紀錄與 digest */
export function playAuto(ticket) {
  const b = new BattleSession(JSON.stringify(ticket.setup), ticket.seed);
  for (let cmd = b.auto(); cmd !== undefined; cmd = b.auto()) b.apply(cmd);
  const state = JSON.parse(b.state());
  const out = { commands: JSON.parse(b.log()), digest: b.digest(), phase: state.phase };
  b.free();
  return out;
}
