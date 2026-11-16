import { mount } from 'svelte';
import '../../../src/app.css';
// Install before mounting so startup exceptions cannot silently invalidate handlers.
const startupError = (message: string) => {
  const host = parent as Window & { __sevenNative?: { record: (type: string, data: Record<string, unknown>) => void } };
  host.__sevenNative?.record('startup-error', { surface: new URLSearchParams(location.search).get('surface'), message });
};
window.addEventListener('error', event => startupError(event.message));
window.addEventListener('unhandledrejection', event => startupError(String(event.reason)));
const label = new URLSearchParams(location.search).get('surface');
const loaders: Record<string, () => Promise<any>> = {
  'top-bar': () => import('../../../src/components/TopBar.svelte'),
  'bottom-bar': () => import('../../../src/components/BottomBar.svelte'),
  'settings-panel': () => import('../../../src/components/SettingsPanelSurface.svelte'),
  'process-manager': () => import('../../../src/components/ProcessManagerSurface.svelte')
};
const component = await loaders[label!]();
mount(component.default, { target: document.querySelector('#surface')! });
// Test-only entry executes the production explicit-show IPC wrappers.
if (label === 'bottom-bar') (window as any).__explicitPanelShow = (request: any) => import('../../../src/lib/processManager').then(api => api.showProcessManager(request));
if (label === 'top-bar') (window as any).__explicitPanelShow = (request: any) => import('../../../src/lib/settingsPanel').then(api => api.showSettingsPanel(request));
