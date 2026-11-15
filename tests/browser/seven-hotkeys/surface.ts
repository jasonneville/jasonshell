import { mount } from 'svelte';
import '../../../src/app.css';
const label = new URLSearchParams(location.search).get('surface');
const loaders: Record<string, () => Promise<any>> = {
  'top-bar': () => import('../../../src/components/TopBar.svelte'),
  'bottom-bar': () => import('../../../src/components/BottomBar.svelte'),
  'settings-panel': () => import('../../../src/components/SettingsPanelSurface.svelte'),
  'process-manager': () => import('../../../src/components/ProcessManagerSurface.svelte')
};
const component = await loaders[label!]();
mount(component.default, { target: document.querySelector('#surface')! });
