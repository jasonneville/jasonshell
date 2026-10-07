import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { fileURLToPath } from 'node:url';
const root = fileURLToPath(new URL('.', import.meta.url));
const bridge = fileURLToPath(new URL('./bridge.ts', import.meta.url));
export default defineConfig({
  root, plugins: [svelte()],
  resolve: { alias: Object.fromEntries(['core', 'event', 'window'].map((name) => [`@tauri-apps/api/${name}`, bridge])) },
  server: { host: '127.0.0.1', port: 4179, strictPort: true, fs: { allow: [fileURLToPath(new URL('../../../', import.meta.url))] } },
  build: { outDir: fileURLToPath(new URL('../../../node_modules/.cache/task-gallery-browser', import.meta.url)), emptyOutDir: true }
});
