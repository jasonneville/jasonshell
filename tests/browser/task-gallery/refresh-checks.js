// Run only in visible browser: wmux browser eval <contents of this file>.
// Production gallery DOM/CSS, synthetic IPC/image data; never native shell.
(async () => {
  const fixture = window.galleryFixture;
  if (!fixture) throw new Error('Open task-gallery fixture on port 4179 first');
  const results = [];
  const check = (name, pass, actual) => results.push({ name, pass: Boolean(pass), actual });
  const settle = async () => {
    await new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve)));
    await new Promise(resolve => setTimeout(resolve, 80));
  };
  fixture.state.deferSync = false;
  fixture.state.failSync = false;
  fixture.configure(960, 600, 'dark');
  await fixture.open(true, fixture.manyRows.slice(0, 6));
  await settle();
  const panel = document.querySelector('.task-gallery-panel');
  const style = getComputedStyle(panel);
  check('gallery outer chrome matches preview 8px rounding and raised bevel',
    style.borderRadius === '8px' && style.boxShadow !== 'none' && style.boxShadow.includes('inset'),
    { radius: style.borderRadius, shadow: style.boxShadow });
  const start = fixture.ledger.length;
  await fixture.snapshot(fixture.state.open.windows);
  await settle();
  check('same-session unchanged refresh does not request empty native destination set',
    !fixture.ledger.slice(start).some(entry => entry.name === 'sync_task_gallery_thumbnails' && entry.args.args.frames.length === 0),
    fixture.ledger.slice(start).filter(entry => entry.name === 'sync_task_gallery_thumbnails'));
  const target = document.querySelector('[data-gallery-hwnd="206"] .task-gallery-activate');
  const bounds = target.getBoundingClientRect();
  target.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true, clientX: bounds.right - 4, clientY: bounds.bottom - 4 }));
  await settle();
  const menu = document.querySelector('[role="menu"]');
  check('menu opens after acknowledged native suspension', menu && fixture.state.liveFrames.length === 0,
    { menu: Boolean(menu), live: fixture.state.liveFrames });
  // Fixture currently marks indexes 0/3 live, 1/4 captured, 2/5 genuinely
  // unavailable. Live rows must have a real backup, not a fabricated placeholder.
  check('live rows retain captured backup under webview context overlay',
    ['201', '204'].every(hwnd => document.querySelector(`[data-gallery-hwnd="${hwnd}"] .task-gallery-captured-image`)),
    ['201', '204'].map(hwnd => ({ hwnd, text: document.querySelector(`[data-gallery-hwnd="${hwnd}"]`).textContent })));
  const focus = document.activeElement;
  await fixture.snapshot(fixture.state.open.windows);
  await settle();
  check('same-session refresh preserves menu DOM and focused action',
    menu && document.querySelector('[role="menu"]') === menu && document.activeElement === focus,
    { menuSurvives: document.querySelector('[role="menu"]') === menu, focused: document.activeElement?.textContent });
  document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true, cancelable: true }));
  await settle();
  check('Escape dismisses menu while gallery remains', !document.querySelector('[role="menu"]') && document.querySelector('.task-gallery-panel'), null);
  const report = { results, failed: results.filter(result => !result.pass),
    evidenceBoundary: 'Visible browser production DOM/CSS and synthetic event/IPC checks; real right-click input and native DWM remain separate gates.' };
  window.galleryRefreshReport = report;
  document.querySelector('#summary').textContent = JSON.stringify(report, null, 2);
  return report;
})()
