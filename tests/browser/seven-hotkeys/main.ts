import { createNative, type NativeFixture } from './bridge';
import './panel-toggles';
import './quick-command-hotkey';
const host = window as Window & { __sevenNative?: NativeFixture; __sevenEvidence?: any; __runSeven?: () => Promise<any> };
host.__sevenNative ??= createNative();
const n = host.__sevenNative;
const evidence = host.__sevenEvidence = { status: 'NOT RUN', checks: [] as string[], trace: n.trace, error: '' };
const frame = (label: string) => document.querySelector<HTMLIFrameElement>(`#${label}`)!;
const doc = (label: string) => frame(label).contentDocument!;
const text = (label: string) => doc(label).body.textContent ?? '';
const buttons = (label: string) => Array.from(doc(label).querySelectorAll<HTMLButtonElement>('button'));
const button = (label: string, name: string | RegExp) => {
  const found = buttons(label).find(node => typeof name === 'string'
    ? (node.getAttribute('aria-label') ?? node.textContent?.trim()) === name
    : name.test(node.getAttribute('aria-label') ?? node.textContent?.trim() ?? ''));
  if (!found) throw new Error(`Missing ${label} button: ${name}`);
  return found;
};
const assert = (condition: unknown, message: string) => { if (!condition) throw new Error(message); };
async function until(check: () => unknown, message: string) {
  const end = performance.now() + 7000;
  while (!check()) { if (performance.now() > end) throw new Error(message); await new Promise(resolve => requestAnimationFrame(resolve)); }
}
const count = (command: string) => n.trace.filter(entry => entry.type === 'invoke' && entry.command === command).length;
const shortcut = (name: string) => button('settings-panel', `Capture ${name} shortcut`);
const shortcutRegion = () => doc('settings-panel').querySelector<HTMLElement>('[aria-labelledby="hotkeys-heading"]')!;
const draftButton = (name: RegExp) => {
  const found = Array.from(shortcutRegion().querySelectorAll<HTMLButtonElement>('button')).find(node => name.test(node.getAttribute('aria-label') ?? node.textContent?.trim() ?? ''));
  if (!found) throw new Error(`Missing shortcut draft button ${name}`); return found;
};
async function startCapture(name: string) {
  const before = count('begin_standard_hotkey_capture'); const node = shortcut(name);
  node.focus(); node.click(); await until(() => count('begin_standard_hotkey_capture') > before && !!n.lease, `No capture ACK for ${name}`);
  await new Promise(resolve => requestAnimationFrame(resolve)); return node;
}
function localChord(node: HTMLElement, letter: string) {
  const win = doc('settings-panel').defaultView!;
  node.dispatchEvent(new win.KeyboardEvent('keydown', { key: letter.toLowerCase(), code: `Key${letter}`, altKey: true, bubbles: true, cancelable: true }));
}
const passed = (name: string) => { evidence.checks.push(name); document.querySelector('#result')!.textContent = `RUNNING ${evidence.checks.length}: ${name}`; };

