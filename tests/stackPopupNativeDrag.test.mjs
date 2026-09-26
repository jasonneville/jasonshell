import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const surface = readFileSync(new URL('../src/components/StackPopupSurface.svelte', import.meta.url), 'utf8');
const api = readFileSync(new URL('../src/lib/stackPopup.ts', import.meta.url), 'utf8');
const commands = readFileSync(new URL('../src/ipc/commands.ts', import.meta.url), 'utf8');
const handler = readFileSync(new URL('../src-tauri/src/stack_popup.rs', import.meta.url), 'utf8');
const native = readFileSync(new URL('../src-tauri/src/stack_popup/native_drag.rs', import.meta.url), 'utf8');
const models = readFileSync(new URL('../src-tauri/src/stack_popup/models.rs', import.meta.url), 'utf8');
const popup = readFileSync(new URL('../src-tauri/src/stack_popup/popup_window.rs', import.meta.url), 'utf8');

test('outbound rows do not give HTML DataTransfer ownership of native file drag', () => {
  const row = surface.slice(surface.indexOf('{#each virtualEntries.rows'), surface.indexOf('{/each}', surface.indexOf('{#each virtualEntries.rows')));
  assert.ok(row.length, 'virtualized row markup must be found');
  const rowButton = row.slice(row.indexOf('<button'), row.indexOf('on:drop=', row.indexOf('<button')));
  assert.doesNotMatch(rowButton, /\bdraggable=|on:dragstart=/);
  assert.doesNotMatch(surface, /function handleRowDragStart\(/);
});

test('native drag start remains behind held-primary pointer threshold (pure state contract)', async () => {
  // State transitions only: live OLE ownership and OS button state need native/manual evidence.
  const { beginStackRowDrag, moveStackRowDrag, releaseStackRowDrag } = await import('../src/features/stack-browser/nativeDragIntent.ts');
  const down = beginStackRowDrag({ pointerId: 4, button: 0, buttons: 1, x: 10, y: 10 });
  assert.equal(moveStackRowDrag(down, { pointerId: 4, buttons: 1, x: 12, y: 12 }).startNativeDrag, false);
  assert.equal(moveStackRowDrag(down, { pointerId: 4, buttons: 1, x: 22, y: 10 }).startNativeDrag, true);
  assert.equal(moveStackRowDrag(down, { pointerId: 4, buttons: 0, x: 22, y: 10 }).startNativeDrag, false,
    'a pointer move queued after button release must never initiate OLE');
  assert.equal(moveStackRowDrag(releaseStackRowDrag(down, 4), { pointerId: 4, buttons: 1, x: 22, y: 10 }).startNativeDrag, false);
  assert.equal(moveStackRowDrag(down, { pointerId: 5, buttons: 1, x: 22, y: 10 }).startNativeDrag, false);
  const secondary = beginStackRowDrag({ pointerId: 4, button: 2, buttons: 2, x: 10, y: 10 });
  assert.equal(moveStackRowDrag(secondary, { pointerId: 4, buttons: 2, x: 22, y: 10 }).startNativeDrag, false);
});

test('typed outbound IPC outcome carries bounded diagnostics, not source paths', () => {
  assert.match(commands, /startStackFileDrag:\s*'start_stack_file_drag'/);
  assert.match(api, /invoke<StackNativeDragOutcome>\(IPC_COMMANDS\.startStackFileDrag, \{ paths \}\)/);
  const rustOutcome = models.slice(models.indexOf('pub struct StackNativeDragOutcome'), models.indexOf('\n}', models.indexOf('pub struct StackNativeDragOutcome')));
  assert.match(rustOutcome, /request_id: String/);
  assert.match(rustOutcome, /item_count: usize/);
  assert.match(rustOutcome, /status: /);
  assert.match(rustOutcome, /effect: /);
  assert.match(rustOutcome, /duration_ms: u64/);
  assert.match(rustOutcome, /mechanism: String/);
  assert.match(rustOutcome, /stage: /);
  assert.doesNotMatch(rustOutcome, /\bpaths?\s*:/);
  assert.match(handler, /let outcome = \|result: native_drag::NativeDragResult\| StackNativeDragOutcome \{/);
  assert.match(handler, /Some\("paths"\),\s*Some\("Drag items unavailable"\.into\(\)\)/);
});

test('native code checks physical left button immediately before Shell drag on UI thread', () => {
  const start = native.slice(native.indexOf('pub(crate) fn start_native_file_drag'));
  const guard = start.indexOf('GetAsyncKeyState(VK_LBUTTON.0 as i32)');
  const drag = start.indexOf('SHDoDragDrop(', guard);
  assert.ok(guard >= 0 && drag > guard);
  assert.match(start.slice(guard, drag), /"cancelled", "none", Some\("gesture-expired"\)/);
  const command = handler.slice(handler.indexOf('pub async fn start_stack_file_drag'), handler.indexOf('#[tauri::command]', handler.indexOf('pub async fn start_stack_file_drag')));
  assert.match(command, /run_on_main_thread\(move \|\| \{/);
  assert.match(command, /native_drag::start_native_file_drag\(&paths, hwnd\)/);
  assert.match(start, /SHDoDragDrop\(\s*Some\(hwnd\),\s*&data_object,\s*None::<&IDropSource>,\s*DROPEFFECT_COPY/);
});

test('same-parent Shell object receives absolute parent and borrowed child-relative PIDLs; mixed parents explicitly unsupported', () => {
  assert.match(native, /paths\.iter\(\)\.any\(\|path\| path\.parent\(\) != Some\(parent\)\)/);
  assert.match(native, /"unsupported",\s*"none",\s*Some\("mixed-parent"\)/);
  assert.match(native, /let parent_pidl = match make_pidl\(parent\)/);
  assert.match(native, /\.map\(\|item\| unsafe \{ ILFindLastID\(item\.0\) as \*const ITEMIDLIST \}\)/);
  assert.match(native, /SHCreateDataObject\(Some\(parent_pidl\.0\), Some\(&children\), None::<&IDataObject>\)/);
  assert.match(native, /impl Drop for OwnedPidl/);
});

test('active drag suppresses popup hiding and focus loss; RAII cleanup clears flag without stealing focus', () => {
  const command = handler.slice(handler.indexOf('pub async fn start_stack_file_drag'), handler.indexOf('#[tauri::command]', handler.indexOf('pub async fn start_stack_file_drag')));
  assert.match(command, /if guard\.native_drag_active \{/);
  assert.match(command, /guard\.native_drag_active = true/);
  assert.match(command, /let hold = popup_window::NativeDragHold::new/);
  assert.match(command, /drop\(hold\)/);
  assert.match(command, /if dispatch\.is_err\(\) \{[\s\S]*?guard\.native_drag_active = false/);
  assert.match(popup, /if guard\.native_drag_active \{\s*return Ok\(\(\)\)/);
  assert.match(popup, /if guard\.native_drag_active \{\s*return true/);
  const cleanup = popup.slice(popup.indexOf('impl Drop for NativeDragHold'), popup.indexOf('fn current_time_millis()', popup.indexOf('impl Drop for NativeDragHold')));
  assert.match(cleanup, /guard\.native_drag_active = false/);
  assert.match(cleanup, /suppress_next_stack_popup_focus_loss_for_runtime_state/);
  assert.doesNotMatch(cleanup, /\.set_focus\(|\.show\(/);
});
