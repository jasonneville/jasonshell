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
  const host = window as Window & { __sevenNative: Fixture; runJourney: () => Promise<string> };
  const n = host.__sevenNative = createFixture();
  n.settings.quickCommands = { orderVersion: 1, listWidth: 180, history: [], entries: [{ id: 'build', label: 'Build', mode: 'direct', targetPath: 'fixture.exe', args: [], commands: [], cwd: null, artifactLocation: null }] };
  const frames = Object.fromEntries(Object.keys(loaders).map(key => {
    const frame = document.createElement('iframe');
    frame.title = key;
    if (key === overlay) frame.style.border = '0'; // Match the undecorated native client viewport.
    frame.width = key === overlay ? '327' : '850';
    frame.height = key === panel ? '500' : key === overlay ? '57' : '100';
    frame.src = `?surface=${key}`;
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
    const token = outcome === 'Succeeded' ? '--js-color-success-border' : '--js-color-error-text';
    const probe = document.createElement('span');
    probe.style.color = `var(${token})`;
    icon.parentElement!.append(probe);
    const css = document.defaultView!.getComputedStyle.bind(document.defaultView);
    const expected = css(probe).color;
    probe.remove();
    if (!css(icon).getPropertyValue(token).trim() || css(icon).stroke !== expected) throw new Error(`${outcome} SVG stroke ${css(icon).stroke} must match ${token} (${expected})`);
    const [labelBox, verdictBox, iconBox, dismissBox] = [label, verdict, icon, dismiss].map(el => el.getBoundingClientRect());
    const card = document.querySelector('[aria-label="Quick Commands notification"]')!.getBoundingClientRect();
    const near = (actual: number, expected: number) => Math.abs(actual - expected) < 0.25;
    if (!near(card.x, 7) || !near(card.y, 7) || !near(card.width, 320) || !near(card.height, 50)) throw new Error(`Expected 320×50 card at (7, 7); got ${JSON.stringify(card.toJSON())}`);
    if (!near(dismissBox.width, 14) || !near(dismissBox.height, 14) || !near(dismissBox.x + dismissBox.width / 2, card.x) || !near(dismissBox.y + dismissBox.height / 2, card.y)) throw new Error('Expected 14px dismiss centered on the card corner');
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
    const dismissHit = document.elementFromPoint(card.x - 3, card.y - 3);
    if (verdictHit?.closest('button') !== open || dismissHit?.closest('button') !== dismiss) throw new Error('Verdict and dismiss must hit their separate buttons');
    return { open, dismissHit };
  }
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
