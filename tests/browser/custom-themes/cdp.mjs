// No added dependency: Node 24's WebSocket drives the already-installed Chrome
// when agent-browser's daemon transport is unavailable. Not a native-shell test.
export async function connectPage(endpoint, url) {
  const target = await (await fetch(`${endpoint}/json/new?${encodeURIComponent(url)}`, { method: 'PUT' })).json();
  const socket = new WebSocket(target.webSocketDebuggerUrl);
  await new Promise((resolve, reject) => { socket.onopen = resolve; socket.onerror = reject; });
  let serial = 0;
  const pending = new Map();
  const errors = [];
  socket.onmessage = event => {
    const message = JSON.parse(event.data);
    if (message.method === 'Runtime.exceptionThrown') errors.push(message.params.exceptionDetails);
    const request = pending.get(message.id);
    if (!request) return;
    pending.delete(message.id); clearTimeout(request.timer);
    if (message.error) request.reject(new Error(JSON.stringify(message.error))); else request.resolve(message.result);
  };
  function send(method, params = {}) {
    const id = ++serial;
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => { pending.delete(id); reject(new Error(`CDP timeout: ${method}`)); }, 15_000);
      pending.set(id, { resolve, reject, timer });
      socket.send(JSON.stringify({ id, method, params }));
    });
  }
  async function evaluate(expression) {
    const response = await send('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true });
    if (response.exceptionDetails) throw new Error(response.exceptionDetails.exception?.description ?? response.exceptionDetails.text);
    return response.result.value;
  }
  async function wait(expression, timeout = 12_000) {
    const deadline = Date.now() + timeout;
    while (Date.now() < deadline) {
      if (await evaluate(expression)) return;
      await new Promise(resolve => setTimeout(resolve, 50));
    }
    throw new Error(`Browser condition timed out: ${expression}\n${JSON.stringify(errors)}`);
  }
  await send('Runtime.enable');
  return { id: target.id, errors, send, evaluate, wait,
    close: async () => { socket.close(); await fetch(`${endpoint}/json/close/${target.id}`); },
    disconnect: () => socket.close() };
}

export async function click(page, name, scope = 'document') {
  await page.evaluate(`(() => { const node = [...${scope}.querySelectorAll('button')].find(node => (node.getAttribute('aria-label') || node.textContent).trim() === ${JSON.stringify(name)}); if (!node) throw new Error('Missing button: ' + ${JSON.stringify(name)}); node.focus(); node.click(); })()`);
}
export async function input(page, label, value) {
  await page.evaluate(`(() => { const node = document.querySelector('input[aria-label=' + JSON.stringify(${JSON.stringify(label)}) + ']') || [...document.querySelectorAll('label')].find(node => node.textContent.trim() === ${JSON.stringify(label)})?.querySelector('input'); if (!node) throw new Error('Missing input: ' + ${JSON.stringify(label)}); node.focus(); node.value = ${JSON.stringify(value)}; node.dispatchEvent(new Event('input', { bubbles:true })); })()`);
}

export async function press(page, key) {
  const codes = { ' ': ['Space', 32], Home: ['Home', 36], ArrowDown: ['ArrowDown', 40], Enter: ['Enter', 13], Tab: ['Tab', 9], Escape: ['Escape', 27] };
  const [code, windowsVirtualKeyCode] = codes[key];
  await page.send('Input.dispatchKeyEvent', { type: 'keyDown', key, code, windowsVirtualKeyCode, ...(key === ' ' ? { text: ' ' } : {}) });
  await page.send('Input.dispatchKeyEvent', { type: 'keyUp', key, code, windowsVirtualKeyCode });
}

export async function selectTheme(page, label) {
  await page.evaluate(`document.querySelector('[role="combobox"][aria-label="Theme"]').click()`);
  await page.evaluate(`(() => { const option = [...document.querySelectorAll('[role="option"]')].find(node => node.getAttribute('data-label') === ${JSON.stringify(label)}); if (!option) throw new Error('Missing theme option'); option.click(); })()`);
}
