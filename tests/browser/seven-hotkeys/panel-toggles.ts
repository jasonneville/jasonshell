// Separate focused journey: historical seven-action results remain historical.
import type { NativeFixture } from './bridge';
const host = window as Window & { __sevenNative?: NativeFixture; __toggleEvidence?: any; __runPanelToggles?: () => Promise<any> };
const doc = (label: string) => document.querySelector<HTMLIFrameElement>(`#${label}`)!.contentDocument!;
const button = (label: string, name: string) => {
  const node = Array.from(doc(label).querySelectorAll<HTMLButtonElement>('button')).find(node => (node.getAttribute('aria-label') ?? node.textContent?.trim()) === name);
  if (!node) throw new Error(`Missing ${label} button ${name}`);
  return node;
};
const assert = (value: unknown, message: string) => { if (!value) throw new Error(message); };
async function until(check: () => unknown, message: string) {
  const end = performance.now() + 1800;
  while (!check()) {
    if (performance.now() > end) throw new Error(message);
    await new Promise(resolve => requestAnimationFrame(resolve));
  }
}
const settle = () => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve)));
host.__runPanelToggles = async () => {
  const n = host.__sevenNative!;
  const result = host.__toggleEvidence = { status: 'RUNNING', cases: [] as any[], trace: n.trace };
  const run = async (name: string, body: () => Promise<void>) => {
    try { await body(); result.cases.push({ name, status: 'PASS' }); }
    catch (error) { result.cases.push({ name, status: 'FAIL', error: String(error) }); }
  };
  await until(() => n.listeners.size > 15 && !!doc('process-manager').querySelector('[role="tab"]'), 'Surfaces not ready');
  for (const action of ['settings', 'processManager'] as const) {
    const label = action === 'settings' ? 'settings-panel' : 'process-manager';
    const origin = action === 'settings' ? 'top-bar' : 'bottom-bar';
    const launcher = action === 'settings' ? 'Open JasonShell settings' : 'Open process manager';
    for (const input of ['hotkey', 'launcher'] as const) {
      await run(`${action} ${input}: open → hide → reopen`, async () => {
        // Reset only test-native visibility; real production components remain mounted.
        n.visible[label] = false;
        const activate = () => input === 'hotkey' ? n.hotkey(action) : button(origin, launcher).click();
        const main = doc(label).querySelector('main');
        activate(); await until(() => n.visible[label], `${action} did not open`);
        let theme: HTMLInputElement | null = null;
        if (action === 'settings') {
          theme = doc(label).querySelector('input[maxlength="80"]');
          if (!theme) { button(label, 'Create copy').click(); await until(() => !!doc(label).querySelector('input[maxlength="80"]'), 'No theme editor'); theme = doc(label).querySelector('input[maxlength="80"]'); }
          theme!.value = 'Retained toggle draft';
          theme!.dispatchEvent(new (doc(label).defaultView!.Event)('input', { bubbles: true }));
        } else {
          button(label, 'System Info').click();
          await until(() => n.context.activeTab === 'system_info', 'System Info not selected');
        }
        const generation = n.context.openGeneration;
        activate(); await until(() => !n.visible[label], `${action} second ${input} must hide, not open/focus`);
        if (action === 'processManager') assert(n.context.openGeneration === generation, 'Hide must not create a new Processes generation');
        activate(); await until(() => n.visible[label], `${action} third ${input} must reopen`);
        if (action === 'settings') assert(main === doc(label).querySelector('main') && theme!.isConnected && theme!.value === 'Retained toggle draft', 'Toggle discarded Settings mount/theme draft');
        else await until(() => n.context.openGeneration > generation && n.context.activeTab === 'processes' && doc(label).querySelector('[role="tab"][aria-selected="true"]')?.textContent?.trim() === 'Processes', 'Real reopen must reset PM to Processes');
      });
    }
  }
  for (const action of ['settings', 'processManager'] as const) {
    await run(`${action}: external blur dismissal then hotkey reopens`, async () => {
      const label = action === 'settings' ? 'settings-panel' : 'process-manager';
      n.dismiss(label); await settle();
      n.hotkey(action); await until(() => n.visible[label], `${action} stayed closed after external dismissal`);
    });
  }
  await run('explicit show wrappers never toggle visible panels and retain PM target PID', async () => {
    const explicit = (label: string, request: object) => (doc(label).defaultView as any).__explicitPanelShow(request);
    const request = { anchorLeft: 0, anchorWidth: 30 };
    await explicit('top-bar', request); await explicit('top-bar', request);
    assert(n.visible['settings-panel'], 'Explicit Settings show toggled closed');
    await explicit('bottom-bar', { ...request, focusPid: 101 });
    await explicit('bottom-bar', { ...request, focusPid: 101 });
    assert(n.visible['process-manager'] && n.context.focusPid === 101, 'Explicit PM show lost target or toggled closed');
    await until(() => !!doc('process-manager').querySelector('input') && Array.from(doc('process-manager').querySelectorAll('input')).some(input => input.value === '101'), 'PM renderer did not apply targeted process filter');
  });
  await run('wrong-target hotkey events are inert even when panels are visible', async () => {
    const before = n.trace.filter(e => e.type === 'invoke').length;
    n.emit('process-manager', 'settings:open-requested');
    n.emit('settings-panel', 'process-manager:open-requested');
    await settle();
    const commands = n.trace.filter(e => e.type === 'invoke').slice(before).map(e => e.command);
    assert(!commands.some(c => /(?:show|hide|toggle)_(?:settings_panel|process_manager)/.test(c)), `Wrong-target panel action: ${commands}`);
  });
  for (const action of ['settings', 'processManager'] as const) {
    const label = action === 'settings' ? 'settings-panel' : 'process-manager';
    const origin = action === 'settings' ? 'top-bar' : 'bottom-bar';
    const name = action === 'settings' ? 'Open JasonShell settings' : 'Open process manager';
    const command = action === 'settings' ? 'show_settings_panel' : 'show_process_manager';
    const calls = () => n.trace.filter(e => e.type === 'invoke' && e.command === command).length;
    const pointer = (type: string, overrides: PointerEventInit = {}) => {
      const event = new (doc(origin).defaultView!.PointerEvent)(type, { bubbles: true, cancelable: true, pointerType: 'mouse', pointerId: 1, button: 0, buttons: type === 'pointerdown' ? 1 : 0, isPrimary: true, ...overrides });
      button(origin, name).dispatchEvent(event); return event;
    };
    const click = (detail: number, mouseButton = 0) => button(origin, name).dispatchEvent(new (doc(origin).defaultView!.MouseEvent)('click', { bubbles: true, cancelable: true, detail, button: mouseButton }));
    await run(`${action} pointerdown → hold >150ms → pointerup/click toggles exactly once`, async () => {
      n.visible[label] = true;
      const before = calls();
      let escaped = false;
      const listener = () => { escaped = true; };
      doc(origin).addEventListener('pointerdown', listener);
      const event = pointer('pointerdown');
      doc(origin).removeEventListener('pointerdown', listener);
      assert(event.defaultPrevented && !escaped, 'Primary pointerdown must prevent focus default and stop bubbling');
      await until(() => calls() === before + 1 && !n.visible[label], 'Primary pointerdown must toggle immediately');
      const start = performance.now();
      await until(() => performance.now() - start > 220, 'Long hold did not elapse');
      pointer('pointerup'); click(1); await settle();
      assert(calls() === before + 1 && !n.visible[label], 'Compatibility click toggled again after long hold');
    });
    await run(`${action} pointercancel cannot suppress subsequent keyboard/AT activation`, async () => {
      n.visible[label] = false;
      const before = calls();
      pointer('pointerdown'); await until(() => calls() === before + 1 && n.visible[label], 'Pointerdown did not open before cancellation');
      pointer('pointercancel'); click(0);
      await until(() => calls() === before + 2 && !n.visible[label], 'Cancelled pointer blocked detail-zero keyboard/AT click');
      click(0); await until(() => calls() === before + 3 && n.visible[label], 'Subsequent keyboard/AT click failed to reopen');
    });
    await run(`${action} secondary button and non-primary pointer are inert`, async () => {
      const before = calls(); const visible = n.visible[label];
      pointer('pointerdown', { button: 2, buttons: 2 }); pointer('pointerup', { button: 2 }); click(1, 2);
      pointer('pointerdown', { isPrimary: false, pointerType: 'touch', pointerId: 2 }); pointer('pointercancel', { isPrimary: false, pointerId: 2 });
      await settle();
      assert(calls() === before && n.visible[label] === visible, 'Secondary/non-primary pointer toggled panel');
    });
  }
  await run('Quick Commands repeated clicks close then reopen', async () => {
    const control = () => button('top-bar', 'Open quick commands');
    const calls = (command: string) => n.trace.filter(e => e.type === 'invoke' && e.command === command).length;
    control().click(); await until(() => calls('show_command_panel') === 1, 'Quick Commands did not open');
    control().click(); await until(() => calls('hide_command_panel') >= 1, 'Quick Commands did not close');
    control().click(); await until(() => calls('show_command_panel') === 2, 'Quick Commands did not reopen');
    assert(n.trace.filter(e => e.type === 'invoke' && e.command === 'show_command_panel').length === 2, 'Quick Commands repeated click opened incorrectly');
    assert(n.trace.filter(e => e.type === 'invoke' && e.command === 'hide_command_panel').length >= 1, 'Quick Commands never hid');
  });
  await run('Quick Commands native closed notification allows launcher reopen', async () => {
    const before = n.trace.filter(e => e.type === 'invoke' && e.command === 'show_command_panel').length;
    n.emit('top-bar', 'command-panel:closed'); await settle();
    button('top-bar', 'Open quick commands').click();
    await until(() => n.trace.filter(e => e.type === 'invoke' && e.command === 'show_command_panel').length === before + 1, 'Native close left stale Quick Commands state');
  });
  await run('production surfaces finish startup without uncaught errors or unexpected IPC', async () => {
    await settle();
    const failures = n.trace.filter(e => e.type === 'startup-error' || e.type === 'unexpected');
    assert(failures.length === 0, `Fixture startup/IPC errors: ${JSON.stringify(failures)}`);
  });
  result.status = result.cases.every(test => test.status === 'PASS') ? 'PASS' : 'FAIL';
  document.querySelector('#result')!.textContent = JSON.stringify(result.cases);
  return { status: result.status, cases: result.cases };
};
