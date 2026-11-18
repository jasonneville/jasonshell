import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';

export default defineConfig({
  plugins: [svelte()],
  cacheDir: 'node_modules/.vite-ui-audit-panels',
  optimizeDeps: { entries: ['tests/browser/ui-audit-panels/index.html'] },
  server: { host: '127.0.0.1', port: 5397, strictPort: true, hmr: false }
});
