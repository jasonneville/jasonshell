import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { fileURLToPath } from 'node:url';

const bridge = fileURLToPath(new URL('./main.ts', import.meta.url));
export default defineConfig({
  root: fileURLToPath(new URL('.', import.meta.url)),
  plugins: [svelte()],
  resolve: { alias: Object.fromEntries(['core', 'event', 'window'].map((name) => [`@tauri-apps/api/${name}`, bridge])) },
  server: { fs: { allow: [fileURLToPath(new URL('../../../', import.meta.url))] } },
});
