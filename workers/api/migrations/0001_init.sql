-- 虛擬新傳說 Server 的資料表（Cloudflare D1，SQLite）。
-- 所有會改玩家狀態的請求都用 users.rev 當樂觀鎖：一個 batch（交易）裡先改 users 並換新的 rev，
-- 其他敘述用 EXISTS (… rev = 新的 rev) 當條件，同時進來的第二個請求就整批不生效。

CREATE TABLE users (
  id TEXT PRIMARY KEY,
  key_hash TEXT NOT NULL,            -- 裝置金鑰的 SHA-256（金鑰本身只在 Client）
  rev TEXT NOT NULL,
  created_at INTEGER NOT NULL,
  stardust INTEGER NOT NULL CHECK (stardust >= 0),   -- 星屑：轉蛋
  gold INTEGER NOT NULL CHECK (gold >= 0),           -- 金幣：升級、突破
  notes INTEGER NOT NULL CHECK (notes >= 0),         -- 練習筆記：升級素材
  pity INTEGER NOT NULL DEFAULT 0,                   -- 距離上一個 ★5 抽了幾次
  pulls INTEGER NOT NULL DEFAULT 0,                  -- 總抽數
  daily_day INTEGER NOT NULL DEFAULT 0,              -- 最後一次領每日登入獎勵是哪一天（台灣時間凌晨 4 點換日）
  transfer_hash TEXT,                                -- 繼承碼的 SHA-256
  transfer_expires INTEGER
);

CREATE TABLE costumes (
  user_id TEXT NOT NULL,
  costume_id TEXT NOT NULL,
  level INTEGER NOT NULL,
  ascension INTEGER NOT NULL DEFAULT 0,
  dupes INTEGER NOT NULL DEFAULT 0,  -- 重複抽到、還沒用來突破的次數
  obtained_at INTEGER NOT NULL,
  PRIMARY KEY (user_id, costume_id)
);

-- 角色的羈絆（同一位角色的所有版本共用）
CREATE TABLE bonds (
  user_id TEXT NOT NULL,
  character_id TEXT NOT NULL,
  exp INTEGER NOT NULL DEFAULT 0,
  PRIMARY KEY (user_id, character_id)
);

-- 抽卡紀錄（玩家可以查、出問題時對帳）
CREATE TABLE pulls (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  user_id TEXT NOT NULL,
  banner TEXT NOT NULL,
  costume_id TEXT NOT NULL,
  rarity INTEGER NOT NULL,
  created_at INTEGER NOT NULL
);
CREATE INDEX pulls_user ON pulls (user_id, id);

-- 戰鬥票：開打時發，結算時用 Rust 引擎重算
CREATE TABLE tickets (
  id TEXT PRIMARY KEY,
  user_id TEXT NOT NULL,
  stage TEXT NOT NULL,
  setup TEXT NOT NULL,               -- BattleSetup 的 JSON（開打當下的等級、突破、羈絆）
  seed TEXT NOT NULL,
  created_at INTEGER NOT NULL,
  expires_at INTEGER NOT NULL,
  status TEXT NOT NULL DEFAULT 'open',  -- open／won／lost／rejected
  digest TEXT
);
CREATE INDEX tickets_user ON tickets (user_id, created_at);

CREATE TABLE clears (
  user_id TEXT NOT NULL,
  stage TEXT NOT NULL,
  first_at INTEGER NOT NULL,
  wins INTEGER NOT NULL DEFAULT 1,
  PRIMARY KEY (user_id, stage)
);

-- M2 非同步 PvP 預留：防守隊伍與積分
CREATE TABLE pvp_defense (
  user_id TEXT PRIMARY KEY,
  team TEXT NOT NULL,
  rating INTEGER NOT NULL DEFAULT 1000,
  updated_at INTEGER NOT NULL
);
