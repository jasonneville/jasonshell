// Production BottomBar + real browser layout. IPC rows/icons are synthetic.
// No style overrides, native shell, or hidden baseline probes are introduced.
(async () => {
  const fixture = window.galleryFixture;
  if (!fixture) throw new Error('Gallery fixture unavailable');
  const results = [];
  const check = (name, pass, actual) => results.push({ name, pass: Boolean(pass), actual });
  const close = (a, b) => Math.abs(a - b) <= 0.5;
  const settle = async () => {
    await document.fonts.ready;
    await new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve)));
  };
  const seed = { ...fixture.rows[1], isActive: false, isMinimized: false, activityState: 'idle', attentionState: 'idle', toastCount: 0 };
  const rows = [{ ...seed, hwnd: '501', processName: 'Single', processId: 501, title: 'Single' }];
  for (const count of [2, 3, 4]) for (let index = 0; index < count; index++) {
    rows.push({ ...seed, hwnd: String(count * 1000 + index), processName: `Group ${count}`, processId: count * 100, title: `Group ${count} window ${index + 1}` });
  }
  const rect = element => {
    const r = element.getBoundingClientRect();
    return { top: r.top, bottom: r.bottom, left: r.left, right: r.right, width: r.width, height: r.height, centerY: (r.top + r.bottom) / 2 };
  };
  const measure = button => {
    const label = button.querySelector('.task-label');
    const range = document.createRange(); range.selectNodeContents(label);
    const text = rect(range);
    const style = getComputedStyle(label);
    return { button: rect(button), icon: rect(button.querySelector('.task-icon')), label: rect(label), text,
      font: style.font, lineHeight: style.lineHeight, paddingBottom: getComputedStyle(button).paddingBottom };
  };
  for (const theme of ['dark', 'light']) {
    fixture.configure(960, 600, theme);
    await fixture.bottomSnapshot(rows); await settle();
    const single = document.querySelector('[data-window-count="1"] .task-button');
    if (!single) throw new Error('Standalone production task button missing');
    const standard = measure(single);
    check(`${theme}: standalone tile keeps normal icon and face geometry`,
      standard.icon.width > 0 && close(standard.icon.width, standard.icon.height) && close(standard.icon.centerY, standard.button.centerY)
      && standard.button.height > standard.icon.height && standard.button.width <= 160.5, standard);
    for (const count of [2, 3, 4]) {
      const group = document.querySelector(`[data-window-count="${count}"]`);
      const button = group?.querySelector('.task-capsule');
      check(`${theme}: ${count}-window group rendered as capsule under strip pressure`, Boolean(button), group?.className);
      if (!button) continue;
      const actual = measure(button);
      check(`${theme}: ${count}-window icon vertical center matches standalone`, close(actual.icon.centerY, standard.icon.centerY), { standard, actual });
      // Same font + same Range text-box bottom establishes matching text baseline
      // without adding an inline probe that could perturb production flex layout.
      check(`${theme}: ${count}-window title vertical center and baseline match standalone`,
        actual.font === standard.font && actual.lineHeight === standard.lineHeight
        && close(actual.text.centerY, standard.text.centerY) && close(actual.text.bottom, standard.text.bottom), { standard, actual });
      check(`${theme}: ${count}-window tile keeps standalone face height and 160px cap`,
        close(actual.button.top, standard.button.top) && close(actual.button.height, standard.button.height) && actual.button.width <= 160.5, { standard, actual });
      const countNode = button.querySelector('.task-count');
      const marks = [...button.querySelectorAll(count < 4 ? '.task-ribbon-marks i' : '.task-group-glyph i')];
      check(`${theme}: ${count}-window count and group marks remain visible within tile`,
        countNode?.textContent === String(count) && marks.length === (count < 4 ? count : 4)
        && [countNode, ...marks].every(node => {
          const r = rect(node);
          return r.width > 0 && r.height > 0 && r.top >= actual.button.top && r.bottom <= actual.button.bottom
            && r.left >= actual.button.left && r.right <= actual.button.right;
        }), { count: countNode?.textContent, marks: marks.map(rect), button: actual.button });
    }
  }
  const report = { results, failed: results.filter(result => !result.pass),
    evidenceBoundary: 'Headed browser, production BottomBar DOM/CSS, actual measured font/icon geometry; mock task snapshots only. No native shell.' };
  window.bottomBarAlignmentReport = report;
  document.querySelector('#summary').textContent = JSON.stringify(report, null, 2);
  return report;
})()
