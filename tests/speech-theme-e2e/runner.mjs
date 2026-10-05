// Uses real main.ts/App/Settings/indicator and browser channels. Instrumentation
// records actual GPU uniform input; no synthetic theme setter replaces Settings.
const params = new URL(location.href).searchParams;
if (params.has('surface')) {
  if (params.has('sampleclock')) {
    // Opt-in test timing installed BEFORE any production module mounts.
    let now = 0;
    let sequence = 0;
    let hidden = false;
    const queued = new Map();
    const reduced = Object.assign(new EventTarget(), { matches: false, media: '(prefers-reduced-motion: reduce)' });
    const match = window.matchMedia.bind(window);
    window.matchMedia = (query) => query === reduced.media ? reduced : match(query);
    Object.defineProperty(performance, 'now', { configurable: true, value: () => now });
    Object.defineProperty(document, 'hidden', { configurable: true, get: () => hidden });
    Object.defineProperty(document, 'visibilityState', { configurable: true, get: () => hidden ? 'hidden' : 'visible' });
    window.requestAnimationFrame = (callback) => { const id = ++sequence; queued.set(id, callback); return id; };
    window.cancelAnimationFrame = (id) => queued.delete(id);
    window.__fillSampling = {
      async flush() { const { tick } = await import('svelte'); await tick(); await tick(); },
      async step(ms) {
        now += ms;
        // Newly queued callbacks belong to the NEXT step; cancellation still works.
        for (const [id, callback] of [...queued]) {
          if (!queued.delete(id)) continue;
          callback(now);
        }
        await this.flush();
      },
      async reduce(value) { reduced.matches = value; reduced.dispatchEvent(new Event('change')); await this.flush(); },
      async visibility(value) { hidden = value; document.dispatchEvent(new Event('visibilitychange')); await this.flush(); },
      read() {
        const shell = document.querySelector('.mic-shell');
        const glyph = document.querySelector('.mic-glyph');
        const style = getComputedStyle(shell);
        return { now, level: Number(glyph.style.getPropertyValue('--mic-level')), queuedRAF: queued.size,
          hidden, reduced: reduced.matches, shell: [shell.offsetWidth, shell.offsetHeight],
          glyph: [glyph.offsetWidth, glyph.offsetHeight], border: style.borderWidth, shadow: style.boxShadow };
      }
    };
  }
  window.__speechThemeNativeHidden = document.hidden;
  if (params.has('visible')) {
    // OpenChamber's capture panel can report hidden for an on-screen iframe.
    // Explicit test affordance: production still uses its normal visibility gate.
    Object.defineProperty(document, 'hidden', { configurable: true, get: () => false });
    Object.defineProperty(document, 'visibilityState', { configurable: true, get: () => 'visible' });
  }
  window.__speechThemeGPU = null;
  const locations = new WeakMap();
  const prototype = WebGLRenderingContext.prototype;
  const getLocation = prototype.getUniformLocation;
  prototype.getUniformLocation = function(program, name) {
    const value = getLocation.call(this, program, name);
    if (value) locations.set(value, name);
    return value;
  };
  const setColor = prototype.uniform3f;
  prototype.uniform3f = function(location, r, g, b) {
    if (locations.get(location) === 'u_color') window.__speechThemeGPU = [r, g, b];
    return setColor.call(this, location, r, g, b);
  };
  if (params.has('reduce')) {
    const match = window.matchMedia.bind(window);
    const reduced = Object.assign(new EventTarget(), { matches: true, media: '(prefers-reduced-motion: reduce)' });
    window.matchMedia = (query) => query === reduced.media ? reduced : match(query);
  }
  if (params.has('fallback')) {
    const getContext = HTMLCanvasElement.prototype.getContext;
    HTMLCanvasElement.prototype.getContext = function(type, options) {
      return type === 'webgl' ? null : getContext.call(this, type, options);
    };
  }
  await import('../../src/main.ts');
  if (params.has('volume')) {
    setTimeout(() => window.__speechThemeVoice(Number(params.get('volume'))), 400);
  }
} else if (params.has('sampling')) {
  document.body.style.cssText = 'background:#eef2f6;color:#111827;font:16px system-ui';
  localStorage.setItem('jasonshell.theme', 'dracula');
  document.body.innerHTML = `<h1>Actual mic: sampled presentation animation</h1>
    <p>EMULATED performance.now, queued RAF, document visibility and reduced motion. Native IPC mocked.
    Actual surface/gradient renders each sampled value. This is NOT uninterrupted playback evidence.</p>
    <button id="accept">Run sampled assertions</button>
    <button id="capture32">Freeze rise at 32ms</button><button id="capture64">Freeze rise at 64ms</button><button id="captureEnd">Freeze rise endpoint</button>
    <button id="targetLow">Raw 0.05 → display 0.1</button><button id="targetHigh">Raw 1 → display 1</button><button id="targetInterrupt">Raw 0.2 → display 0.4</button>
    <button id="step16">Step 16ms</button><button id="step32">Step 32ms</button>
    <button id="reducedOn">Reduced on</button><button id="reducedOff">Reduced off</button>
    <button id="hidden">Hidden</button><button id="visible">Visible</button>
    <iframe id="sample" title="Actual sampled speech indicator" style="display:block;width:240px;height:160px;margin:16px 0" src="./index.html?surface=speech-indicator&sampleclock=1"></iframe>
    <pre id="sampleEvidence" style="white-space:pre-wrap">Waiting for actual surface…</pre>`;
  const frame = document.querySelector('#sample');
  const output = document.querySelector('#sampleEvidence');
  const trace = [];
  const api = () => frame.contentWindow.__fillSampling;
  const record = (action) => {
    const state = api().read(); trace.push({ action, ...state });
    window.__sampleTrace = trace;
    output.textContent = JSON.stringify({ emulation: 'manual time/RAF/visibility/preferences; actual DOM paint', latest: { action, ...state },
      samples: trace.map(({ action, now, level, queuedRAF }) => ({ action, now, level, queuedRAF })) }, null, 2);
    return state;
  };
  const target = async (value) => { frame.contentWindow.__speechThemeVoice(value); await api().flush(); return record(`raw target ${value}; NO frame`); };
  const step = async (ms) => { await api().step(ms); return record(`frame +${ms}ms`); };
  const reset = async () => {
    await api().reduce(false); await api().visibility(false);
    frame.contentWindow.__speechThemeStatus('idle'); await api().flush();
    frame.contentWindow.__speechThemeStatus('recording'); await api().flush();
  };
  const assert = (value, message) => { if (!value) throw new Error(message); };
  const close = (a, b) => Math.abs(a - b) < 1e-10;
  for (const [id, value] of [['targetLow', 0.05], ['targetHigh', 1], ['targetInterrupt', 0.2]]) document.querySelector(`#${id}`).onclick = () => target(value);
  for (const ms of [16, 32]) document.querySelector(`#step${ms}`).onclick = () => step(ms);
  for (const [id, value] of [['reducedOn', true], ['reducedOff', false]]) document.querySelector(`#${id}`).onclick = async () => { await api().reduce(value); record(id); };
  for (const [id, value] of [['hidden', true], ['visible', false]]) document.querySelector(`#${id}`).onclick = async () => { await api().visibility(value); record(id); };
  for (const [id, ms] of [['capture32', 32], ['capture64', 64], ['captureEnd', 112]]) document.querySelector(`#${id}`).onclick = async () => {
    trace.length = 0; await reset(); await target(0.05); await step(112); await target(1);
    for (let elapsed = 0; elapsed < ms; elapsed += 16) await step(16);
    record(`FROZEN rise ${ms}ms; actual gradient visible above`);
  };
  document.querySelector('#accept').onclick = async () => {
    try {
      trace.length = 0; await reset(); await target(0.05); await step(112);
      assert(api().read().level === 0.1, 'raw 0.05 display 0.1 endpoint not exact');
      assert((await target(1)).level === 0.1, 'rise jumped without frame');
      await step(16); const rise32 = (await step(16)).level;
      assert(rise32 > 0.1 && rise32 < 1, 'rise32 not intermediate');
      let prior = rise32;
      for (let i = 0; i < 5; i++) { const s = await step(16); assert(s.level >= prior && s.level <= 1, 'rise nonmonotonic/overshoot'); prior = s.level; }
      assert(prior === 1, 'rise endpoint not exact');
      assert((await target(0.05)).level === 1, 'fall jumped without frame');
      await step(16); const fall32 = (await step(16)).level;
      assert(fall32 > 0.1 && fall32 < 1, 'fall32 not intermediate');
      assert((1 - fall32) / 0.9 < (rise32 - 0.1) / 0.9, 'fall not gentler');
      prior = fall32;
      for (let i = 0; i < 10; i++) { const s = await step(16); assert(s.level <= prior && s.level >= 0.1, 'fall nonmonotonic/overshoot'); prior = s.level; }
      assert(prior === 0.1, 'raw 0.05 display 0.1 fall endpoint not exact');
      await target(1); const interrupted = (await step(32)).level;
      assert(close((await target(0.2)).level, interrupted), 'retarget jumped');
      const retarget32 = (await step(32)).level;
      assert(retarget32 > 0.4 && retarget32 < interrupted, 'interrupted fall not intermediate');
      await api().reduce(true); assert(api().read().level === 0.4, 'dynamic reduce did not snap'); record('reduce ON snaps raw 0.2 to display 0.4');
      assert((await target(1)).level === 1, 'reduced target did not snap');
      await api().reduce(false); await target(0.05); await step(32);
      await api().visibility(true); const hiddenLevel = api().read().level;
      assert(api().read().queuedRAF === 0, 'hidden RAF leaked'); record('hidden cancels ALL RAF');
      await target(1); await step(32);
      assert(api().read().queuedRAF === 0 && api().read().level === hiddenLevel, 'hidden work/paint continued');
      await api().visibility(false); record('visible resumes'); await step(32); await step(80);
      assert(api().read().level === 1, 'resume did not converge');
      assert(trace.every((s) => Number.isFinite(s.level) && s.level >= 0 && s.level <= 1 && s.shell.join() === '40,40' && s.glyph.join() === '24,24' && s.border === '0px' && s.shadow === 'none'), 'finite/geometry/ring contract failed');
      record('PASS: sampled rise/fall/retarget/reduce/hidden/resume assertions');
    } catch (error) { output.textContent += `\nFAIL: ${error.message}`; throw error; }
  };
} else {
  document.body.style.background = '#eef2f6';
  document.body.style.color = '#111827';
  localStorage.setItem('jasonshell.theme', 'dracula');
  document.body.innerHTML = `<h1>Real Settings → real indicator</h1>
    <p>Native IPC mocked; real theme channels. Indicator visibility explicitly emulated visible (panel reports hidden). Voice packets only through volume controls.</p>
    <button id="monokai">Choose Monokai in real Settings</button>
    <button id="andromeda">Choose Andromeda in real Settings</button>
    <button id="dracula">Choose Dracula in real Settings</button>
    <button id="base-light">Choose Base Light in real Settings</button>
    <button id="low">Low voice 0.05</button><button id="mid">Mid voice 0.5</button><button id="full">Full voice 1</button><button id="quiet">Quiet voice 0</button>
    <button id="pause">Pause capture</button><button id="record">Recording</button>
    <button id="reduce">Reload reduced-motion indicator</button>
    <button id="fallback">Reload GPU-unavailable indicator</button>
    <button id="normal">Reload normal indicator</button>
    <div style="display:flex;gap:20px;margin-top:16px"><iframe id="settings" title="Actual Settings" style="width:55%;height:650px"></iframe>
    <section><iframe id="indicator" title="Actual speech indicator" style="width:180px;height:120px"></iframe>
    <div id="contrast" style="display:flex;gap:10px"></div>
    <pre id="evidence" style="font-size:11px;max-height:490px;overflow:auto">Loading…</pre></section></div>`;
  const settings = document.querySelector('#settings');
  const indicator = document.querySelector('#indicator');
  settings.src = './index.html?surface=settings-panel';
  const loadIndicator = (mode = '') => { indicator.src = `./index.html?surface=speech-indicator&visible=1${mode}`; };
  loadIndicator();
  for (const [name, level] of [['low', 0.05], ['mid', 0.5], ['full', 1]]) {
    const cell = document.createElement('div');
    cell.innerHTML = `<strong>${name}: ${level}</strong><br><iframe title="Actual ${name} fill" style="width:100px;height:90px" src="./index.html?surface=speech-indicator&visible=1&volume=${level}"></iframe>`;
    document.querySelector('#contrast').append(cell);
  }
  const wait = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
  for (const [id, label] of [['monokai', 'Monokai'], ['andromeda', 'Andromeda'], ['dracula', 'Dracula'], ['base-light', 'Base Light']]) {
    document.querySelector(`#${id}`).onclick = async () => {
      const doc = settings.contentDocument;
      const trigger = doc.querySelector('button[aria-label="Theme"]');
      if (!trigger) { document.querySelector('#evidence').textContent = 'ERROR: real Settings theme control missing'; return; }
      trigger.click();
      await wait(100);
      const option = [...doc.querySelectorAll('.melt-select-option')].find((node) => node.querySelector('span')?.textContent === label);
      if (!option) throw new Error(`Real Settings option missing: ${label}`);
      option.click();
    };
  }
  document.querySelector('#pause').onclick = () => indicator.contentWindow.__speechThemeStatus('idle');
  document.querySelector('#record').onclick = () => indicator.contentWindow.__speechThemeStatus('recording');
  document.querySelector('#reduce').onclick = () => loadIndicator('&reduce=1');
  document.querySelector('#fallback').onclick = () => loadIndicator('&fallback=1');
  document.querySelector('#normal').onclick = () => loadIndicator();
  for (const [id, level] of [['low', 0.05], ['mid', 0.5], ['full', 1], ['quiet', 0]]) {
    document.querySelector(`#${id}`).onclick = () => indicator.contentWindow.__speechThemeVoice(level);
  }
  const fallbackBackgrounds = new Map();
  setInterval(() => {
    const doc = indicator.contentDocument;
    const win = indicator.contentWindow;
    const shell = doc?.querySelector('.mic-shell');
    if (!shell) return;
    const token = win.getComputedStyle(doc.documentElement).getPropertyValue('--js-color-accent').trim();
    const probe = doc.createElement('span'); probe.style.color = token; doc.body.append(probe);
    const rgb = win.getComputedStyle(probe).color.match(/[\d.]+/g)?.slice(0, 3).map(Number); probe.remove();
    const actual = win.__speechThemeGPU;
    const match = actual && rgb && actual.every((value, index) => Math.abs(value - rgb[index] / 255) < 0.00001);
    const background = win.getComputedStyle(shell).backgroundColor;
    let result = actual ? (match ? 'PASS: theme accent reaches GPU' : 'RED: theme propagated but GPU remains wrong') : 'FALLBACK: awaiting two themes';
    if (indicator.src.includes('fallback=1')) {
      fallbackBackgrounds.set(doc.documentElement.dataset.theme, background);
      const rgbAlpha = background.startsWith('rgba(') ? Number(background.match(/[\d.]+/g)?.[3]) : 1;
      const slashAlpha = background.match(/\/\s*([\d.]+)(%)?/);
      const opaque = background !== 'transparent' && rgbAlpha === 1 && (!slashAlpha || Number(slashAlpha[1]) === (slashAlpha[2] ? 100 : 1));
      if (!opaque) result = 'RED: fallback is not fully opaque';
      else if (fallbackBackgrounds.size > 1) result = new Set(fallbackBackgrounds.values()).size > 1
        ? 'PASS: opaque fallback changes with theme' : 'RED: opaque fallback ignores theme';
    }
    const state = {
      settingsTheme: settings.contentDocument?.documentElement.dataset.theme,
      indicatorTheme: doc.documentElement.dataset.theme,
      resolvedAccent: token, expectedRGB: rgb, actualGPU: actual,
      result, background,
      documentHidden: doc.hidden,
      nativePanelHidden: win.__speechThemeNativeHidden,
      visibilityEmulated: indicator.src.includes('visible=1'),
      canvasVisibility: doc.querySelector('canvas')?.style.visibility,
      reducedMotion: win.matchMedia('(prefers-reduced-motion: reduce)').matches,
      forcedColors: win.matchMedia('(forced-colors: active)').matches,
      micLevel: doc.querySelector('.mic-glyph')?.style.getPropertyValue('--mic-level'),
      size: [shell.getBoundingClientRect().width, shell.getBoundingClientRect().height],
      glyphSize: (() => { const rect = doc.querySelector('.mic-glyph').getBoundingClientRect(); return [rect.width, rect.height]; })(),
      shellRing: (() => { const style = win.getComputedStyle(shell); return { border: style.borderWidth, outline: style.outlineStyle, shadow: style.boxShadow }; })(),
      contrastLevels: [...document.querySelectorAll('#contrast iframe')].map((frame) => ({
        name: frame.title,
        theme: frame.contentDocument?.documentElement.dataset.theme,
        level: frame.contentDocument?.querySelector('.mic-glyph')?.style.getPropertyValue('--mic-level')
      }))
    };
    document.querySelector('#evidence').textContent = JSON.stringify(state, null, 2);
  }, 200);
}
