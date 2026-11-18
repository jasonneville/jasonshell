import { defineConfig } from 'vite';
import { fileURLToPath } from 'node:url';
import { svelte } from '@sveltejs/vite-plugin-svelte';

export default defineConfig({
  plugins: [{
    name: 'close-parity-stack-xterm-options',
    enforce: 'pre',
    resolveId(source, importer) {
      if (source === '@xterm/xterm' && importer?.endsWith('/src/components/StackTerminalPane.svelte')) {
        return this.resolve(fileURLToPath(new URL('./stack-xterm.ts', import.meta.url)), importer, { skipSelf: true });
      }
    }
  }, svelte()],
  cacheDir: 'node_modules/.vite-close-parity',
  optimizeDeps: { entries: ['tests/browser/close-parity/index.html'] },
  server: { host: '127.0.0.1', port: 5196, strictPort: true, hmr: false }
});
