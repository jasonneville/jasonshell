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
  for (const count of [2, 3, 4, 6, 30]) for (let index = 0; index < count; index++) {
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
    const leadingGap = measurement => measurement.label.left - measurement.icon.right;
    const ribbonGaps = [2, 3].map(count => leadingGap(measure(document.querySelector(`[data-window-count="${count}"] .task-capsule`))));
    for (const count of [2, 3, 4, 6, 30]) {
      const group = document.querySelector(`[data-window-count="${count}"]`);
      const button = group?.querySelector('.task-capsule');
      check(`${theme}: ${count}-window group rendered as capsule under strip pressure`, Boolean(button), group?.className);
      if (!button) continue;
      const actual = measure(button);
      check(`${theme}: ${count}-window icon-to-title gap matches standalone and Ribbon`,
        close(leadingGap(actual), leadingGap(standard))
        && ribbonGaps.every(gap => close(leadingGap(actual), gap)),
        { actualGap: leadingGap(actual), standaloneGap: leadingGap(standard), ribbonGaps });
      check(`${theme}: ${count}-window icon vertical center matches standalone`, close(actual.icon.centerY, standard.icon.centerY), { standard, actual });
      // Same font + same Range text-box bottom establishes matching text baseline
      // without adding an inline probe that could perturb production flex layout.
      check(`${theme}: ${count}-window title vertical center and baseline match standalone`,
        actual.font === standard.font && actual.lineHeight === standard.lineHeight
        && close(actual.text.centerY, standard.text.centerY) && close(actual.text.bottom, standard.text.bottom), { standard, actual });
      check(`${theme}: ${count}-window tile keeps standalone face height and 160px cap`,
        close(actual.button.top, standard.button.top) && close(actual.button.height, standard.button.height) && actual.button.width <= 160.5, { standard, actual });
      const countNode = button.querySelector('.task-count');
      const marks = [...button.querySelectorAll(count < 4 ? '.task-ribbon-marks i' : '.task-group-glyph rect')];
      check(`${theme}: ${count}-window count and group marks remain visible within tile`,
        countNode?.textContent === String(count) && marks.length === (count < 4 ? count : 4)
        && [countNode, ...marks].every(node => {
          const r = rect(node);
          return r.width > 0 && r.height > 0 && r.top >= actual.button.top && r.bottom <= actual.button.bottom
            && r.left >= actual.button.left && r.right <= actual.button.right;
         }), { count: countNode?.textContent, marks: marks.map(rect), button: actual.button });
        if (count >= 4) {
          const labelNode = button.querySelector('.task-label');
          const glyphNode = button.querySelector('.task-group-glyph');
          check(`${theme}: ${count}-window Matrix DOM order is icon, title, glyph, count`,
            button.querySelector('.task-icon').nextElementSibling === labelNode
            && labelNode.nextElementSibling === glyphNode && glyphNode.nextElementSibling === countNode,
            [...button.children].map(node => node.className));
         const glyph = rect(button.querySelector('.task-group-glyph'));
         const cells = marks.map(rect);
         check(`${theme}: ${count}-window Matrix is a centered compact 8x8 collection glyph`,
           close(glyph.width, 8) && close(glyph.height, 8)
           && close(glyph.centerY, actual.icon.centerY), { glyph, icon: actual.icon, cells });
         check(`${theme}: ${count}-window Matrix cells stay 3x3 with equal 2px row and column gaps`,
           cells.length === 4 && cells.every(cell => close(cell.width, 3) && close(cell.height, 3))
           && close(cells[0].top, cells[1].top) && close(cells[2].top, cells[3].top)
           && close(cells[0].left, cells[2].left) && close(cells[1].left, cells[3].left)
           && close(cells[1].left - cells[0].right, 2)
           && close(cells[3].left - cells[2].right, 2)
           && close(cells[2].top - cells[0].bottom, 2)
           && close(cells[3].top - cells[1].bottom, 2), { glyph, cells });
       }
    }
  }
  const report = { results, failed: results.filter(result => !result.pass),
    evidenceBoundary: 'Headed browser, production BottomBar DOM/CSS, actual measured font/icon geometry; mock task snapshots only. No native shell.' };
  window.bottomBarAlignmentReport = report;
  document.querySelector('#summary').textContent = JSON.stringify(report, null, 2);
  return report;
})()
