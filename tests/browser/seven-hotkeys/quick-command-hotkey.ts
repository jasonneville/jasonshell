import type { NativeFixture } from './bridge';
const host = window as Window & { __sevenNative?: NativeFixture; __runEight?: () => Promise<any>; __eightEvidence?: any };
const doc = (label: string) => document.querySelector<HTMLIFrameElement>(`#${label}`)!.contentDocument!;
const button = (label: string, name: string | RegExp) => {
  const node = Array.from(doc(label).querySelectorAll<HTMLButtonElement>('button')).find(node => {
    const text = node.getAttribute('aria-label') ?? node.textContent?.trim() ?? '';
    return typeof name === 'string' ? text === name : name.test(text);
  });
  if (!node) throw new Error(`Missing ${label} button ${name}`); return node;
};
const assert = (value: unknown, message: string) => { if (!value) throw new Error(message); };
const settle = () => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve)));
async function until(check: () => unknown, message: string) {
  const end = performance.now() + 1800;
  while (!check()) { if (performance.now() > end) throw new Error(message); await new Promise(resolve => requestAnimationFrame(resolve)); }
}
host.__runEight = async () => {
  const n = host.__sevenNative!;
  const evidence = host.__eightEvidence = { status: 'RUNNING', cases: [] as any[], trace: n.trace };
  const run = async (name: string, body: () => Promise<void>) => {
    try { await body(); evidence.cases.push({ name, status: 'PASS' }); }
    catch (error) { evidence.cases.push({ name, status: 'FAIL', error: String(error) }); }
  };
  const calls = (command: string) => n.trace.filter(e => e.type === 'invoke' && e.command === command).length;
  const shortcutRegion = () => doc('settings-panel').querySelector('[aria-labelledby="hotkeys-heading"]')!;
  const draft = (name: RegExp) => {
    const node = Array.from(shortcutRegion().querySelectorAll<HTMLButtonElement>('button')).find(node => name.test(node.getAttribute('aria-label') ?? node.textContent?.trim() ?? ''));
    if (!node) throw new Error(`Missing draft control ${name}`); return node;
  };
  await until(() => n.listeners.size > 15 && !!doc('settings-panel').querySelector('[aria-label="Capture Search shortcut"]'), 'Surfaces not ready');
  await run('eighth Settings row captures, cancels, saves and stages Reset without changing committed seven', async () => {
    n.hotkey('settings'); await until(() => n.visible['settings-panel'], 'Settings failed to open');
    const qc = button('settings-panel', 'Capture Quick Commands shortcut');
    assert(qc.textContent?.includes('Alt+Q'), 'Quick Commands default must be Alt+Q');
    const original = structuredClone(n.settings.hotkeys);
    const captureLocal = async () => {
      qc.focus(); qc.click(); await until(() => !!n.lease, 'No capture ACK'); await settle();
      qc.dispatchEvent(new (doc('settings-panel').defaultView!.KeyboardEvent)('keydown', { bubbles: true, cancelable: true, altKey: true, key: 'x', code: 'KeyX' }));
      await until(() => qc.textContent?.includes('Alt+X'), 'Custom Quick Commands chord not staged');
    };
    await captureLocal(); assert(calls('save_standard_hotkeys') === 0, 'Capture saved prematurely');
    draft(/^cancel(?: shortcuts)?$/i).click(); await until(() => qc.textContent?.includes('Alt+Q'), 'Cancel did not restore Alt+Q');
    await captureLocal(); draft(/^save(?: shortcuts)?$/i).click();
    await until(() => (n.settings.hotkeys as any).quickCommands === 'Alt+X', 'Explicit Save did not commit eighth binding');
    for (const [action, binding] of Object.entries(original)) if (action !== 'quickCommands') assert((n.settings.hotkeys as any)[action] === binding, `Changed existing ${action}`);
    draft(/Reset shortcuts to defaults/i).click(); await until(() => qc.textContent?.includes('Alt+Q'), 'Reset omitted eighth default');
    assert((n.settings.hotkeys as any).quickCommands === 'Alt+X' && calls('save_standard_hotkeys') === 1, 'Reset persisted');
    draft(/^cancel(?: shortcuts)?$/i).click(); await until(() => qc.textContent?.includes('Alt+X'), 'Reset Cancel lost committed eighth binding');
  });
  await run('targeted Quick Commands hotkey uses production launcher toggle open → close → reopen', async () => {
    const shown = calls('show_command_panel'); const hidden = calls('hide_command_panel');
    n.emit('settings-panel', 'quick-commands:toggle'); await settle();
    assert(calls('show_command_panel') === shown && calls('hide_command_panel') === hidden, 'Foreign target executed Quick Commands');
    n.hotkey('quickCommands'); await until(() => calls('show_command_panel') === shown + 1, 'Quick Commands hotkey did not open');
    n.hotkey('quickCommands'); await until(() => calls('hide_command_panel') === hidden + 1, 'Quick Commands hotkey did not close');
    n.hotkey('quickCommands'); await until(() => calls('show_command_panel') === shown + 2, 'Quick Commands hotkey did not reopen');
    const registration = n.trace.find(e => e.type === 'listen' && e.name === 'quick-commands:toggle' && e.receiver === 'top-bar');
    assert(registration?.target?.kind === 'WebviewWindow' && registration.target.label === 'top-bar', 'Quick Commands listener must pair explicit WebviewWindow target');
    button('top-bar', 'Open quick commands').click(); await until(() => calls('hide_command_panel') === hidden + 2, 'Launcher did not share hotkey toggle state');
  });
  await run('acknowledged Quick Commands capture suppresses toggle action and stages registered chord', async () => {
    const search = button('settings-panel', 'Capture Search shortcut'); search.focus(); search.click();
    await until(() => !!n.lease, 'No capture lease'); await settle();
    const shown = calls('show_command_panel'); const hidden = calls('hide_command_panel');
    n.hotkey('quickCommands'); await until(() => search.textContent?.includes((n.settings.hotkeys as any).quickCommands), 'Registered Quick Commands chord not diverted into capture');
    assert(calls('show_command_panel') === shown && calls('hide_command_panel') === hidden, 'Capture replayed Quick Commands action');
    draft(/^cancel(?: shortcuts)?$/i).click();
  });
  await run('no surface startup exception or unexpected native command', async () => {
    await settle(); assert(!n.trace.some(e => e.type === 'startup-error' || e.type === 'unexpected'), 'Fixture startup/IPC error');
  });
  evidence.status = evidence.cases.every(test => test.status === 'PASS') ? 'PASS' : 'FAIL';
  document.querySelector('#result')!.textContent = JSON.stringify(evidence.cases);
  return { status: evidence.status, cases: evidence.cases };
};
