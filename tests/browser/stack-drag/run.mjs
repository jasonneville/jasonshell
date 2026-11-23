import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { existsSync, mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { setTimeout as wait } from 'node:timers/promises';
import { createServer } from 'vite';

const chrome = [process.env.CHROME_PATH, 'C:/Program Files/Google/Chrome/Application/chrome.exe', '/usr/bin/chromium'].find(path => path && existsSync(path));
assert(chrome, 'Set CHROME_PATH to a Chromium executable');
const server = await createServer({ configFile: fileURLToPath(new URL('./vite.config.ts', import.meta.url)), server: { host: '127.0.0.1', port: 0 } });
await server.listen();
const origin = `http://127.0.0.1:${server.httpServer.address().port}`;
const profile = mkdtempSync(join(tmpdir(), 'stack-drag-'));
const browser = spawn(chrome, ['--headless', '--remote-debugging-port=0', '--force-device-scale-factor=1.5', '--window-size=900,650', '--no-first-run', `--user-data-dir=${profile}`, 'about:blank']);
let socket;
try {
  const endpoint = await new Promise((resolve, reject) => {
    const timer = setTimeout(() => reject(new Error('Chromium debug endpoint unavailable')), 10000);
    let output = '';
    browser.stderr.on('data', chunk => {
      output += chunk;
      const match = output.match(/DevTools listening on ws:\/\/([^/\s]+)/);
      if (match) { clearTimeout(timer); resolve(`http://${match[1]}`); }
    });
    browser.once('error', error => { clearTimeout(timer); reject(error); });
  });
  const tabs = await (await fetch(`${endpoint}/json/list`)).json();
  socket = new WebSocket(tabs.find(tab => tab.type === 'page').webSocketDebuggerUrl);
  await once(socket, 'open');
  let id = 0;
  function send(method, params = {}) {
    const current = ++id;
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => { socket.removeEventListener('message', listener); reject(new Error(`CDP timeout: ${method}`)); }, 5000);
      function listener(event) {
        const response = JSON.parse(event.data);
        if (response.id !== current) return;
        clearTimeout(timer);
        socket.removeEventListener('message', listener);
        if (response.error) reject(new Error(JSON.stringify(response.error))); else resolve(response.result);
      }
      socket.addEventListener('message', listener);
      socket.send(JSON.stringify({ id: current, method, params }));
    });
  }
  async function evaluate(expression) {
    const result = await send('Runtime.evaluate', { expression, returnByValue: true, awaitPromise: true });
    if (result.exceptionDetails) throw new Error(result.exceptionDetails.exception?.description ?? result.exceptionDetails.text);
    return result.result.value;
  }
  async function until(expression) {
    for (let i = 0; i < 200; i++) { const value = await evaluate(expression); if (value) return value; await wait(25); }
    throw new Error(`Timed out: ${expression}`);
  }
  // CDP needs the pressed button on held moves to keep pointer capture active.
  const mouse = (type, { x, y }, buttons) => send('Input.dispatchMouseEvent', { type, x, y, button: type === 'mouseMoved' && buttons === 0 ? 'none' : 'left', buttons, clickCount: 1 });
  const point = (selector, y, x = 'r.left + 40') => evaluate(`(() => { const r = document.querySelector(${JSON.stringify(selector)}).getBoundingClientRect(); return { x: ${x}, y: ${y} }; })()`);
  const row = name => `[data-stack-entry-path$="${name}"]`;
  const midpoint = selector => point(selector, 'r.top + r.height / 2');
  const center = selector => point(selector, 'r.top + r.height / 2', 'r.left + r.width / 2');
  const mutationCalls = () => evaluate(`fixture.ledger.filter(e => /^(cut|copy|paste)_stack_items$|^commit_stack_local_drop$/.test(e.command))`);
  const drop = (paths, at) => evaluate(`fixture.drop(${JSON.stringify(paths)}, ${JSON.stringify(at)})`);
  async function open() {
    await send('Page.navigate', { url: origin });
    await until(`window.fixture && document.querySelector('${row('beta.txt')}')`);
    assert.equal(await evaluate('devicePixelRatio'), 1.5);
    return evaluate('({paths: fixture.paths, root: fixture.root, dest: fixture.dest, external: fixture.external})');
  }
  async function drag(start, to) {
    await mouse('mouseMoved', start, 0);
    await mouse('mousePressed', start, 1);
    await mouse('mouseMoved', to, 1);
  }
  async function marquee(geometry) {
    const start = await point('.details-body', 'r.bottom - 40');
    const end = await midpoint(row('alpha.txt'));
    await drag(start, { x: end.x + 35, y: end.y });
    await mouse('mouseReleased', { x: end.x + 35, y: end.y }, 0);
    assert.deepEqual(await evaluate(`Array.from(document.querySelectorAll('[data-stack-entry-path][aria-selected=true]'), row => row.dataset.stackEntryPath)`), geometry.paths);
  }
  const callsFor = command => evaluate(`fixture.ledger.filter(e => e.command === ${JSON.stringify(command)})`);
  const drain = () => evaluate('new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve)))');
  const undoButton = 'button[aria-label="Undo move"]';
  async function ctrl(key, repeat = false) {
    for (const type of ['rawKeyDown', 'keyUp']) await send('Input.dispatchKeyEvent', {
      type, key, code: `Key${key.toUpperCase()}`, modifiers: 2, windowsVirtualKeyCode: key.toUpperCase().charCodeAt(0), autoRepeat: repeat && type === 'rawKeyDown',
    });
    await drain();
  }
  const ctrlZ = repeat => ctrl('z', repeat);
  async function click(at, clickCount = 1) {
    for (const [type, buttons] of [['mousePressed', 1], ['mouseReleased', 0]]) {
      await send('Input.dispatchMouseEvent', { type, ...at, button: 'left', buttons, clickCount });
    }
  }
  async function assertUndoReceipt(sequence = 1) {
    assert.deepEqual(await callsFor('undo_stack_move'), [{ command: 'undo_stack_move', args: { undoMoveId: `fixture-opaque-undo-${sequence}` } }]);
  }
  const deleteButton = 'button[aria-label="Undo delete"]';
  const confirmation = '[role="alertdialog"]';
  async function openDeleteDialog() {
    for (const type of ['rawKeyDown', 'keyUp']) await send('Input.dispatchKeyEvent', { type, key: 'Delete', code: 'Delete', windowsVirtualKeyCode: 46 });
    await until(`document.querySelector('${confirmation}')`);
  }
  async function deleteDialog(action) {
    await openDeleteDialog();
    const at = await evaluate(`(() => {
      const dialog = document.querySelector('[role="alertdialog"], [role="dialog"]');
      const button = Array.from(dialog.querySelectorAll('button')).find(button => button.textContent.trim() === ${JSON.stringify(action)});
      if (!button || button.disabled) throw new Error('Confirmation action unavailable');
      const r = button.getBoundingClientRect();
      return { x: r.left + r.width / 2, y: r.top + r.height / 2 };
    })()`);
    await click(at);
    await drain();
  }
  async function deletedBatch(paths) {
    const calls = await callsFor('delete_stack_items');
    assert.equal(calls.length, 1, `Expected one batch delete; legacy calls: ${JSON.stringify(await callsFor('delete_stack_item'))}`);
    assert.deepEqual([...calls[0].args.paths].sort(), [...paths].sort());
    assert.equal(typeof calls[0].args.operationId, 'string');
    assert.deepEqual(await callsFor('delete_stack_item'), []);
  }
  async function restored(g) {
    await until(`fixture.ledger.some(e => e.command === 'undo_stack_move') && document.querySelector('${row('alpha.txt')}') && document.querySelector('${row('beta.txt')}')`);
    await assertUndoReceipt();
    assert.deepEqual(await evaluate('fixture.entries.map(e => e.path).sort()'), [g.dest, ...g.paths].sort());
    assert.ok((await callsFor('read_stack_folder')).every(e => e.args.path === g.root), 'Undo must refresh without folder navigation');
  }
  async function release(at) {
    await mouse('mouseMoved', at, 1);
    await mouse('mouseReleased', at, 0);
    await drain();
  }
  async function inside(source, at) {
    await drag(source, { ...source, x: source.x + 12 });
    await mouse('mouseMoved', at, 1);
  }
  async function move(g, paths, delayed = false) {
    const at = await midpoint(row('Dest'));
    if (delayed) await evaluate('fixture.holdPrepare()');
    await inside(await midpoint(row('alpha.txt')), at);
    assert.deepEqual(await mutationCalls(), []);
    await release(at);
    const ole = await callsFor('start_stack_file_drag');
    assert.deepEqual(ole, [], `In-popup release must not invoke OLE; mutations=${JSON.stringify(await mutationCalls())}`);
    assert.deepEqual((await callsFor('prepare_stack_local_drop')).map(e => e.args.paths), [paths]);
    if (delayed) {
      assert.deepEqual(await mutationCalls(), []);
      await evaluate('fixture.releasePrepare()');
    }
    await until(`fixture.ledger.some(e => e.command === 'commit_stack_local_drop')`);
    for (const path of paths) await until(`!document.querySelector('${row(path.split('\\').pop())}')`);
    assert.deepEqual(await mutationCalls(), [{ command: 'commit_stack_local_drop', args: { requestId: 'fixture-native-request', destination: g.dest } }]);
    assert.deepEqual(await evaluate('fixture.entries.map(e => e.path).sort()'), [g.dest, ...g.paths.map(path => paths.includes(path) ? `${g.dest}\\${path.split('\\').pop()}` : path)].sort());
  }
  async function noLocalMutation() {
    assert.deepEqual(await mutationCalls(), []);
    assert.deepEqual(await callsFor('start_stack_file_drag'), []);
  }
  const cases = {
    'Hide and reopen the same folder discards an unconfirmed delete': async g => {
      await click(await midpoint(row('alpha.txt')));
      await openDeleteDialog();
      const priorReads = (await callsFor('read_stack_folder')).length;
      await evaluate('fixture.hide()');
      await drain();
      await evaluate('fixture.reopen()');
      await until(`fixture.ledger.filter(e => e.command === 'read_stack_folder').length > ${priorReads} && document.querySelector('${row('alpha.txt')}')`);
      assert.equal(await evaluate(`!!document.querySelector('${confirmation}')`), false);
      assert.deepEqual(await callsFor('delete_stack_items'), []);
      assert.deepEqual(await callsFor('delete_stack_item'), []);
      assert.deepEqual(await evaluate('fixture.entries.map(e => e.path).sort()'), [g.dest, ...g.paths].sort());
    },
    'Cancel delete keeps an earlier move receipt available': async g => {
      await move(g, [g.paths[0]]);
      await click(await midpoint(row('beta.txt')));
      await deleteDialog('Cancel');
      assert.deepEqual(await callsFor('delete_stack_items'), []);
      await ctrlZ();
      await restored(g);
    },
    'Cancel keeps selected files; confirmed batch deletes once and toolbar restores both': async g => {
      await marquee(g);
      await deleteDialog('Cancel');
      assert.deepEqual(await callsFor('delete_stack_items'), []);
      assert.deepEqual(await callsFor('delete_stack_item'), []);
      assert.deepEqual(await evaluate('fixture.entries.map(e => e.path).sort()'), [g.dest, ...g.paths].sort());
      await deleteDialog('Delete');
      await deletedBatch(g.paths);
      await until(`!document.querySelector('${row('alpha.txt')}') && !document.querySelector('${row('beta.txt')}')`);
      await until(`document.querySelector(${JSON.stringify(`${deleteButton}:enabled`)})`);
      await click(await center(deleteButton));
      await restored(g);
      await ctrlZ();
      await assertUndoReceipt();
    },
    'Confirmed single delete replaces a previous move receipt for Ctrl+Z': async g => {
      await move(g, [g.paths[0]]);
      await click(await midpoint(row('beta.txt')));
      await deleteDialog('Delete');
      await deletedBatch([g.paths[1]]);
      await until(`!document.querySelector('${row('beta.txt')}')`);
      await ctrlZ();
      await until(`document.querySelector('${row('beta.txt')}')`);
      await ctrlZ();
      await assertUndoReceipt(2);
      assert.deepEqual(await evaluate('fixture.entries.map(e => e.path).sort()'), [g.dest, `${g.dest}\\alpha.txt`, g.paths[1]].sort());
    },
    'An untrackable confirmed delete retires rather than revives the previous move': async g => {
      await move(g, [g.paths[0]]);
      await evaluate('fixture.untrackableDelete()');
      await click(await midpoint(row('beta.txt')));
      await deleteDialog('Delete');
      await deletedBatch([g.paths[1]]);
      await until(`!document.querySelector('${row('beta.txt')}')`);
      await ctrlZ();
      assert.deepEqual(await callsFor('undo_stack_move'), []);
      assert.deepEqual(await evaluate('fixture.entries.map(e => e.path).sort()'), [g.dest, `${g.dest}\\alpha.txt`].sort());
    },
    'Shift-drop stages one cut while a second cut is ignored': async g => {
      await click(await midpoint(row('beta.txt')));
      await evaluate(`fixture.holdCut(); fixture.shiftDrop(fixture.paths[0], document.querySelector('${row('Dest')}'))`);
      await until(`fixture.ledger.some(e => e.command === 'cut_stack_items')`);
      await ctrl('x');
      await evaluate('fixture.releaseCut()');
      await until(`!document.querySelector('${row('alpha.txt')}')`);
      assert.deepEqual((await callsFor('cut_stack_items')).map(e => e.args.paths), [[g.paths[0]]]);
      assert.deepEqual((await callsFor('paste_stack_items')).map(e => e.args.destination), [g.dest]);
      assert.deepEqual(await evaluate('fixture.entries.map(e => e.path).sort()'), [g.dest, g.paths[1], `${g.dest}\\alpha.txt`].sort());
    },
    'A watcher error does not disable Undo move in a visible folder': async g => {
      await move(g, [g.paths[0]]);
      await until(`fixture.ledger.some(e => e.command === 'start_stack_folder_watch')`);
      await evaluate('fixture.failWatch()');
      await until(`document.body.textContent.includes('Fixture watcher error')`);
      await until(`document.querySelector(${JSON.stringify(`${undoButton}:enabled`)})`);
      await click(await center(undoButton));
      await restored(g);
    },
    'Copy without an undo receipt leaves the latest move available': async g => {
      await move(g, [g.paths[0]]);
      await drop([g.external], await midpoint(row('Dest')));
      await until(`document.querySelector('${row('incoming.txt')}')`);
      await ctrlZ();
      await until(`fixture.ledger.some(e => e.command === 'undo_stack_move') && document.querySelector('${row('alpha.txt')}')`);
      await assertUndoReceipt();
      assert.deepEqual(await evaluate('fixture.entries.map(e => e.path).sort()'), [g.dest, ...g.paths, `${g.root}\\incoming.txt`].sort());
    },
    'Ctrl+Z restores the moved batch exactly once and ignores repeat': async g => {
      await marquee(g);
      await move(g, g.paths);
      await ctrlZ(true);
      assert.deepEqual(await callsFor('undo_stack_move'), []);
      await ctrlZ();
      await restored(g);
      await ctrlZ();
      assert.equal((await callsFor('undo_stack_move')).length, 1);
    },
    'Undo move toolbar button restores a moved file': async g => {
      await move(g, [g.paths[0]]);
      await until(`document.querySelector(${JSON.stringify(`${undoButton}:enabled`)})`);
      await click(await center(undoButton));
      await restored(g);
    },
    'Search input owns Ctrl+Z while focused without consuming move undo': async g => {
      await move(g, [g.paths[0]]);
      const search = 'input[aria-label="Search current folder"]';
      await click(await midpoint(search));
      assert.equal(await evaluate(`document.activeElement.matches(${JSON.stringify(search)})`), true);
      await send('Input.insertText', { text: 'beta' });
      await until(`!document.querySelector('${row('Dest')}')`);
      await ctrlZ();
      await until(`document.querySelector('${row('Dest')}')`);
      assert.deepEqual(await callsFor('undo_stack_move'), []);
      await click(await midpoint(row('beta.txt')));
      assert.equal(await evaluate(`document.activeElement.matches(${JSON.stringify(search)})`), false);
      await ctrlZ();
      await restored(g);
    },
    'Cut and paste into a folder can be undone without leaving that folder': async g => {
      await click(await midpoint(row('alpha.txt')));
      await ctrl('x');
      const at = await midpoint(row('Dest'));
      await click(at);
      await click(at, 2);
      await until(`fixture.ledger.some(e => e.command === 'read_stack_folder' && e.args.path === ${JSON.stringify(g.dest)}) && !document.querySelector('${row('beta.txt')}')`);
      await ctrl('v');
      await until(`document.querySelector('${row('alpha.txt')}')`);
      const mutations = await mutationCalls();
      assert.deepEqual(mutations.map(entry => entry.command), ['cut_stack_items', 'paste_stack_items']);
      assert.deepEqual(mutations[0].args.paths, [g.paths[0]]);
      assert.equal(mutations[1].args.destination, g.dest);
      assert.deepEqual(await evaluate('fixture.entries.map(e => e.path).sort()'), [g.dest, g.paths[1], `${g.dest}\\alpha.txt`].sort());
      const priorReads = (await callsFor('read_stack_folder')).length;
      await ctrlZ();
      await until(`fixture.ledger.some(e => e.command === 'undo_stack_move') && !document.querySelector('${row('alpha.txt')}')`);
      await assertUndoReceipt();
      assert.deepEqual(await evaluate('fixture.entries.map(e => e.path).sort()'), [g.dest, ...g.paths].sort());
      const reads = (await callsFor('read_stack_folder')).slice(priorReads);
      assert.ok(reads.length > 0 && reads.every(e => e.args.path === g.dest));
    },
    'pointer release moves marquee selection once without OLE or clipboard': async g => {
      await marquee(g);
      await move(g, g.paths);
    },
    'pointer release moves one file': async g => move(g, [g.paths[0]]),
    'quick release waits for local prepare': async g => move(g, [g.paths[0]], true),
    'Escape cancels pointer move': async () => {
      const at = await midpoint(row('Dest'));
      await inside(await midpoint(row('alpha.txt')), at);
      for (const type of ['keyDown', 'keyUp']) await send('Input.dispatchKeyEvent', { type, key: 'Escape', code: 'Escape', windowsVirtualKeyCode: 27 });
      await release(at);
      await noLocalMutation();
    },
    'lost pointer capture cancels move': async () => {
      const at = await midpoint(row('Dest'));
      await evaluate(`window.addEventListener('pointerdown', event => { window.testPointer = event.pointerId; }, {once:true,capture:true})`);
      await inside(await midpoint(row('alpha.txt')), at);
      assert.ok(await evaluate(`(() => { const e = Array.from(document.querySelectorAll('*')).find(e => e.hasPointerCapture(window.testPointer)); if (!e) return false; e.releasePointerCapture(window.testPointer); return true; })()`));
      await release(at);
      await noLocalMutation();
    },
    'external incoming copy': async g => {
      await drop([g.external], await midpoint(row('Dest')));
      await until(`document.querySelector('${row('incoming.txt')}')`);
      const calls = await mutationCalls();
      assert.deepEqual(calls.map(c => c.command), ['copy_stack_items', 'paste_stack_items']);
      assert.deepEqual(calls[0].args.paths, [g.external]);
      assert.equal(calls[1].args.destination, g.root);
      await ctrlZ();
      assert.deepEqual(await callsFor('undo_stack_move'), []);
      assert.ok(await evaluate(`!!document.querySelector('${row('incoming.txt')}')`));
    },
    'outbound only': async g => {
      await marquee(g);
      const source = await midpoint(row('alpha.txt'));
      const outside = { x: -5, y: source.y };
      await drag(source, { ...source, x: source.x + 12 });
      assert.deepEqual(await callsFor('start_stack_file_drag'), []);
      await mouse('mouseMoved', outside, 1);
      await release(await midpoint(row('Dest')));
      assert.deepEqual((await callsFor('start_stack_file_drag')).map(e => e.args.paths), [g.paths]);
      assert.deepEqual(await mutationCalls(), []);
    },
    'same-path untagged incoming drop stays copy-only': async g => {
      await marquee(g);
      const at = await midpoint(row('Dest'));
      await drop(g.paths, at);
      await drain();
      const calls = await mutationCalls();
      assert.deepEqual(calls.map(c => c.command), ['copy_stack_items', 'paste_stack_items']);
      assert.deepEqual(calls[0].args.paths, g.paths);
      assert.equal(calls[1].args.destination, g.root);
      assert.deepEqual(await evaluate('fixture.entries.map(entry => entry.path).sort()'), [g.dest, ...g.paths, ...g.paths.map(path => `${path} (copy)`)].sort());
    },
    'own background': async g => {
      await marquee(g);
      const at = await point('.details-body', 'r.bottom - 40');
      await inside(await midpoint(row('alpha.txt')), at);
      await release(at);
      await noLocalMutation();
    },
    'own file row': async g => {
      const at = await midpoint(row('beta.txt'));
      await inside(await midpoint(row('alpha.txt')), at);
      await release(at);
      await noLocalMutation();
    },
    'self folder row': async g => {
      const at = await midpoint(row('Dest'));
      await inside(at, at);
      await release(at);
      await noLocalMutation();
    },
  };
  for (const [name, run] of Object.entries(cases)) {
    try { await run(await open()); console.log(`PASS ${name}`); }
    catch (error) { process.exitCode = 1; console.error(`FAIL ${name}:`, error); }
  }
} finally {
  socket?.close();
  if (browser.exitCode === null && browser.pid) { const exited = once(browser, 'exit'); browser.kill(); await exited; }
  await server.close();
  try { rmSync(profile, { recursive: true, force: true, maxRetries: 5, retryDelay: 100 }); }
  catch (error) { console.warn(`Could not remove test Chromium profile ${profile}: ${error.message}`); }
}
