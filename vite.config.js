import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';

const host = process.env.TAURI_DEV_HOST;

export default defineConfig({
  plugins: [svelte()],
  // 網頁版放在子路徑（例如 GitHub Pages）時用 PAGES_BASE 指定；App 用根路徑
  base: process.env.PAGES_BASE || '/',
  clearScreen: false,
  server: {
    // 1420～1424 是其他五款在用的，錯開才能同時開
    port: 1425,
    strictPort: true,
    host: host || false,
    watch: { ignored: ['**/src-tauri/**', '**/workers/**', '**/crates/**'] },
  },
});
