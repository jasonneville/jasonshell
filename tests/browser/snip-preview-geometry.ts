// Browser-only fixture: real component/global CSS; no native shell or clipboard.
import '../../src/app.css';
import { mount, tick } from 'svelte';
import { mockIPC, mockWindows } from '@tauri-apps/api/mocks';
import { applyShellPreferences, storedShellPreferences } from '../../src/lib/shellPreferences';
import { applyShellTheme, storedShellThemeId } from '../../src/lib/themes';

const params = new URLSearchParams(location.search);
const target = document.getElementById('app')!;
if (!params.has('pane')) {
  // Iframes create exact browser viewport dimensions without changing app CSS.
  document.body.style.overflow = 'auto';
  target.style.height = 'auto';
  target.innerHTML = '<h1>Snip preview geometry</h1><p>Production CSS/component; mocked IPC. Error cases stress wrapped status text.</p><pre id="results">Waiting for four panes...</pre><div id="panes" style="display:grid;grid-template-columns:repeat(2,max-content);gap:16px"></div><details><summary>Raw measured rectangles</summary><pre id="raw-results"></pre></details>';
  const results = new Map<string, { case: string; pass: boolean; error?: string }>();
  const origins = new Set<Window>();
  window.addEventListener('message', event => {
    if (event.origin !== location.origin || !origins.has(event.source as Window) || event.data?.kind !== 'snip-geometry') return;
    results.set(event.data.case, event.data);
    document.getElementById('results')!.textContent = [...results.values()].map(result => `${result.pass ? 'PASS' : 'FAIL'} ${result.case}${result.error ? `: ${result.error}` : ''}`).join('\n');
    document.getElementById('raw-results')!.textContent = JSON.stringify([...results.values()], null, 2);
  });
  for (const [width, height] of [[320, 264], [280, 220]]) {
    for (const error of [false, true]) {
      const card = document.createElement('div'); document.getElementById('panes')!.append(card);
      const label = document.createElement('h2'); label.textContent = `${width}x${height} ${error ? 'wrapped error stress' : 'normal'}`; label.style.fontSize = '14px'; card.append(label);
      const frame = document.createElement('iframe');
      frame.title = label.textContent; frame.width = String(width); frame.height = String(height);
      frame.style.border = '0'; frame.src = `?pane=1&error=${Number(error)}`; card.append(frame);
      origins.add(frame.contentWindow!);
    }
  }
} else {
  void (async () => {
  try {
    applyShellTheme(storedShellThemeId(), { storage: null });
    applyShellPreferences(storedShellPreferences(), { storage: null, dispatch: false });
    const token = { generation: '7', captureId: '77777777777777777777777777777777' };
    const canvas = document.createElement('canvas'); canvas.width = 126; canvas.height = 75;
    const paint = canvas.getContext('2d')!;
    paint.fillStyle = '#4c72b0'; paint.fillRect(0, 0, 126, 75);
    paint.fillStyle = '#ffffff'; paint.fillRect(15, 15, 40, 35);
    const png = await new Promise<Blob>(resolve => canvas.toBlob(blob => resolve(blob!), 'image/png'));
    const bytes = new Uint8Array(await png.arrayBuffer());
    mockWindows('snip-preview-7');
    // The embedded browser can leave decode() pending despite complete=true and
    // naturalWidth=126. Exercise production's real load-event fallback instead;
    // do not stub a successful decode or change production behavior.
    Object.defineProperty(HTMLImageElement.prototype, 'decode', { configurable: true, value: undefined });
    mockIPC(command => {
      if (command === 'get_snip_context') return { ...token, phase: 'preview', monitorId: 'm0', width: 126, height: 75 };
      if (command === 'get_snip_image') return bytes;
      if (command === 'copy_snip') return { ...token, status: 'rejected', committed: false, durable: false, code: 'clipboard-busy' };
      if (command === 'save_snip') return { ...token, status: 'cancelled' };
      if (command === 'dismiss_snip') return { ...token, closed: true };
      throw new Error(`Unexpected fixture IPC: ${command}`);
    }, { shouldMockEvents: true });
    const { default: Preview } = await import('../../src/components/SnipPreviewSurface.svelte');
    mount(Preview, { target });
    await document.fonts.ready;
    const until = async (ready: () => boolean) => {
      const deadline = performance.now() + 5000;
      while (!ready()) {
        if (performance.now() > deadline) throw new Error('Fixture readiness timeout');
        await new Promise(resolve => setTimeout(resolve, 25));
      }
    };
    const button = (name: string) => target.querySelector<HTMLButtonElement>(`button[aria-label="${name}"]`)!;
    await until(() => !!button('Copy') && !button('Copy').disabled);
    if (params.get('error') === '1') {
      button('Copy').click();
      await until(() => !!target.querySelector('[role="alert"]'));
      // Deliberate layout stress, not a claimed native message: exercise wrapped
      // error handling if a future longer message/localization replaces this one.
      target.querySelector('[role="alert"]')!.textContent = 'Clipboard busy; try again. Could not confirm clipboard publication. Please retry copying the screenshot.';
    }
    await tick(); await new Promise(resolve => setTimeout(resolve, 50));
    const rect = (element: Element) => {
      const { x, y, width, height, right, bottom } = element.getBoundingClientRect();
      return { x, y, width, height, right, bottom };
    };
    const pane = { width: innerWidth, height: innerHeight };
    const frame = rect(target.querySelector('.preview-frame')!);
    const names = ['Copy', 'Save', 'Dismiss', 'Close screen snip'];
    const buttons = names.map(name => ({ name, ...rect(button(name)), svg: !!button(name).querySelector('svg') }));
    const status = rect(target.querySelector('.preview-status')!);
    const inside = buttons.every(b => b.x >= 0 && b.y >= 0 && b.right <= pane.width && b.bottom <= pane.height && b.width > 0 && b.height > 0 && b.svg);
    const belowImage = buttons.slice(0, 3).every(b => b.y >= frame.bottom);
    const rightAligned = Math.abs(buttons[2].right - buttons[3].right) < 1;
    const noOverflow = document.documentElement.scrollWidth <= pane.width && document.documentElement.scrollHeight <= pane.height;
    const wrapped = params.get('error') !== '1' || status.height > 20;
    parent.postMessage({ kind: 'snip-geometry', case: `${pane.width}x${pane.height}-${params.get('error') === '1' ? 'wrapped-error-stress' : 'normal'}`, pass: inside && belowImage && rightAligned && noOverflow && wrapped, inside, belowImage, rightAligned, noOverflow, wrapped, pane, frame, status, buttons }, location.origin);
  } catch (error) {
    target.append(document.createTextNode(`Fixture failed: ${String(error)}`));
    const image = target.querySelector('img');
    parent.postMessage({ kind: 'snip-geometry', case: `${innerWidth}x${innerHeight}-${params.get('error')}`, pass: false, error: String(error), image: image && { complete: image.complete, naturalWidth: image.naturalWidth, src: image.src }, status: target.querySelector('.preview-status')?.textContent }, location.origin);
  }
  })();
}
