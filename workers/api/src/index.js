// Cloudflare Workers 的入口：載入 Rust 引擎的 WebAssembly（npm run build:wasm 產生），接上 D1
import wasm from '../../../pkg/engine/legends_wasm_bg.wasm';
import { initSync, simulate, checkSetup, levelCap } from '../../../pkg/engine/legends_wasm.js';
import { handle } from './app.js';

initSync({ module: wasm });

const engine = { simulate, checkSetup, levelCap };

export default {
  fetch(request, env) {
    return handle(request, {
      db: env.DB,
      engine,
      now: () => Date.now(),
      randomBytes: (n) => crypto.getRandomValues(new Uint8Array(n)),
    });
  },
};
