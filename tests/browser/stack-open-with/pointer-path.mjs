// Fast trusted CDP path inside the browser owned by agent-browser. Separate CLI
// invocations can consume the entire 220ms grace between neighboring points.
const [endpoint, serialized] = process.argv.slice(2);
const socket = new WebSocket(endpoint);
await new Promise((resolve, reject) => { socket.onopen = resolve; socket.onerror = reject; });
let next = 0;
const pending = new Map();
socket.onmessage = ({ data }) => {
  const message = JSON.parse(String(data));
  const callback = pending.get(message.id);
  if (callback) { pending.delete(message.id); message.error ? callback.reject(new Error(JSON.stringify(message.error))) : callback.resolve(message.result); }
};
function send(method, params = {}, sessionId) {
  return new Promise((resolve, reject) => {
    const id = ++next; pending.set(id, { resolve, reject }); socket.send(JSON.stringify({ id, method, params, sessionId }));
  });
}
try {
  const { targetInfos } = await send('Target.getTargets');
  const target = targetInfos.find(t => t.type === 'page' && t.url.startsWith('http://127.0.0.1:4186'));
  if (!target) throw new Error('Fixture target missing');
  const { sessionId } = await send('Target.attachToTarget', { targetId: target.targetId, flatten: true });
  const trace = [];
  for (const [x,y] of JSON.parse(serialized)) {
    await send('Input.dispatchMouseEvent', { type: 'mouseMoved', x, y, pointerType: 'mouse' }, sessionId);
    await new Promise(resolve => setTimeout(resolve, 16));
    const snapshot = await send('Runtime.evaluate', { expression: `JSON.stringify({x:${x},y:${y},expanded:document.querySelector('.submenu-trigger')?.getAttribute('aria-expanded'),hit:document.elementFromPoint(${x},${y})?.textContent?.trim()})`, returnByValue: true }, sessionId);
    trace.push(JSON.parse(snapshot.result.value));
  }
  console.log(JSON.stringify(trace));
} finally { socket.close(); }
