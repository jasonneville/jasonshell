import '../../../src/app.css';
import { mount, tick } from 'svelte';
import BottomBar from '../../../src/components/BottomBar.svelte';
import { ledger, rows, settings, publish, control } from './edge-bridge';
document.documentElement.dataset.theme = new URLSearchParams(location.search).get('theme') === 'light' ? 'base-light' : 'base-dark';
document.getElementById('host')!.style.height = `${settings.ui.bottomBarHeightLogical}px`;
mount(BottomBar, { target: document.getElementById('host')! });
const rect = (element: Element) => {
  const r = element.getBoundingClientRect();
  return { left: r.left, right: r.right, top: r.top, bottom: r.bottom, width: r.width, height: r.height };
};
const selectors = { quick: '.quick-launch-button', direct: '.task-group-direct .task-button', group: '.task-capsule', process: '.process-manager-button' };
Object.assign(window, { edgeFixture: { ledger, rows, settings, publish, control, selectors,
  clear: () => { ledger.length = 0; },
  measure: () => Object.fromEntries(Object.entries(selectors).map(([name, selector]) => {
    const button = document.querySelector<HTMLButtonElement>(selector)!;
    return [name, { face: rect(button), children: [...button.children].map(rect), disabled: button.disabled,
      edgeHit: document.elementFromPoint((rect(button).left + rect(button).right) / 2, innerHeight - 1)?.closest('button') === button }];
  })),
  snapshot: async (windows = rows) => { await publish('taskbar:windows-snapshot', { sequence: Date.now(), windows }); await tick(); }
} });
