import { invoke } from '@tauri-apps/api/core';
const status = document.querySelector('#status');
const events = [];
const mark = (kind, metadata = {}) => events.push({ kind, timeMs: performance.now(), metadata });
const paintCallback = () => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve)));
async function run() {
  mark('intent');
  const mode = await invoke('probe_mode');
  if (mode === 'control') {
    const start = performance.now();
    while (performance.now() - start < 80) { /* Deliberate no-editor stall calibration. */ }
    mark('controlStall'); status.textContent = 'Control complete; deliberate 80ms stall.';
  } else {
    const worker = new Worker(new URL('./worker.js', import.meta.url), { type: 'module' });
    try {
      await new Promise((resolve, reject) => {
        const timer = setTimeout(() => reject(Error('WorkerTimeout')), 3000);
        worker.onmessage = ({ data }) => { clearTimeout(timer); data === 'ready' ? resolve() : reject(Error('WorkerProtocol')); };
        worker.onerror = () => { clearTimeout(timer); reject(Error('WorkerUnavailable')); };
        worker.postMessage('ready');
      });
      mark('workerReady');
    } finally { worker.terminate(); }
    const [{ EditorState }, { EditorView }, { defaultKeymap }, { keymap }] = await Promise.all([
      import('@codemirror/state'), import('@codemirror/view'), import('@codemirror/commands'), import('@codemirror/view')
    ]);
    const prefix = await invoke('probe_prefix');
    const view = new EditorView({ state: EditorState.create({ doc: prefix, extensions: [keymap.of(defaultKeymap), EditorView.contentAttributes.of({ 'aria-label': 'Seeded test document' })] }), parent: document.querySelector('#editor') });
    await paintCallback(); mark('firstPaintCallback', { bytesRead: 128 });
    view.dispatch({ changes: { from: 0, insert: 'x' } }); mark('localMutation');
    await paintCallback(); mark('inputPaintCallback');
    status.textContent = 'Local programmatic edit visible; ACK pending; EOF withheld.';
    await invoke('probe_ack'); mark('ack', { revision: '1' });
    status.textContent = 'ACK received; durability and EOF deliberately unobserved.';
  }
  await invoke('probe_report', { events });
}
run().catch(() => { status.textContent = 'Probe failed. Inspect native stderr.'; });
