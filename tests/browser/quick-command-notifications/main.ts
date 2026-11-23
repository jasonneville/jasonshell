import { mount } from 'svelte';
import '../../../src/app.css';
import { createFixture, fixture, type Fixture } from './bridge';

const label = new URLSearchParams(location.search).get('surface');
const root = document.querySelector<HTMLElement>('#surface')!;
const overlay = 'quick-command-notification';
const panel = 'command-panel';
const loaders = {
  'top-bar': () => import('../../../src/components/TopBar.svelte'),
  [panel]: () => import('../../../src/components/CommandPanelSurface.svelte'),
  [overlay]: () => import('../../../src/components/QuickCommandNotificationSurface.svelte'),
};

if (label) {
  document.documentElement.dataset.theme = new URLSearchParams(location.search).get('theme') ?? 'base-dark';
  const recordError = (message: string) => fixture().record('startup-error', { surface: label, message });
  window.addEventListener('error', event => recordError(event.message));
  window.addEventListener('unhandledrejection', event => recordError(String(event.reason)));
  try {
    const component = await loaders[label as keyof typeof loaders]();
    mount(component.default, { target: root });
    root.dataset.ready = 'true';
  } catch (error) {
    recordError(String(error));
  }
} else {
  const host = window as Window & { __sevenNative: Fixture; runJourney: () => Promise<string>; beginHover: () => Promise<any>; hoverStatus: () => any; endHover: () => Promise<void> };
  const n = host.__sevenNative = createFixture();
  n.settings.quickCommands = { orderVersion: 1, listWidth: 180, history: [], entries: [{ id: 'build', label: 'Build', mode: 'direct', targetPath: 'fixture.exe', args: [], commands: [], cwd: null, artifactLocation: null }] };
  const frames = Object.fromEntries(Object.keys(loaders).map(key => {
    const frame = document.createElement('iframe');
    frame.title = key;
    if (key === overlay) frame.style.border = '0'; // Match the undecorated native client viewport.
    frame.width = key === overlay ? '327' : '850';
    frame.height = key === panel ? '500' : key === overlay ? '57' : '100';
    frame.src = `?surface=${key}&theme=${encodeURIComponent(new URLSearchParams(location.search).get('theme') ?? 'base-dark')}`;
    root.append(frame);
    return [key, frame];
  }));
  const doc = (key: string) => frames[key].contentDocument!;
  const button = (key: string, name: RegExp) => [...doc(key).querySelectorAll('button')].find(b => name.test(b.getAttribute('aria-label') ?? b.textContent ?? ''));
  const traceCount = (type: string) => n.trace.filter(e => e.type === type).length;
  function verdictHitTargets(outcome: 'Succeeded' | 'Failed') {
    const document = doc(overlay);
    const open = button(overlay, /open/i)!;
    const dismiss = button(overlay, /dismiss/i)!;
    const label = [...open.querySelectorAll('strong, span')].find(el => el.textContent === 'Build');
    const verdict = [...open.querySelectorAll('span')].find(el => el.textContent === outcome);
    const icon = open.querySelector('svg[aria-hidden="true"]');
    if (!label || !verdict || !icon || open.textContent?.trim().replace(/\s+/g, ' ') !== `Build ${outcome}`) throw new Error('Expected command, plain verdict, and decorative icon');
    // VIS-20b: the verdict icon uses a solid semantic colour (the old 42%-alpha success-border token
    // is rejected) that keeps >= 3:1 non-text contrast against the card surface.
    const css = document.defaultView!.getComputedStyle.bind(document.defaultView);
    const channels = (value: string) => { const m = value.match(/^rgba?\(([^)]*)\)/); if (!m) throw new Error(`Unparseable colour ${value}`); const p = m[1].split(/[\s,/]+/).filter(Boolean).map(Number); return { r: p[0], g: p[1], b: p[2], a: p[3] ?? 1 }; };
    const luminance = ({ r, g, b }: { r: number; g: number; b: number }) => [r, g, b].map(c => { const s = c / 255; return s <= 0.03928 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4; }).reduce((sum, c, i) => sum + c * [0.2126, 0.7152, 0.0722][i], 0);
    const stroke = channels(css(icon).stroke);
    if (stroke.a !== 1) throw new Error(`${outcome} icon colour ${css(icon).stroke} must be opaque`);
    const cardBackground = channels(css(document.querySelector('[aria-label="Quick Commands notification"]')!).backgroundColor);
    const [hi, lo] = [luminance(stroke), luminance(cardBackground)].sort((x, y) => y - x);
    if ((hi + 0.05) / (lo + 0.05) < 3) throw new Error(`${outcome} icon contrast ${((hi + 0.05) / (lo + 0.05)).toFixed(2)} < 3:1`);
    const live = document.querySelector('[role="status"], [role="alert"]');
    if (!live || live.getAttribute('role') !== (outcome === 'Succeeded' ? 'status' : 'alert') || !live.textContent?.includes('Build') || !live.textContent.includes(outcome)) throw new Error(`Expected role=${outcome === 'Succeeded' ? 'status' : 'alert'} live region containing the label and verdict`);
    const [labelBox, verdictBox, iconBox, dismissBox] = [label, verdict, icon, dismiss].map(el => el.getBoundingClientRect());
    const card = document.querySelector('[aria-label="Quick Commands notification"]')!.getBoundingClientRect();
    const near = (actual: number, expected: number) => Math.abs(actual - expected) < 0.25;
    // The 50 px strip geometry is unchanged (320x50 card); its offset inside the native client area may change.
    if (!near(card.width, 320) || !near(card.height, 50)) throw new Error(`Expected a 320×50 card; got ${JSON.stringify(card.toJSON())}`);
    const viewport = { width: document.documentElement.clientWidth, height: document.documentElement.clientHeight };
    if (card.x < 0 || card.y < 0 || card.right > viewport.width + 0.25 || card.bottom > viewport.height + 0.25) throw new Error('Card is clipped by the native client viewport');
    // VIS-20b: >= 24x24 CSS px dismiss target, fully inside the client viewport (not half outside).
    if (dismissBox.width < 24 - 0.25 || dismissBox.height < 24 - 0.25) throw new Error(`Dismiss target ${dismissBox.width}x${dismissBox.height} must be at least 24x24`);
    if (dismissBox.x < 0 || dismissBox.y < 0 || dismissBox.right > viewport.width + 0.25 || dismissBox.bottom > viewport.height + 0.25) throw new Error('Dismiss target is clipped by the client viewport');
    const dismissCenter = { x: dismissBox.x + dismissBox.width / 2, y: dismissBox.y + dismissBox.height / 2 };
    for (const [dx, dy] of [[0, 0], [-11, -11], [11, -11], [-11, 11], [11, 11]]) {
      if (document.elementFromPoint(dismissCenter.x + dx, dismissCenter.y + dy)?.closest('button') !== dismiss) throw new Error(`24x24 area around the dismiss centre is not all dismiss hit target (offset ${dx},${dy})`);
    }
    if (labelBox.right > verdictBox.left || verdictBox.right > iconBox.left) throw new Error('Trailing verdict overlaps or is out of order');
    const circles = [...dismiss.querySelectorAll('svg circle')];
    const countdown = circles.flatMap(circle => circle.getAnimations().map(animation => ({ circle, animation })))
      .find(({ animation }) => animation.effect?.getTiming().duration === 6000);
    if (document.defaultView!.matchMedia('(prefers-reduced-motion: reduce)').matches) {
      if (circles.some(circle => circle.getAnimations().length)) throw new Error('Reduced-motion ring must not animate');
    } else {
      if (!countdown || countdown.animation.playState !== 'running') throw new Error('No running six-second dismiss countdown');
      const { animation, circle } = countdown;
      const currentTime = animation.currentTime;
      animation.currentTime = 0;
      const full = parseFloat(document.defaultView!.getComputedStyle(circle).strokeDashoffset);
      animation.currentTime = 3000;
      const halfway = parseFloat(document.defaultView!.getComputedStyle(circle).strokeDashoffset);
      animation.currentTime = currentTime;
      if (!(halfway > full)) throw new Error('Dismiss ring does not visibly drain');
    }
    const hit = (box: DOMRect) => document.elementFromPoint(box.x + box.width / 2, box.y + box.height / 2) as HTMLElement | null;
    const verdictHit = hit(verdictBox);
    const dismissHit = document.elementFromPoint(dismissCenter.x, dismissCenter.y);
    if (verdictHit?.closest('button') !== open || dismissHit?.closest('button') !== dismiss) throw new Error('Verdict and dismiss must hit their separate buttons');
    return { open, dismissHit };
  }
  const reducedMotion = () => doc(overlay).defaultView!.matchMedia('(prefers-reduced-motion: reduce)').matches;
  const ring = () => [...button(overlay, /dismiss/i)!.querySelectorAll('svg circle')].flatMap(circle => circle.getAnimations()).find(animation => (animation.effect as KeyframeEffect | null)?.getTiming().duration === 6000);
  const bannerActions = () => n.trace.filter(e => e.type === 'banner-action').map(e => e.action);
  // VIS-20b: focus-within pauses the native deadline and the visible ring (WCAG 2.2.1); leaving resumes.
  // Under reduced motion there is no ring animation but native pausing must still work.
  async function pauseByFocus(outcome: string) {
    const dismiss = button(overlay, /dismiss/i)!;
    const before = bannerActions().filter(action => action === 'pause').length;
    dismiss.focus();
    await until(() => n.paused, `${outcome}: focusing the banner did not pause native`);
    if (bannerActions().filter(action => action === 'pause').length !== before + 1) throw new Error('Focus must send exactly one pause');
    if (!reducedMotion()) await until(() => ring()?.playState === 'paused', `${outcome}: ring animation did not pause`);
    else if (ring()) throw new Error('Reduced-motion ring must not animate');
    await new Promise(resolve => setTimeout(resolve, 150));
    if (!n.paused || bannerActions().filter(action => action === 'pause').length !== before + 1) throw new Error('Pause must be idempotent while focus stays inside');
    dismiss.blur();
    await until(() => !n.paused, `${outcome}: leaving did not resume native`);
    if (!reducedMotion()) await until(() => ring()?.playState === 'running', `${outcome}: ring animation did not resume`);
  }
  host.beginHover = async () => {
    n.finish('hover-run', true, 'success');
    await until(() => n.renderedId === n.banner?.id && button(overlay, /open/i), 'No rendered banner for hover journey');
    frames[overlay].scrollIntoView({ block: 'center' });
    const box = frames[overlay].getBoundingClientRect();
    const open = button(overlay, /open/i)!.getBoundingClientRect();
    return { over: { x: Math.round(box.left + open.left + open.width / 2), y: Math.round(box.top + open.top + open.height / 2) }, away: { x: Math.round(box.left + box.width / 2), y: Math.max(2, Math.round(box.top - 30)) }, reducedMotion: reducedMotion() };
  };
  host.hoverStatus = () => ({ paused: n.paused, actions: bannerActions(), ring: ring()?.playState ?? null });
  host.endHover = async () => { button(overlay, /dismiss/i)!.click(); await until(() => !n.banner, 'Hover journey banner did not dismiss'); };
  async function until(check: () => unknown, message: string) {
    const deadline = performance.now() + 3000;
    while (!check()) {
      const startupError = n.trace.find(e => e.type === 'startup-error');
      if (startupError) throw new Error(JSON.stringify(startupError));
      if (performance.now() > deadline) throw new Error(message);
      await new Promise(resolve => requestAnimationFrame(resolve));
    }
  }
  host.runJourney = async () => {
    await until(() => Object.keys(frames).every(key => doc(key).querySelector('[data-ready]')), 'Production surfaces did not mount');
    await until(() => button('top-bar', /open quick commands/i), 'No Quick Commands launcher');
    n.finish('dismissed', true, 'failure');
    await until(() => n.renderedId === n.banner?.id && button(overlay, /dismiss/i), 'No rendered and acknowledged dismiss action');
    verdictHitTargets('Failed');
    await pauseByFocus('Failed');
    verdictHitTargets('Failed').dismissHit!.dispatchEvent(new MouseEvent('click', { bubbles: true, clientX: 4, clientY: 4 }));
    await until(() => !button(overlay, /dismiss/i), 'Dismiss did not remove banner');
    if (n.panelVisible || traceCount('show') || !n.trace.some(e => e.type === 'banner-action' && e.action === 'dismiss')) throw new Error('Dismiss must acknowledge only the banner and leave panel closed');

    n.finish('older', false);
    n.finish('newer', false);
    button('top-bar', /open quick commands/i)!.click();
    await until(() => n.panelVisible, 'Could not show warm panel');
    const runs = () => [...doc(panel).querySelectorAll<HTMLDetailsElement>('details')];
    const saved = button(panel, /edit build/i)!;
    saved.click();
    await until(() => button(panel, /previous runs/i), 'Previous Runs tab not available');
    button(panel, /previous runs/i)!.click();
    await until(() => runs().some(row => row.textContent?.includes('newer output sentinel')), 'Newer run missing before notification');
    runs().find(row => row.textContent?.includes('newer output sentinel'))!.querySelector('summary')!.click();
    await until(() => runs().some(row => row.open && row.innerText.includes('newer output sentinel')), 'Could not pre-expand newer run in warm panel');
    button('top-bar', /open quick commands/i)!.click();
    await until(() => !n.panelVisible, 'Could not hide warm panel');
    n.announce('older');
    await until(() => n.renderedId === n.banner?.id && button(overlay, /open/i), 'No rendered and acknowledged Open action');
    verdictHitTargets('Succeeded');
    await pauseByFocus('Succeeded');
    verdictHitTargets('Succeeded').open.click();
    await until(() => n.panelVisible && runs().some(row => row.open && row.innerText.includes('older output sentinel')), 'Actual panel did not expand the exact older output');
    if (!runs().some(row => !row.open && row.textContent?.includes('newer output sentinel'))) throw new Error('Newer run must be present and collapsed');
    if (traceCount('show') !== 2 || traceCount('hide') !== 1) throw new Error('Notification must use show-only');

    button('top-bar', /open quick commands/i)!.click();
    await until(() => !n.panelVisible, 'Ordinary launcher did not close shown panel');
    n.hotkey('quickCommands');
    await until(() => n.panelVisible, 'Mocked Alt+Q wake did not reopen panel');
    n.hotkey('quickCommands');
    await until(() => !n.panelVisible, 'Mocked Alt+Q wake did not close panel');
    const unexpected = n.trace.filter(e => e.type === 'unexpected' || e.type === 'startup-error');
    if (unexpected.length) throw new Error(JSON.stringify(unexpected));
    return 'PASS: dismiss leaves panel closed; Open expands only older exact run; launcher and Alt+Q still toggle';
  };
}
