// Uses the mounted production gallery and the existing mocked event bridge.
window.checkGalleryTightSeams = async () => {
  const results = [];
  const check = (name, pass, actual) => results.push({ name, pass, actual });
  const near = (name, actual, expected) => check(name, Math.abs(actual - expected) < 0.16, { actual, expected });
  const css = (el, pseudo) => getComputedStyle(el, pseudo);
  const rect = el => el.getBoundingClientRect();
  const frame = () => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve)));
  const fixture = window.galleryFixture;
  const host = document.querySelector('#surface');
  for (const theme of ['base-dark', 'base-light']) {
    document.documentElement.dataset.theme = theme;
    for (const height of [24, 32.4]) for (const scenario of [{ width: 960, rows: fixture.rows }, { width: 240, rows: fixture.rows }, { width: 240, rows: fixture.manyRows }]) {
      host.style.height = `${height}px`; host.style.width = `${scenario.width}px`;
      await fixture.open(); await fixture.snapshot(scenario.rows); await frame();
      const panel = document.querySelector('.task-gallery-panel');
      const strip = document.querySelector('.task-gallery-strip');
      const tiles = [...document.querySelectorAll('.task-gallery-tile')];
      await Promise.all(tiles.flatMap(tile => tile.getAnimations().map(animation => animation.finished.catch(() => {}))));
      const p = rect(panel), s = rect(strip), t = rect(tiles[0]);
      const prefix = `${theme}/${height}/${scenario.width}/${tiles.length}`;
      near(`${prefix} native panel height`, p.height, height);
      near(`${prefix} vertical top inset`, s.top - p.top, 2);
      near(`${prefix} vertical bottom inset`, p.bottom - s.bottom, 2);
      near(`${prefix} horizontal left inset`, s.left - p.left, 4);
      near(`${prefix} horizontal right inset`, p.right - s.right, 4);
      near(`${prefix} intertile gap`, rect(tiles[1]).left - t.right, 3);
      near(`${prefix} tile corner`, parseFloat(css(tiles[0]).borderTopLeftRadius), 2);
      check(`${prefix} firm tile border`, parseFloat(css(tiles[0]).borderBottomWidth) >= 1 && css(tiles[0]).borderBottomStyle === 'solid', css(tiles[0]).borderBottom);
      check(`${prefix} beveled face`, css(tiles[0]).backgroundImage !== 'none' && css(tiles[0]).boxShadow.includes('inset'), { background: css(tiles[0]).backgroundImage, shadow: css(tiles[0]).boxShadow });
      const accent = css(tiles[0], '::before');
      check(`${prefix} no painted active stripe`, ['none', 'normal'].includes(accent.content) || accent.display === 'none' || accent.visibility === 'hidden' || parseFloat(accent.opacity) === 0 || parseFloat(accent.width) === 0, { content: accent.content, width: accent.width });
      check(`${prefix} active fill retained`, css(tiles[0]).backgroundColor !== css(tiles[1]).backgroundColor && css(tiles[0]).backgroundColor !== 'rgba(0, 0, 0, 0)', css(tiles[0]).backgroundColor);
      check(`${prefix} active border retained`, css(tiles[0]).borderBottomColor !== css(tiles[1]).borderBottomColor && parseFloat(css(tiles[0]).borderBottomWidth) >= 1, css(tiles[0]).borderBottom);
      check(`${prefix} horizontal scroll preserved`, css(strip).overflowX === 'auto' && (tiles.length < 30 || strip.scrollWidth > strip.clientWidth), { client: strip.clientWidth, scroll: strip.scrollWidth });
      for (const tile of tiles) {
        const x = tile.querySelector('.task-gallery-close');
        const icon = tile.querySelector('img');
        const xr = rect(x), ir = rect(icon), tr = rect(tile);
        near(`${prefix}/${tile.dataset.galleryHwnd} close width retained`, xr.width, 24);
        check(`${prefix}/${tile.dataset.galleryHwnd} close vertically fits`, xr.height > 0 && xr.top >= Math.max(tr.top, s.top) - 0.16 && xr.bottom <= Math.min(tr.bottom, s.bottom) + 0.16, { closeHeight: xr.height, tileHeight: tr.height, stripHeight: s.height, closeTop: xr.top, stripTop: s.top });
        check(`${prefix}/${tile.dataset.galleryHwnd} icon vertically fits`, icon.complete && icon.naturalWidth > 0 && ir.top >= s.top - 0.16 && ir.bottom <= s.bottom + 0.16, { iconHeight: ir.height, stripHeight: s.height });
      }
      const title = tiles.at(-1).querySelector('.task-gallery-tab-title');
      check(`${prefix} long title ellipsis`, css(title).textOverflow === 'ellipsis' && title.scrollWidth > title.clientWidth, { client: title.clientWidth, scroll: title.scrollWidth });
    }
  }
  return { total: results.length, failed: results.filter(result => !result.pass), results };
};
