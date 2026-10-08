// Actual production component/global CSS in native-sized mocked hosts. No copied
// gallery paint/layout and no native runtime. Bounded DOM/IPC evidence only.
window.checkGalleryTightSeams = async () => {
  const fixture = window.galleryFixture;
  const results = [];
  const check = (name, pass, actual) => results.push({ name, pass: Boolean(pass), actual });
  const near = (name, actual, expected) => check(name, Math.abs(actual - expected) < 0.6, { actual, expected });
  const frame = () => new Promise(resolve => requestAnimationFrame(resolve));
  const until = async (predicate, name) => {
    for (let i = 0; i < 120; i++) { if (predicate()) return; await frame(); }
    throw new Error(`Condition not reached: ${name}`);
  };
  const settle = async () => { await frame(); await frame(); };
  const commands = name => fixture.ledger.filter(entry => entry.kind === 'command' && entry.name === name);
  const viewport = () => document.querySelector('.task-gallery-viewport');
  const tiles = () => [...document.querySelectorAll('[data-gallery-hwnd]')];
  const activation = index => tiles()[index].querySelector('.task-gallery-activate');
  const close = index => tiles()[index].querySelector('.task-gallery-close');
  const key = async (control, key, extras = {}) => {
    // Focus handlers update Svelte's roving index on its next flush.
    await settle();
    control.dispatchEvent(new KeyboardEvent('keydown', { key, bubbles: true, ...extras })); await settle();
  };
  const rowsFor = count => count === 2
    ? [fixture.rows[0], { ...fixture.rows[1], title: 'Beta — exceptionally long document title that must truncate without moving Close' }]
    : count === 3 ? fixture.rows : fixture.manyRows.slice(0, count);
  const fresh = async (count, width = 960, theme = 'dark') => {
    fixture.state.deferSync = false; fixture.state.failSync = false;
    fixture.configure(width, 600, theme);
    await fixture.open(true, rowsFor(count));
    await until(() => commands('sync_task_gallery_thumbnails').some(entry => entry.args.args.revision === fixture.state.open.revision), 'current thumbnail sync');
    await settle();
  };
  try {
    for (const theme of ['dark', 'light']) for (const count of [2, 3, 4, 6, 30]) for (const width of [960, 396, 144]) {
      await fresh(count, width, theme);
      const prefix = `${theme}/${count}/${width}`;
      const geometry = fixture.measure(), layout = fixture.state.open.layout;
      near(`${prefix} native host panel width`, geometry.panel.width, layout.panelWidth);
      near(`${prefix} native host panel height`, geometry.panel.height, layout.panelHeight);
      check(`${prefix} clamped available width`, geometry.panel.width <= width + 0.6, geometry.panel);
      check(`${prefix} exact snapshot order`, JSON.stringify(geometry.tiles.map(tile => tile.hwnd)) === JSON.stringify(rowsFor(count).map(row => row.hwnd)), geometry.tiles.map(tile => tile.hwnd));
      check(`${prefix} <=3 columns`, layout.columns <= 3 && layout.columns >= 1, layout);
      check(`${prefix} <=2 visible rows`, geometry.viewport.clientHeight <= 280, geometry.viewport);
      check(`${prefix} vertical scroll for hidden rows`, Math.ceil(count / layout.columns) <= 2 || geometry.viewport.scrollHeight > geometry.viewport.clientHeight, geometry.viewport);
      check(`${prefix} no horizontal content overflow`, geometry.viewport.scrollWidth <= geometry.viewport.clientWidth + 1, geometry.viewport);
      near(`${prefix} card width`, geometry.tiles[0].tile.width, layout.cardWidth);
      near(`${prefix} card height`, geometry.tiles[0].tile.height, 136);
      for (const tile of geometry.tiles) {
        check(`${prefix}/${tile.hwnd} Close >=24x24`, tile.close.width >= 24 && tile.close.height >= 24, tile.close);
        check(`${prefix}/${tile.hwnd} title nonzero`, tile.titleWidth > 0, tile.titleWidth);
      }
      check(`${prefix} long title truncates`, geometry.tiles.at(-1).titleTruncated, geometry.tiles.at(-1));
      const sync = commands('sync_task_gallery_thumbnails').at(-1).args.args;
      check(`${prefix} bounded native set`, sync.frames.length <= 12, sync.frames.length);
      check(`${prefix} only fully contained native destinations`, sync.frames.every(({ bounds: b }) => b.left >= sync.viewport.left && b.top >= sync.viewport.top && b.left + b.width <= sync.viewport.left + sync.viewport.width && b.top + b.height <= sync.viewport.top + sync.viewport.height), sync);
      check(`${prefix} token colors resolve opaque/readable`, geometry.tiles[0].tileBackground !== 'rgba(0, 0, 0, 0)' && geometry.tiles[0].textColor !== geometry.tiles[0].tileBackground, geometry.tiles[0]);
      activation(0).dispatchEvent(new MouseEvent('mouseenter')); activation(0).focus(); await settle();
      check(`${prefix} no separate hover preview`, commands('show_task_gallery_window_preview').length === 0, commands('show_task_gallery_window_preview'));
    }

    await fresh(3);
    check('minimized card caption remains truthful', /Minimized/.test(tiles()[2].textContent));
    check('captured fallback is actual mocked native image', tiles()[1].querySelector('.task-gallery-captured-image')?.src === fixture.capturedTestImage());
    check('unavailable card has no fabricated image', !tiles()[2].querySelector('.task-gallery-captured-image') && /Preview unavailable/.test(tiles()[2].textContent));
    const captureImage = tiles()[1].querySelector('.task-gallery-captured-image');
    captureImage?.dispatchEvent(new Event('error')); await settle();
    check('failed capture visibly degrades without broken image', /Preview unavailable/.test(tiles()[1].textContent) && (!tiles()[1].querySelector('.task-gallery-captured-image') || getComputedStyle(tiles()[1].querySelector('.task-gallery-captured-image')).visibility === 'hidden'));
    const icon = tiles()[0].querySelector('.task-gallery-app-icon');
    icon?.dispatchEvent(new Event('error')); await settle();
    check('failed app icon does not erase app/window identity', /Alpha/.test(tiles()[0].textContent) && (!icon || getComputedStyle(icon).visibility === 'hidden'));
    await fresh(6);
    activation(0).focus(); await key(activation(0), 'ArrowDown');
    check('three-column ArrowDown moves one row', document.activeElement === activation(3), document.activeElement?.getAttribute('aria-label'));
    await key(activation(3), 'ArrowUp'); check('ArrowUp reverses row', document.activeElement === activation(0));
    await fresh(6, 396); activation(1).focus(); await key(activation(1), 'ArrowDown');
    check('narrow two-column ArrowDown', document.activeElement === activation(3));
    await fresh(4); activation(1).focus(); await key(activation(1), 'ArrowDown');
    check('short last row clamps ArrowDown to final card', document.activeElement === activation(3));

    await fresh(3); close(1).focus(); await settle(); const survivor = close(1);
    await fixture.snapshot([...fixture.rows, fixture.manyRows[3]]); await settle();
    check('3→4 switches Matrix and preserves Close focus', fixture.state.open.layout.mode === 'matrix' && document.activeElement === survivor);
    await fixture.snapshot(fixture.rows); await settle();
    check('4→3 switches Ribbon and preserves Close focus', fixture.state.open.layout.mode === 'ribbon' && document.activeElement === survivor);

    await fresh(6); await fixture.snapshot(fixture.manyRows); await settle();
    activation(0).focus(); await key(activation(0), 'End');
    await until(() => fixture.measure().tiles.at(-1).closeFullyWithinViewport, 'End scrolls final pair into viewport');
    check('6→30 End selects final card and scrolls visible', document.activeElement === activation(29) && viewport().scrollTop > 0, fixture.measure());
    await key(activation(29), 'Home');
    check('Home restores visible first card', document.activeElement === activation(0) && fixture.measure().tiles[0].closeFullyWithinViewport);
    const previousRequest = commands('sync_task_gallery_thumbnails').at(-1).args.args.requestId;
    viewport().scrollTop = 72; viewport().dispatchEvent(new Event('scroll')); await settle();
    await until(() => commands('sync_task_gallery_thumbnails').at(-1).args.args.requestId > previousRequest, 'scroll advances sync');
    check('scroll first clears native overlays', commands('sync_task_gallery_thumbnails').some(entry => entry.args.args.requestId > previousRequest && entry.args.args.frames.length === 0));
    const beforeResize = commands('sync_task_gallery_thumbnails').at(-1).args.args.requestId;
    window.dispatchEvent(new Event('resize')); await settle();
    check('resize advances thumbnail request', commands('sync_task_gallery_thumbnails').at(-1).args.args.requestId > beforeResize);

    await fresh(3); close(1).focus(); close(1).click(); close(1).click(); await settle();
    check('Close duplicates suppressed, no activation', commands('close_task_gallery_window').length === 1 && commands('activate_task_gallery_window').length === 0);
    fixture.settle(); await settle();
    check('accepted WM_CLOSE keeps all cards until authoritative snapshot', tiles().length === 3 && fixture.ledger.some(entry => entry.name === 'taskbar:refresh-windows'));
    await fixture.snapshot([fixture.rows[0], fixture.rows[2]]); await settle();
    check('authoritative snapshot alone removes closed card', tiles().length === 2 && !tiles().some(tile => tile.dataset.galleryHwnd === '102'));
    await fresh(3); close(0).focus(); close(0).click(); await settle(); fixture.settle(false); await settle();
    const status = document.querySelector('[role="status"]');
    check('close failure visible and actionable', status && /could not|failed/i.test(status.textContent) && status.getBoundingClientRect().width > 1 && status.getBoundingClientRect().height > 1, status?.textContent);
    check('close failure preserves focus/cards', document.activeElement === close(0) && tiles().length === 3);

    await fresh(3); const current = fixture.state.open;
    await fixture.snapshot(fixture.rows.slice(0, 2), current.revision - 1); await settle();
    check('stale same-nonce snapshot cannot remove cards', tiles().length === 3);
    fixture.state.deferSync = true;
    window.dispatchEvent(new Event('resize')); await until(() => fixture.pendingSync.length > 0, 'deferred sync ready');
    const stale = fixture.pendingSync.splice(0);
    fixture.state.deferSync = false; await fixture.open(false, fixture.rows); await settle();
    const poison = fixture.capturedTestImage('#ff00ff');
    stale.forEach(request => request.resolve({ ...fixture.syncResult(request.args), thumbnails: request.args.frames.map(frame => ({ hwnd: frame.hwnd, source: 'captured', imageDataUrl: poison })) }));
    await settle();
    check('late prior-session thumbnail cannot repaint', ![...document.querySelectorAll('.task-gallery-captured-image')].some(image => image.src === poison));
    activation(0).focus(); await key(activation(0), 'ContextMenu'); await settle();
    check('context overlay clears native thumbnail destinations', commands('sync_task_gallery_thumbnails').at(-1).args.args.frames.length === 0);
    await key(window, 'Escape'); await settle();
    check('Escape first dismisses context overlay', !document.querySelector('[role="menu"]') && document.querySelector('[role="dialog"]'));
    await key(window, 'Escape'); await settle();
    check('Escape hides gallery and clears resources', !document.querySelector('[role="dialog"]') && fixture.state.liveFrames.length === 0);
    await fixture.open(); await settle(); await fixture.unmount(); await settle();
    check('unmount cleans gallery and resources', !document.querySelector('[role="dialog"]') && fixture.state.liveFrames.length === 0);

    // Real BottomBar component enters the same production gallery through IPC.
    await fixture.mountGallery(); await fixture.bottomSnapshot(fixture.rows); await settle();
    fixture.ledger.length = 0;
    const capsule = document.querySelector('.task-capsule');
    check('production grouped BottomBar tile present', capsule);
    check('Ribbon tile echoes count/marks while preserving app label', capsule?.classList.contains('task-capsule-ribbon') && capsule.querySelectorAll('.task-ribbon-marks i').length === 3 && /Fixture Editor/.test(capsule.textContent));
    capsule?.click(); await until(() => document.querySelector('[role="dialog"]'), 'BottomBar opens gallery');
    check('BottomBar→native open→production gallery journey', commands('show_task_gallery').length === 1 && tiles().length === 3);
    activation(0).focus(); await key(activation(0), 'Enter');
    check('gallery Enter activates with captured session and minimize intent', commands('activate_task_gallery_window').at(-1)?.args.args.hwnd === '101' && commands('activate_task_gallery_window').at(-1)?.args.args.minimizeIfActive === true);
    const bottomNonce = fixture.state.open.nonce;
    close(1).focus(); close(1).click(); await settle();
    check('BottomBar journey Close carries same native session', commands('close_task_gallery_window').at(-1)?.args.args.nonce === bottomNonce && commands('close_task_gallery_window').at(-1)?.args.args.hwnd === '102');
    fixture.settle(); await settle();
    check('BottomBar close acceptance retains card', tiles().length === 3);
    await fixture.bottomSnapshot([fixture.rows[0], fixture.rows[2]]); await settle();
    await until(() => tiles().length === 2, 'BottomBar authoritative removal');
    check('BottomBar snapshot removes exact closed card', !tiles().some(tile => tile.dataset.galleryHwnd === '102'));
    await fixture.publish('task-gallery:open', { ...fixture.state.open, revision: fixture.state.open.revision - 1, windows: fixture.rows }); await settle();
    check('BottomBar journey stale snapshot cannot resurrect card', tiles().length === 2);
  } catch (error) { check('acceptance journey completed without harness/runtime exception', false, String(error)); }
  return { total: results.length, failed: results.filter(result => !result.pass), results };
};
