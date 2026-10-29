// 遊戲資料：跟 Rust 引擎編進去的是同一份 repo 的 data/*.json
import costumeList from '../../../data/costumes.json' with { type: 'json' };
import stageList from '../../../data/stages.json' with { type: 'json' };
import bannerList from '../../../data/banners.json' with { type: 'json' };

export const COSTUMES = new Map(costumeList.map((c) => [c.id, c]));
/** 關卡照 stages.json 的順序解鎖：要先打過前一關 */
export const STAGES = stageList;
export const STAGE_INDEX = new Map(stageList.map((s, i) => [s.id, i]));
export const BANNERS = new Map(bannerList.map((b) => [b.id, b]));

/** 新帳號送的角色與貨幣（四個 ★3 剛好一隊、十連抽一次） */
export const STARTER = {
  costumes: ['xiaocui.uniform', 'aming.uniform', 'tangtang.uniform', 'lanjie.uniform'],
  stardust: 1000,
  gold: 5000,
  notes: 100,
};

/** 每日登入獎勵 */
export const DAILY = { stardust: 100, gold: 1000 };

/** 重複抽到、已經滿突破時換成的星屑與羈絆經驗 */
export const OVERFLOW = {
  3: { stardust: 10, bond: 20 },
  4: { stardust: 50, bond: 50 },
  5: { stardust: 200, bond: 100 },
};

export const MAX_ASCENSION = 5;
export const BOND_PER_LEVEL = 100;
export const MAX_BOND = 10;
export const TICKET_MINUTES = 30;
export const TRANSFER_DAYS = 7;

export const bondLevel = (exp) => Math.min(MAX_BOND, Math.floor(exp / BOND_PER_LEVEL));

/** 從 lv 升到 lv+1 的花費 */
export const levelCost = (lv) => ({ gold: 50 * lv, notes: 5 * lv });
/** 突破到 asc+1 的金幣 */
export const ascendCost = (asc) => 1000 * (asc + 1);
