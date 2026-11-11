import { spawnSync } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { pngPixels } from './png-pixels.mjs';

// Visible production DOM/CSS; only host origin and CSS zoom are controlled.
// Hidden-glyph reference separates actual glyph paint from the tile gradient.
const session = 'taskbar-glyph-pixels-visible';
const executable = process.env.AGENT_BROWSER_EXECUTABLE ?? join(process.env.APPDATA, 'npm/node_modules/agent-browser/bin/agent-browser-win32-x64.exe');
const artifacts = resolve('test-results/task-gallery/glyph-pixels');
mkdirSync(artifacts, { recursive: true });
function browser(args) {
  const run = spawnSync(executable, ['--session', session, '--headed', ...args, '--json'], { encoding: 'utf8', timeout: 45000, maxBuffer: 16 * 1024 * 1024 });
  if (run.status !== 0) throw new Error(run.error?.message || run.stderr || run.stdout);
  const reply = JSON.parse(run.stdout);
  if (!reply.success) throw new Error(reply.error);
  return reply.data;
}
const evaluate = code => browser(['eval', '-b', Buffer.from(code).toString('base64')]).result;
const results = [];
try {
  browser(['open', 'http://127.0.0.1:4179/']);
  browser(['set', 'viewport', '800', '600']);
  browser(['wait', '--fn', 'Boolean(window.galleryFixture && document.querySelector(".task-button"))']);
  for (const theme of ['dark', 'light']) for (const zoom of [1, 1.25, 1.5]) for (const offset of [0, 0.5]) {
    const geometry = evaluate(`(async () => {
      const f = window.galleryFixture;
      f.configure(960, 600, '${theme}');
      await f.bottomSnapshot(Array.from({length:4}, (_,i) => ({...f.rows[0], hwnd:String(700+i), isActive:false, isMinimized:false})));
      document.body.style.zoom = '${zoom}';
      const host = document.querySelector('#bottom-bar-host');
      Object.assign(host.style, {position:'fixed', left:'${20 + offset}px', top:'${20 + offset}px', bottom:'auto', width:'600px', height:'32.4px'});
      const glyph = document.querySelector('.task-group-glyph'); glyph.style.visibility = '';
      document.activeElement?.blur();
      await document.fonts.ready;
      await new Promise(r => requestAnimationFrame(() => requestAnimationFrame(r)));
      await Promise.all(document.getAnimations().filter(a => a.effect?.getTiming().iterations !== Infinity).map(a => a.finished.catch(() => {})));
      const rect = el => { const r = el.getBoundingClientRect(); return {left:r.left, top:r.top, right:r.right, bottom:r.bottom, width:r.width, height:r.height}; };
      const color = el => { const s = getComputedStyle(el), c = document.createElement('canvas').getContext('2d'); c.fillStyle = el.tagName.toLowerCase() === 'rect' ? s.fill : s.backgroundColor; c.fillRect(0,0,1,1); return [...c.getImageData(0,0,1,1).data]; };
      return {glyph:rect(glyph), cells:[...glyph.querySelectorAll('i, rect')].map(el => ({...rect(el), rgba:color(el)})), viewportWidth:innerWidth, dpr:devicePixelRatio};
    })()`);
    const name = `${theme}-zoom${zoom}-offset${offset}`;
    const actualPath = resolve(artifacts, `${name}.png`);
    browser(['screenshot', actualPath]);
    evaluate(`(async()=>{document.querySelector('.task-group-glyph').style.visibility='hidden';await new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r)));})()`);
    const referencePath = resolve(artifacts, `${name}-background.png`);
    browser(['screenshot', referencePath]);
    const actual = pngPixels(actualPath), reference = pngPixels(referencePath);
    const scale = actual.width / geometry.viewportWidth;
    const cells = geometry.cells.map(cell => {
      const rgb = cell.rgba.slice(0, 3), opacity = cell.rgba[3] / 255;
      if (!opacity) throw new Error('Cannot resolve visible glyph foreground color');
      const samples = [];
      for (let y = Math.floor(cell.top * scale) - 1; y < Math.ceil(cell.bottom * scale) + 1; y++) {
        for (let x = Math.floor(cell.left * scale) - 1; x < Math.ceil(cell.right * scale) + 1; x++) {
          const p = actual.pixel(x, y), bg = reference.pixel(x, y);
          const denominator = rgb.reduce((sum, v, i) => sum + (v - bg[i]) ** 2, 0);
          const alpha = denominator ? p.reduce((sum, v, i) => sum + (v - bg[i]) * (rgb[i] - bg[i]), 0) / denominator / opacity : 0;
          samples.push({x, y, alpha});
        }
      }
      // Normalize intentional per-cell opacity/accent differences; compare shape,
      // not color. A crisp cell has uniform positive coverage, AA adds levels.
      const peak = Math.max(...samples.map(p => p.alpha));
      const painted = samples.filter(p => p.alpha > peak * 0.08 && p.alpha > 0.03).map(p => [p.x, p.y]);
      const blended = samples.filter(p => p.alpha > peak * 0.08 && p.alpha < peak * 0.92 && p.alpha > 0.03);
      const xs = painted.map(p => p[0]), ys = painted.map(p => p[1]);
      const left = Math.min(...xs), top = Math.min(...ys);
      return {width:Math.max(...xs)-left+1, height:Math.max(...ys)-top+1, paintedPixels:painted.length,
        mask:painted.map(([x,y]) => `${x-left},${y-top}`).join(';'), blended};
    });
    const matching = cells.length === 4 && cells.every(cell => cell.mask === cells[0].mask);
    const sharpSquares = cells.every(cell => cell.width === cell.height && cell.paintedPixels === cell.width * cell.height && cell.blended.length === 0);
    results.push({name, pass:matching && sharpSquares, matching, sharpSquares, geometry, cells, actualPath, referencePath});
  }
} catch (error) {
  results.push({name:'visible glyph pixel runner completed', pass:false, error:String(error)});
} finally {
  try { browser(['close']); } catch {}
}
const report = {results, failed:results.filter(r => !r.pass), evidenceBoundary:'Production Matrix glyph, PNG foreground-minus-hidden-reference pixel masks. Controlled fractional host origins, CSS zoom 1/1.25/1.5, recorded browser DPR. Not native Windows DPI or WebView2 certification.'};
writeFileSync(resolve(artifacts, 'glyph-pixels.json'), JSON.stringify(report, null, 2));
console.log(JSON.stringify({total:results.length, passed:results.filter(r=>r.pass).length, failed:report.failed.map(r=>({name:r.name,error:r.error,matching:r.matching,sharpSquares:r.sharpSquares,cells:r.cells?.map(c=>({width:c.width,height:c.height,blended:c.blended.length}))})), artifacts}, null, 2));
process.exitCode = report.failed.length ? 1 : 0;
