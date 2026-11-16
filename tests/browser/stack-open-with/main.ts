import '../../../src/app.css';
import { mount, unmount } from 'svelte';
import StackPopupSurface from '../../../src/components/StackPopupSurface.svelte';
import { child, ledger, overrides, resetBridge } from './bridge';

let popup: ReturnType<typeof mount> | undefined;
const pending: { path: unknown; resolve: (value: unknown) => void; reject: (error: Error) => void }[] = [];
const launches: { resolve: (value?: unknown) => void; reject: (error: Error) => void }[] = [];
const state = { deferLaunch: false, failLaunch: false };
const pointer: { x: number; y: number; trusted: boolean; expanded: string | null }[] = [];
const trigger = () => [...document.querySelectorAll<HTMLElement>('[role="menuitem"]')].find((e) => e.textContent?.includes('Open with'));
window.addEventListener('pointermove', (event) => pointer.push({ x: event.clientX, y: event.clientY, trusted: event.isTrusted, expanded: trigger()?.getAttribute('aria-expanded') ?? null }));
async function reset() {
  if (popup) await unmount(popup);
  resetBridge(); pending.length = 0; launches.length = 0; pointer.length = 0;
  state.deferLaunch = false; state.failLaunch = false;
  overrides.set('list_stack_open_with_candidates', ({ path }) => new Promise((resolve, reject) => pending.push({ path, resolve, reject })));
  for (const command of ['open_stack_item_with_app', 'open_stack_item_with_picker']) {
    overrides.set(command, () => {
      if (state.failLaunch) throw new Error('Fixture association launch failed');
      return state.deferLaunch ? new Promise((resolve, reject) => launches.push({ resolve, reject })) : undefined;
    });
  }
  popup = mount(StackPopupSurface, { target: document.querySelector('#surface')! });
}
const candidate = (id: string) => ({ id, label: `Associated ${id}`, source: 'windows-association' });
Object.assign(window, { openWithFixture: { reset, pending, launches, state, ledger, pointer, child, candidate } });
await reset();
