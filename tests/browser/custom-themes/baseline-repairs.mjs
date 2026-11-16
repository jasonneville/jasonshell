import assert from 'node:assert/strict';
import { writeFile } from 'node:fs/promises';
import { connectPage } from './cdp.mjs';

const endpoint = process.argv[2];
if (!endpoint) throw new Error('Usage: node tests/browser/custom-themes/baseline-repairs.mjs http://127.0.0.1:CDP_PORT');
const pages = [];
const report = { checks: [], menus: [], speech: {}, tiles: {}, screenshots: [] };
async function screenshot(page, name) {
  const { data } = await page.send('Page.captureScreenshot', { format: 'png', captureBeyondViewport: true });
  const path = `C:/Users/jnev1/AppData/Local/Temp/opencode/${name}.png`;
  await writeFile(path, Buffer.from(data, 'base64')); report.screenshots.push(path);
}
try {
  const menu = await connectPage(endpoint, 'http://127.0.0.1:4186/?surface=baseline'); pages.push(menu);
  await menu.wait('!!window.themeFixture && !!document.querySelector(".mic-shell")');
  await menu.evaluate('Promise.all(document.getAnimations().map(animation=>animation.finished))');
  for (const theme of ['base-dark', 'base-light']) for (const reduced of [false, true]) {
    const metrics = await menu.evaluate(`(async () => {
      document.documentElement.dataset.theme=${JSON.stringify(theme)};
      delete document.documentElement.dataset.customTheme;
      document.documentElement.dataset.shellTransparency=${JSON.stringify(reduced ? 'reduced' : 'normal')};
      // Preference changes legitimately transition background paint. Flush style
      // and await finite browser animations before sampling the settled result.
      getComputedStyle(document.querySelector('[role=menu]')).backgroundColor;
      await new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve)));
      await Promise.all(document.getAnimations().filter(animation=>animation.effect?.getTiming().iterations !== Infinity).map(animation=>animation.finished));
      return [...document.querySelectorAll('[role=menu]')].map(node=>{
        const css=getComputedStyle(node),canvas=document.createElement('canvas');canvas.width=canvas.height=1;
        const ctx=canvas.getContext('2d');ctx.fillStyle=css.backgroundColor;ctx.fillRect(0,0,1,1);
        return {label:node.getAttribute('aria-label'),background:css.backgroundColor,alpha:ctx.getImageData(0,0,1,1).data[3],blur:css.backdropFilter,opacity:css.opacity,borderRadius:css.borderRadius};
      });
    })()`);
    const generic = metrics.find(row => row.label === 'Generic baseline menu');
    const overlay = metrics.find(row => row.label === 'Bar baseline menu');
    assert.equal(overlay.alpha, 255); assert.equal(overlay.opacity, '1');
    if (reduced) {
      assert.equal(generic.alpha, 255); assert.equal(generic.blur, 'none'); assert.equal(overlay.blur, 'none');
    } else {
      assert.ok(generic.alpha > 0 && generic.alpha < 255, JSON.stringify(generic));
      assert.equal(generic.blur, 'blur(8px)'); assert.equal(overlay.blur, 'blur(8px)');
    }
    assert.equal(generic.borderRadius, '12px');
    report.menus.push({ theme, reduced, metrics });
  }
  report.checks.push('generic menu translucent+8px blur; bar overlay alpha1; reduced transparency alpha1/no blur in dark/light');
  const measureSpeech = `(() => { const shell=document.querySelector('.mic-shell'),glyph=document.querySelector('.mic-glyph'),r=shell.getBoundingClientRect(),g=glyph.getBoundingClientRect();return {width:r.width,height:r.height,radius:getComputedStyle(shell).borderRadius,glyphWidth:g.width,glyphHeight:g.height}; })()`;
  report.speech.quiet = await menu.evaluate(measureSpeech);
  assert.deepEqual(report.speech.quiet, { width: 40, height: 40, radius: '50%', glyphWidth: 24, glyphHeight: 24 });
  await menu.evaluate('themeFixture.speechLevel(.5)');
  await menu.wait('Number(document.querySelector(".mic-glyph").style.getPropertyValue("--mic-level")) > .9');
  report.speech.speaking = await menu.evaluate(measureSpeech);
  assert.deepEqual(report.speech.speaking, report.speech.quiet);
  report.checks.push('production speech shell stays40x40 quiet/speaking; artwork24x24 and circle unchanged');
  await screenshot(menu, 'custom-theme-baseline-menu-speech');

  const gallery = await connectPage(endpoint, 'http://127.0.0.1:4179/'); pages.push(gallery);
  await gallery.wait('!!window.galleryFixture && !!document.querySelector(".task-gallery-tile") && !!document.querySelector(".bottom-bar .task-button")');
  report.tiles = await gallery.evaluate(`(() => {
    const result={};
    for(const selector of ['.bottom-bar','.bottom-bar .process-manager-button','.bottom-bar .quick-launch-button','.bottom-bar .task-button','.task-gallery-tile','.task-gallery-panel']){
      result[selector]=[...document.querySelectorAll(selector)].map(node=>{const css=getComputedStyle(node),r=node.getBoundingClientRect();return {image:css.backgroundImage,width:r.width,height:r.height,radius:css.borderRadius,border:css.borderTopWidth,shadow:css.boxShadow};});
    }return result;
  })()`);
  for (const [selector, rows] of Object.entries(report.tiles)) {
    assert.ok(rows.length > 0, `fixture must mount ${selector}`);
    for (const row of rows) {
      if (selector === '.task-gallery-panel') assert.match(row.image, /^linear-gradient/);
      else assert.equal(row.image, 'none', selector);
    }
  }
  for (const row of report.tiles['.task-gallery-tile']) {
    assert.equal(row.width, 180); assert.equal(row.height, 136); assert.equal(row.radius, '2px');
    assert.match(row.shadow, /inset/);
  }
  report.checks.push('production bottom controls+task tiles and gallery tiles flat; gallery180x136 bevel geometry and approved outer gradient preserved');
  await screenshot(gallery, 'custom-theme-baseline-flat-tiles');
  assert.ok(pages.every(page => page.errors.length === 0), JSON.stringify(pages.flatMap(page => page.errors)));
  console.log(JSON.stringify(report, null, 2));
} finally {
  await writeFile('C:/Users/jnev1/AppData/Local/Temp/opencode/custom-theme-baseline-render-report.json', JSON.stringify(report, null, 2));
  for (const page of pages.reverse()) await page.close();
}
