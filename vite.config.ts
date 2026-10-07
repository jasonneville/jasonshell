import { defineConfig, type PluginOption } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import Terminal from 'vite-plugin-terminal';

const host = process.env.TAURI_DEV_HOST;

export default defineConfig(({ command }) => {
  const plugins: PluginOption[] = [svelte()];

  if (command === 'serve') {
    plugins.push(
      Terminal({
        console: 'terminal',
        output: ['terminal', 'console']
      })
    );
  }

  return {
    clearScreen: false,
    plugins,
    build: {
      rollupOptions: {
        output: {
          manualChunks(id) {
            // Keep the shared editor runtime out of the Stack surface chunk.
            // XML is already required statically for atomic closing-tag edits.
            const modulePath = id.replace(/\\/g, '/');
            if (/\/node_modules\/(?:@codemirror\/(?:commands|language|lang-xml|search|state|view)|@lezer\/(?:common|highlight|lr))\//.test(modulePath)) {
              return 'stack-editor-core';
            }
          }
        }
      }
    },
    server: {
      host: host || false,
      hmr: host
        ? {
            host,
            port: 1421,
            protocol: 'ws'
          }
        : {
            port: 1421
          },
      port: 1420,
      strictPort: true,
      watch: {
        // Avoid traversing Rust build output during dev-server startup.
        ignored: ['**/src-tauri/target', '**/src-tauri/target/**']
      }
    }
  };
});


