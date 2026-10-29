# 虛擬新傳說（Virtual Legends）

[虛擬台麻館](https://github.com/YuutaTsubasa/TaiwanMahjong)系列（Virtual 系列）的第六款：原創的回合制轉蛋 RPG。轉蛋抽到的是系列角色的各種版本（例：小翠・夏日祭典），用速度決定出手順序的「行動條」回合制戰鬥闖關（推條、接棒、反擊、連攜、追擊、五屬性相剋）。

**開發中（M1）。** 企劃見 [`docs/plan.md`](docs/plan.md)。

## 內容

| 路徑 | 是什麼 |
| --- | --- |
| `src/` | 遊戲畫面（Svelte 5＋Vite 6）：標題、大廳、招募、角色養成、出擊編隊、戰鬥。戰鬥在裝置上用 wasm 引擎跑，打完把指令紀錄送給 Server |
| `public/characters/` | 系列角色的半身像（90 年代版，沿用其他五款） |
| `crates/legends-core/` | Rust 戰鬥引擎：行動條回合制（Scheduler／Resolver／Effect／Trigger／Resource／State）、敵人 AI、自動戰鬥、Server 用的重算驗證。確定性：同樣的隊伍、關卡、種子、指令，一定得到同樣的結果 |
| `crates/legends-wasm/` | 引擎的 WebAssembly 介面（`BattleSession`、`simulate`），`npm run build:wasm` 產生到 `pkg/engine/`，Client 與 Server 共用 |
| `workers/api/` | Server（Cloudflare Workers＋D1）：帳號、轉蛋、庫存、養成、戰鬥票與重算驗證，見下面「Server」 |
| `data/` | 角色、技能、敵人、關卡、卡池的資料（JSON）。Rust 引擎與 Server 讀同一份 |

## 開發

```bash
npm ci
npm run dev:server   # 本機 Server（wrangler dev＋本機 D1），http://127.0.0.1:8790
npm run dev          # 遊戲畫面，http://localhost:1425（會先 build:wasm）
```

Server 網址用 `VITE_API_SERVER`（寫在 `.env.local`）換，預設是本機的 8790。

## 測試

```bash
npm ci
npm test            # 引擎（cargo test）＋ Server（wasm 引擎＋node:sqlite 模擬 D1）
cargo test --release --manifest-path crates/legends-core/Cargo.toml
# 平衡表（各等級打各關的勝率與回合數）、重算一場的時間
cargo test --release --manifest-path crates/legends-core/Cargo.toml -- --ignored --nocapture
```

- 第一版不收真錢，只用遊戲內貨幣轉蛋，遊戲內公開每張卡的機率。
- 純 PvE 起步，之後加非同步 PvP（打別人存的防守隊伍）。
- 技術沿用系列：Tauri v2＋Svelte 5＋Vite 6、Rust 戰鬥引擎（WebAssembly）、[VirtualCore](https://github.com/YuutaTsubasa/VirtualCore) 的 FM 音源／多國語言／手把／CI；Server 用 Cloudflare Workers＋D1，負責帳號、轉蛋、庫存與戰鬥結算驗證。

## Server

Server 是權威：抽卡、貨幣、庫存、獎勵都由它決定，Client 只送「我想做什麼」。驗證用 `Authorization: Bearer <帳號 id>.<裝置金鑰>`（訪客帳號，換裝置用繼承碼）。

| API | 做什麼 |
| --- | --- |
| `POST /v1/account` | 開訪客帳號（送四套 ★3、1000 星屑），回傳 id 與裝置金鑰 |
| `POST /v1/account/transfer`／`claim` | 發繼承碼（12 字、7 天）／用繼承碼在新裝置取回帳號（舊金鑰失效） |
| `GET /v1/me` | 個人資料；每天第一次讀時發登入獎勵（台灣時間凌晨 4 點換日） |
| `GET /v1/banners` | 卡池與**每張卡的公開機率**、保底 |
| `POST /v1/gacha` | 單抽／十連（第 80 抽保底 ★5、十連至少一張 ★4；重複的變突破次數，滿了換星屑與羈絆） |
| `POST /v1/costumes/level`／`ascend` | 升級（金幣＋練習筆記，加羈絆）／突破（要練滿＋重複抽到的次數） |
| `POST /v1/battle/start` | 發戰鬥票：用 Server 上的等級組隊伍、發種子（30 分鐘有效） |
| `POST /v1/battle/finish` | 送指令紀錄；Server 用同一份引擎重算，贏了才發獎勵，一張票只能結算一次 |

同時進來的請求用 `users.rev` 樂觀鎖擋（見 `migrations/0001_init.sql` 開頭），第二個回 409。

本機跑：`npm run build:wasm`，再到 `workers/api`：`npm install`、`npm run migrate:local`、`npm run dev`（http://127.0.0.1:8790，用 wrangler 內建的本機 D1）。部署步驟寫在 `workers/api/wrangler.toml` 開頭，要先建 D1 資料庫。
