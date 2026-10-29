// 跟 Server（workers/api）溝通。抽卡、貨幣、庫存、獎勵都由 Server 決定，這裡只送「想做什麼」、收結果。
// 帳號 id 與裝置金鑰存在這台裝置（localStorage `vl_account`）；換裝置用繼承碼。
// Server 網址：VITE_API_SERVER（.env.local），預設是本機的 `npm run dev:server`。
import { t } from './i18n.svelte.js';

const SERVER = (import.meta.env.VITE_API_SERVER || 'http://127.0.0.1:8790').replace(/\/$/, '');
const ACCOUNT_KEY = 'vl_account';

function loadAccount() {
  try {
    const a = JSON.parse(localStorage.getItem(ACCOUNT_KEY) ?? 'null');
    return a?.id && a?.key ? a : null;
  } catch {
    return null;
  }
}

function saveAccount(a) {
  try {
    if (a) localStorage.setItem(ACCOUNT_KEY, JSON.stringify({ id: a.id, key: a.key }));
    else localStorage.removeItem(ACCOUNT_KEY);
  } catch {
    /* 存不了就只在這次開著時有效 */
  }
}

export const session = $state({
  account: loadAccount(),
  profile: null,
  banners: [],
  /** 剛領到的每日登入獎勵（顯示一次） */
  daily: null,
  busy: false,
  error: '',
});

const ERRORS = {
  unauthorized: '帳號驗證失敗，請重新開始或輸入繼承碼',
  not_enough_stardust: '星屑不夠',
  not_enough: '金幣或練習筆記不夠',
  level_cap: '已經到等級上限，要先突破',
  no_dupes: '要再抽到同一個角色才能突破',
  level_not_capped: '要先練到等級上限才能突破',
  max_ascension: '已經滿突破了',
  locked: '要先打過前一關',
  bad_team: '隊伍不合法（同一位角色只能上一個）',
  ticket_used: '這場戰鬥已經結算過了',
  ticket_expired: '戰鬥超過 30 分鐘，已經失效',
  replay_failed: '戰鬥紀錄驗證失敗，沒有發獎勵',
  bad_code: '繼承碼不對或已經過期',
  conflict: '同時有別的操作，請再試一次',
  network: '連不上伺服器',
};

export const errorText = (code) => t(ERRORS[code] ?? '發生錯誤（{code}）', { code });

async function call(method, path, body) {
  const headers = { 'content-type': 'application/json' };
  if (session.account) headers.authorization = `Bearer ${session.account.id}.${session.account.key}`;
  let res;
  try {
    res = await fetch(SERVER + path, { method, headers, body: body === undefined ? undefined : JSON.stringify(body) });
  } catch {
    throw new ApiError('network');
  }
  const data = await res.json().catch(() => ({}));
  if (!res.ok) throw new ApiError(data.error ?? `http_${res.status}`);
  return data;
}

export class ApiError extends Error {
  constructor(code) {
    super(code);
    this.code = code;
  }
}

/** 包一層：忙碌中、錯誤訊息都放進 session，畫面直接顯示 */
async function run(fn) {
  session.busy = true;
  session.error = '';
  try {
    return await fn();
  } catch (e) {
    session.error = errorText(e.code ?? 'internal');
    throw e;
  } finally {
    session.busy = false;
  }
}

function useAccount(data) {
  session.account = { id: data.id, key: data.key };
  saveAccount(session.account);
  session.profile = data.profile;
}

export const signUp = () => run(async () => useAccount(await call('POST', '/v1/account')));

export const claim = (code) => run(async () => useAccount(await call('POST', '/v1/account/claim', { code })));

export function signOut() {
  session.account = null;
  session.profile = null;
  saveAccount(null);
}

export const refresh = () =>
  run(async () => {
    const [me, b] = await Promise.all([call('GET', '/v1/me'), call('GET', '/v1/banners')]);
    session.profile = me.profile;
    session.banners = b.banners;
    if (me.daily) session.daily = me.daily;
  });

export const issueTransfer = () => run(() => call('POST', '/v1/account/transfer', {}));

export const gacha = (banner, count) =>
  run(async () => {
    const r = await call('POST', '/v1/gacha', { banner, count });
    session.profile = r.profile;
    return r.results;
  });

export const levelUp = (costume, levels) =>
  run(async () => {
    session.profile = (await call('POST', '/v1/costumes/level', { costume, levels })).profile;
  });

export const ascend = (costume) =>
  run(async () => {
    session.profile = (await call('POST', '/v1/costumes/ascend', { costume })).profile;
  });

export const startBattle = (stage, team) => run(() => call('POST', '/v1/battle/start', { stage, team }));

export const finishBattle = (ticket, commands, digest) =>
  run(async () => {
    const r = await call('POST', '/v1/battle/finish', { ticket, commands, digest });
    session.profile = r.profile;
    return r;
  });
