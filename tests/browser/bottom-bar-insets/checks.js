// Browser geometry, not jsdom/regex. Run window.checkBottomBarInsets() via agent-browser.
window.checkBottomBarInsets = async () => {
  const results = [];
  const near = (name, actual, expected) => results.push({ name, actual, expected, pass: Math.abs(actual - expected) < 0.15 });
  const check = (name, pass, actual) => results.push({ name, pass, actual });
  const host = document.querySelector('#host');
  const bar = document.querySelector('.bottom-bar');
  const source = document.querySelector('#source');
  const groups = [...document.querySelectorAll('.task-group')];
  const rect = el => el.getBoundingClientRect();
  const css = (el, pseudo) => getComputedStyle(el, pseudo);
  for (const theme of ['base-dark', 'base-light']) {
    document.documentElement.dataset.theme = theme;
    for (const height of [24, 32.4, 48]) {
      host.style.height = `${height}px`;
      bar.style.setProperty('--bottom-bar-height-logical', `${height}px`);
      await new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve)));
      const prefix = `${theme}/${height}`;
      const b = rect(bar), g = rect(groups[0]);
      near(`${prefix} native height preserved`, b.height, height);
      near(`${prefix} vertical inset top padding`, g.top - b.top - parseFloat(css(bar).borderTopWidth), 2);
      near(`${prefix} vertical inset bottom`, b.bottom - g.bottom, 2);
      near(`${prefix} intergroup gutter`, rect(groups[1]).left - g.right, 3);
      near(`${prefix} direct tile gutter`, rect(source.nextElementSibling).left - rect(source).right, 3);
      near(`${prefix} outer left inset`, rect(document.querySelector('.quick-launch-button')).left - b.left, 4);
      near(`${prefix} outer right inset`, b.right - rect(document.querySelector('.process-manager-button')).right, 4);
      near(`${prefix} tile corners`, parseFloat(css(source).borderTopLeftRadius), 2);
      const accent = css(groups[0], '::before');
      const accentNotPainted = ['none', 'normal'].includes(accent.content) || accent.display === 'none' || accent.visibility === 'hidden' || parseFloat(accent.opacity) === 0 || parseFloat(accent.width) === 0;
      check(`${prefix} no painted active left stripe`, accentNotPainted, { content: accent.content, display: accent.display, width: accent.width });
      check(`${prefix} active fill retained`, css(source).backgroundColor !== css(source.nextElementSibling).backgroundColor && css(source).backgroundColor !== 'rgba(0, 0, 0, 0)', css(source).backgroundColor);
      check(`${prefix} active border retained`, parseFloat(css(source).borderBottomWidth) >= 1 && css(source).borderBottomStyle === 'solid' && css(source).borderBottomColor !== css(source.nextElementSibling).borderBottomColor && css(source).borderBottomColor !== 'rgba(0, 0, 0, 0)', css(source).borderBottomColor);
      check(`${prefix} fine tile border`, parseFloat(css(source).borderBottomWidth) >= 1, css(source).borderBottomWidth);
      check(`${prefix} beveled face`, css(source).boxShadow.includes('inset'), css(source).boxShadow);
      check(`${prefix} direct/capsule widths bounded`, g.width <= 320.15 && rect(groups[1]).width <= 160.15, [g.width, rect(groups[1]).width]);
      near(`${prefix} equal direct widths`, rect(source).width, rect(source.nextElementSibling).width);
      const label = source.querySelector('.task-label');
      check(`${prefix} long label ellipsis`, css(label).textOverflow === 'ellipsis' && label.scrollWidth > label.clientWidth, css(label).textOverflow);
      for (const icon of document.querySelectorAll('.task-icon')) {
        const i = rect(icon), tile = rect(icon.parentElement);
        check(`${prefix} icon fits ${icon.parentElement.textContent.trim().slice(0, 12)}`, icon.complete && icon.naturalWidth > 0 && i.top >= tile.top && i.bottom <= tile.bottom && i.height > 0, { iconHeight: i.height, tileHeight: tile.height });
      }
      check(`${prefix} toast cue on opaque capsule face`, css(groups[1].querySelector('button')).boxShadow.includes('inset') && css(groups[1].querySelector('button')).boxShadow !== 'none', css(groups[1].querySelector('button')).boxShadow);
      const busy = css(groups[1], '::after');
      check(`${prefix} busy cue stays visible`, parseFloat(busy.height) >= 2 && busy.content !== 'none' && busy.pointerEvents === 'none', busy.height);
      check(`${prefix} drop target frame`, css(groups[2]).outlineStyle !== 'none' && parseFloat(css(groups[2]).outlineWidth) >= 1, css(groups[2]).outline);
      groups[2].classList.add('task-group-dragging');
      await new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve)));
      check(`${prefix} dragging cue`, css(groups[2]).cursor === 'grabbing' && parseFloat(css(groups[2]).opacity) < 0.6 && css(groups[2]).transitionDuration.split(',').every(v => parseFloat(v) <= (matchMedia('(prefers-reduced-motion: reduce)').matches ? 0.001 : 0)), css(groups[2]).opacity);
      groups[2].classList.remove('task-group-dragging');
      check(`${prefix} disabled cue`, source.nextElementSibling.disabled && parseFloat(css(source.nextElementSibling).opacity) < 1, css(source.nextElementSibling).opacity);
      check(`${prefix} minimized cue`, parseFloat(css(groups[2]).opacity) < 1, css(groups[2]).opacity);
      check(`${prefix} count remains within capsule`, rect(document.querySelector('.task-count')).bottom <= rect(groups[1]).bottom, rect(document.querySelector('.task-count')).height);
      // Previous iteration removes attention and theme changes animate token colors.
      // Compare settled states on both sides, not a transient pre-toggle frame.
      await Promise.all(source.getAnimations().map(animation => animation.finished.catch(() => {})));
      const normalChrome = { background: css(source).background, border: css(source).border, shadow: css(source).boxShadow };
      source.classList.add('task-button-preview-connected');
      await Promise.all(source.getAnimations().map(animation => animation.finished.catch(() => {})));
      const s = rect(source);
      bar.style.setProperty('--preview-connector-left', `${s.left - b.left}px`);
      bar.style.setProperty('--preview-connector-width', `${s.width}px`);
      bar.style.setProperty('--preview-connector-color', css(source).backgroundColor);
      await new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve)));
      const bridge = css(bar, '::after');
      check(`${prefix} floating preview paints no bridge`, ['none', 'normal'].includes(bridge.content) || bridge.display === 'none', { content: bridge.content, display: bridge.display });
      check(`${prefix} preview state preserves source background`, css(source).background === normalChrome.background, css(source).background);
      check(`${prefix} preview state preserves source border`, css(source).border === normalChrome.border, css(source).border);
      check(`${prefix} preview state preserves source bevel/accent`, css(source).boxShadow === normalChrome.shadow, css(source).boxShadow);
      check(`${prefix} no adjoining group frame`, parseFloat(css(groups[0]).borderTopWidth) === 0 && parseFloat(css(groups[0]).borderBottomWidth) === 0 && parseFloat(css(groups[0]).gap) === 3, css(groups[0]).border);
      check(`${prefix} source retains visible top edge`, css(source).borderTopColor !== 'rgba(0, 0, 0, 0)' && parseFloat(css(source).borderTopWidth) >= 1, css(source).borderTopColor);
      check(`${prefix} neighbor not connected`, !source.nextElementSibling.classList.contains('task-button-preview-connected') && css(source.nextElementSibling).borderTopColor !== 'rgba(0, 0, 0, 0)');
      source.classList.add('task-window-attention');
      await Promise.all(source.getAnimations().map(animation => animation.finished.catch(() => {})));
      check(`${prefix} connected attention retained`, css(source).boxShadow.includes('255, 213, 79'), css(source).boxShadow);
      const previewAttentionShadow = css(source).boxShadow;
      source.classList.remove('task-button-preview-connected');
      await Promise.all(source.getAnimations().map(animation => animation.finished.catch(() => {})));
      check(`${prefix} preview state preserves active attention chrome`, css(source).boxShadow === previewAttentionShadow, css(source).boxShadow);
      source.classList.remove('task-window-attention');
      source.classList.remove('task-button-preview-connected');
    }
  }
  host.style.width = '280px';
  await new Promise(requestAnimationFrame);
  check('narrow viewport utility endcap stays inside host', rect(document.querySelector('.process-manager-button')).right <= rect(host).right + 0.15);
  check('overflow stays clipped', css(document.querySelector('.task-strip')).overflowX === 'hidden');
  host.style.width = '960px';
  return { total: results.length, failed: results.filter(r => !r.pass), results };
};

window.checkBottomBarMotionAndFocus = () => {
  const busyEl = document.querySelector('.task-group-busy');
  const busy = getComputedStyle(busyEl, '::after');
  const reduced = matchMedia('(prefers-reduced-motion: reduce)').matches;
  const focused = document.activeElement;
  const focus = getComputedStyle(focused);
  const checks = [
    { name: 'reduced motion requested', pass: reduced },
    { name: 'reduced motion busy cue static', pass: busy.animationName === 'none' && busy.transform === 'none' && Math.abs(parseFloat(busy.width) - busyEl.getBoundingClientRect().width) < 0.15, actual: { animation: busy.animationName, transform: busy.transform, width: busy.width } },
    { name: 'keyboard focus visible', pass: focused.matches('button:focus-visible') && focus.outlineStyle !== 'none' && parseFloat(focus.outlineWidth) >= 2, actual: { label: focused.getAttribute('aria-label') || focused.textContent.trim(), outline: focus.outline } }
  ];
  return { total: checks.length, failed: checks.filter(c => !c.pass), results: checks };
};