host.__runSeven = async () => {
  evidence.status = 'RUNNING'; evidence.checks = []; evidence.error = '';
  try {
    await until(() => n.listeners.size > 15 && !!doc('settings-panel').querySelector('[aria-label="Capture Search shortcut"]') && !!doc('process-manager').querySelector('[role="tab"]'), 'Production surfaces/listeners not mounted');
    await until(() => n.trace.some(entry => entry.type === 'listen' && entry.name === 'settings:open-requested'), 'Settings open-requested listener absent');
    // Wrong target must not be rescued by an any-target listener.
    const shows = count('show_settings_panel'); const managers = count('show_process_manager');
    n.emit('process-manager', 'settings:open-requested'); n.emit('settings-panel', 'process-manager:open-requested');
    await new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve)));
    assert(count('show_settings_panel') === shows && count('show_process_manager') === managers, 'foreign target opened a surface');
    n.hotkey('settings'); await until(() => count('show_settings_panel') === shows + 1, 'Settings hotkey did not route through production TopBar');
    n.hotkey('settings'); await until(() => !n.visible['settings-panel'], 'repeated Settings hotkey failed to hide');
    n.hotkey('settings'); await until(() => n.visible['settings-panel'], 'Settings third hotkey failed to reopen');
    const settingsRoot = doc('settings-panel').querySelector('main');
    button('settings-panel', 'Create copy').click();
    await until(() => !!doc('settings-panel').querySelector('input[maxlength="80"]'), 'theme draft not mounted');
    const themeName = doc('settings-panel').querySelector<HTMLInputElement>('input[maxlength="80"]')!;
    themeName.value = 'Unsaved hotkey journey theme'; themeName.dispatchEvent(new (doc('settings-panel').defaultView!.Event)('input', { bubbles: true }));
    n.hotkey('settings'); await until(() => !n.visible['settings-panel'], 'Settings draft hide absent');
    n.hotkey('settings'); await until(() => n.visible['settings-panel'], 'Settings draft reopen absent');
    assert(doc('settings-panel').querySelector('main') === settingsRoot && themeName.isConnected && themeName.value === 'Unsaved hotkey journey theme', 'Settings opening remounted/discarded theme draft');
    passed('paired top-bar routing; Settings toggles hide/reopen; mount and unsaved theme draft retained');

    n.hotkey('processManager'); await until(() => count('show_process_manager') === managers + 1, 'PM hotkey did not route through production BottomBar');
    const processes = () => doc('process-manager').querySelector('[role="tab"][aria-selected="true"]')?.textContent?.trim() === 'Processes';
    assert(processes(), 'PM did not open Processes');
    button('process-manager', 'System Info').click(); await until(() => n.context.activeTab === 'system_info', 'System Info tab not committed');
    const generation = n.context.openGeneration;
    n.hotkey('processManager'); await until(() => !n.visible['process-manager'], 'repeated PM press did not hide');
    assert(n.context.openGeneration === generation, 'PM hide advanced open generation');
    n.hotkey('processManager'); await until(() => n.visible['process-manager'] && processes(), 'PM reopen did not return to Processes');
    passed('paired bottom-bar routing; PM toggles hide/reopen and resets Processes only on reopening');

    let search = await startCapture('Search');
    const panelActions = () => n.trace.filter(entry => entry.type === 'invoke' && /(?:show|hide)_(?:settings_panel|process_manager)/.test(entry.command)).length;
    const beforeCaptureActions = panelActions();
    n.hotkey('settings'); await until(() => search.textContent?.includes('Alt+O'), 'Settings hotkey did not divert into capture');
    assert(panelActions() === beforeCaptureActions && n.visible['settings-panel'], 'Settings hotkey executed a panel action during capture');
    draftButton(/^cancel(?: shortcuts)?$/i).click();
    await until(() => search.textContent?.includes('Ctrl+Space'), 'Cancel did not clear capture diversion draft');
    search = await startCapture('Search');
    localChord(search, 'P'); assert(search.textContent?.includes('Ctrl+Space'), 'registered chord captured from local keydown');
    n.emit('process-manager', 'standard-hotkeys:capture', { ...n.lease, sequence: 1, action: 'processManager', binding: 'Alt+P' });
    await new Promise(resolve => requestAnimationFrame(resolve)); assert(search.textContent?.includes('Ctrl+Space'), 'foreign capture populated draft');
    n.capture('Alt+P'); await until(() => search.textContent?.includes('Alt+P'), 'native registered capture did not stage duplicate');
    assert(count('save_standard_hotkeys') === 0 && count('save_shell_settings') === 0, 'capture wrote before Save');
    draftButton(/^save(?: shortcuts)?$/i).click(); assert(count('save_standard_hotkeys') === 0, 'unresolved duplicate saved');
    const pm = await startCapture('Process Manager'); n.capture('Ctrl+Space', { action: 'search' });
    await until(() => pm.textContent?.includes('Ctrl+Space'), 'swap second half not staged');
    passed('native ACK registered chord dedupe; foreign capture rejected; temporary duplicate swap staged without writes');

    n.failSave = true; draftButton(/^save(?: shortcuts)?$/i).click();
    await until(() => /registration failed/i.test(shortcutRegion().textContent ?? ''), 'failed Save not reported');
    assert(n.settings.hotkeys.search === 'Ctrl+Space' && search.textContent?.includes('Alt+P'), 'failed Save lost draft or falsely committed');
    n.failSave = false; n.loseFocusOnSave = true; draftButton(/^save(?: shortcuts)?$/i).click();
    await until(() => n.settings.hotkeys.search === 'Alt+P', 'retry Save did not commit');
    assert((n.settings.hotkeys as any).processManager === 'Ctrl+Space' && count('save_standard_hotkeys') === 2, 'swap not committed once per explicit attempt');
    await until(() => /Shortcuts saved\./.test(shortcutRegion().textContent ?? ''), 'focus loss after commit falsely reported Save failure');
    assert(!n.settingsFocused && !shortcutRegion().querySelector('[role="alert"]'), 'postcommit focus-loss observation was not exercised cleanly');
    passed('failed strict Save retains draft; retry commits seven-way swap despite postcommit focus loss');
    n.dismiss('settings-panel'); n.hotkey('settings'); await until(() => n.settingsFocused, 'Settings did not regain focus before next capture');

    search = await startCapture('Search'); localChord(search, 'X');
    await until(() => search.textContent?.includes('Alt+X'), 'unregistered local chord not captured after ACK');
    const locked = button('settings-panel', 'Lock top bar');
    assert(locked, 'top-bar lock control absent'); locked!.click();
    await until(() => count('save_shell_bar_lock') > 0, 'unrelated control not persisted');
    assert(n.settings.hotkeys.search === 'Alt+P' && search.textContent?.includes('Alt+X'), 'unrelated control committed or discarded shortcut draft');
    draftButton(/^cancel(?: shortcuts)?$/i).click(); await until(() => search.textContent?.includes('Alt+P'), 'Cancel failed to restore committed shortcut');
    draftButton(/Reset shortcuts to defaults/i).click(); await until(() => search.textContent?.includes('Ctrl+Space'), 'Reset failed to stage defaults');
    assert(n.settings.hotkeys.search === 'Alt+P' && count('save_standard_hotkeys') === 2, 'Reset persisted without Save');
    draftButton(/^cancel(?: shortcuts)?$/i).click();
    const hidden = count('hide_settings_panel');
    button('settings-panel', 'Close settings').click(); await until(() => count('hide_settings_panel') === hidden + 1, 'Settings close did not hide');
    n.hotkey('settings'); await until(() => n.visible['settings-panel'], 'Settings reopen absent');
    assert(themeName.isConnected && themeName.value === 'Unsaved hotkey journey theme', 'hide/reopen lost unsaved theme draft');
    passed('unregistered local capture after ACK; unrelated control committed-only; Reset/Cancel staged; hide/reopen retains theme draft');

    search = await startCapture('Search'); localChord(search, 'Y');
    await until(() => search.textContent?.includes('Alt+Y'), 'postcommit channel-failure draft absent');
    n.statusUnavailableOnSave = true; draftButton(/^save(?: shortcuts)?$/i).click();
    await until(() => /Shortcuts saved; runtime status unavailable/.test(shortcutRegion().textContent ?? ''), 'committed channel failure was misreported as failed Save');
    assert(n.settings.hotkeys.search === 'Alt+Y' && !shortcutRegion().querySelector('[role="alert"]'), 'unavailable status undid committed settings or rendered failure');
    draftButton(/Reset shortcuts to defaults/i).click(); draftButton(/^cancel(?: shortcuts)?$/i).click();
    await until(() => search.textContent?.includes('Alt+Y'), 'Cancel restored stale precommit settings after status-channel failure');
    assert(count('save_standard_hotkeys') === 3, 'status-channel failure triggered duplicate persistence');
    passed('native postcommit observation failure returns committed nullable status; renderer commits settings and announces saved warning without retry');

    for (const [event, target, receiver] of [['settings:open-requested', 'top-bar', 'top-bar'], ['process-manager:open-requested', 'bottom-bar', 'bottom-bar'], ['standard-hotkeys:capture', 'settings-panel', 'settings-panel']]) {
      const registration = n.trace.find(entry => entry.type === 'listen' && entry.name === event && entry.receiver === receiver);
      assert(registration?.target?.kind === 'WebviewWindow' && registration.target.label === target, `unpaired listener target ${event}`);
    }
    assert(!n.trace.some(entry => entry.type === 'invoke' && /kill_process|start_snip|start_speech/.test(entry.command)), 'capture replayed an action');
    assert(!n.trace.some(entry => entry.type === 'unexpected'), `unhandled fixture IPC: ${n.trace.filter(entry => entry.type === 'unexpected').map(entry => entry.command).join(', ')}`);
    assert(!n.trace.some(entry => entry.type === 'startup-error'), `surface startup exception: ${JSON.stringify(n.trace.filter(entry => entry.type === 'startup-error'))}`);
    passed('all new listeners pair explicit WebviewWindow targets; capture never replays actions');
    evidence.status = 'PASS';
  } catch (error) { evidence.status = 'FAIL'; evidence.error = error instanceof Error ? error.stack ?? error.message : String(error); }
  document.querySelector('#result')!.textContent = `${evidence.status}: ${evidence.checks.length} checks ${evidence.error}`;
  return evidence;
};
document.querySelector<HTMLButtonElement>('#run')!.onclick = () => void host.__runSeven!();
