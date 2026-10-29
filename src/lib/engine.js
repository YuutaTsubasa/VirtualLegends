// 戰鬥引擎（Rust 編成的 WebAssembly，`npm run build:wasm` 產生），跟 Server 重算用的是同一份
import init, { BattleSession } from '../../pkg/engine/legends_wasm.js';

let ready;
export const loadEngine = () => (ready ??= init());

export { BattleSession };
