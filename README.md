# 虛擬新傳說（Virtual Legends）

[虛擬台麻館](https://github.com/YuutaTsubasa/TaiwanMahjong)系列（Virtual 系列）的第六款：原創的回合制轉蛋 RPG。轉蛋抽到的是系列角色的各種衣裝，用以麻將牌為資源的「牌氣」戰鬥系統闖關。

**目前只有企劃，還沒有遊戲程式。** 企劃見 [`docs/plan.md`](docs/plan.md)。

- 第一版不收真錢，只用遊戲內貨幣轉蛋，遊戲內公開每張卡的機率。
- 純 PvE 起步，之後加非同步 PvP（打別人存的防守隊伍）。
- 技術沿用系列：Tauri v2＋Svelte 5＋Vite 6、Rust 戰鬥引擎（WebAssembly）、[VirtualCore](https://github.com/YuutaTsubasa/VirtualCore) 的 FM 音源／多國語言／手把／CI；Server 用 Cloudflare Workers＋D1，負責帳號、轉蛋、庫存與戰鬥結算驗證。
