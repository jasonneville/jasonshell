import { defineConfig } from 'vitest/config';
import { svelte } from '@sveltejs/vite-plugin-svelte';

// Observation only: production functions/branches remain untouched. This export
// exists solely in test compilation, never in a shipped component or bundle.
const probes: Record<string, string> = {
  'ProcessManagerSurface.svelte': `({ rows: processes.length, payloadCharacters: processes.reduce((n, row) => n + JSON.stringify(row).length, 0), taskbarPids: taskbarProcessPids.length, armedKillPid, killingPid, expandedGroups: Object.keys(processGroupExpansionState).length, isOpen })`,
  'TerminalPanelSurface.svelte': `({ titleEntries: terminalTitleStates.size, titleOutputCharacters: [...terminalTitleStates.values()].reduce((n, state) => n + (state.recentOutputText?.length ?? 0), 0), runtimes: paneRuntimes.size })`,
  'StackPopupSurface.svelte': `({ entries: iconCache.size, dataUrlCharacters: typeof iconCache.values === 'function' ? [...iconCache.values()].reduce((n, value) => n + (value?.length ?? 0), 0) : null, estimatedStringBytes: typeof iconCache.retainedBytes === 'number' ? iconCache.retainedBytes : [...iconCache.entries()].reduce((n, [key, value]) => n + 2 * (key.length + (value?.length ?? 0)), 0), negativeEntries: typeof iconCache.values === 'function' ? [...iconCache.values()].filter(value => value === null).length : null, currentPath: stackState.currentPath, visibleIcons: stackState.entries.filter(entry => Boolean(entry.iconDataUrl)).length })`
};

export default defineConfig({
  plugins: [{ name: 'ram-test-only-observer', enforce: 'pre', transform(source, id) {
    const name = id.replaceAll('\\', '/').split('/').at(-1);
    if (!name || !probes[name]) return;
    return source.replace('</script>', `\n export function __ramProbe() { return ${probes[name]}; }\n</script>`);
  } }, svelte()],
  resolve: { conditions: ['browser'] },
  test: { environment: 'jsdom', include: ['tests/runtime-ram/**/*.test.ts'], setupFiles: ['tests/components/setup.ts'], restoreMocks: true }
});
