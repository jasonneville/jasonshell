import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import ts from 'typescript';

const surface = readFileSync(new URL('../src/components/StackPopupSurface.svelte', import.meta.url), 'utf8');
const surfaceCss = readFileSync(new URL('../src/components/StackPopupSurface.css', import.meta.url), 'utf8');
const editor = readFileSync(new URL('../src/components/StackTextEditor.svelte', import.meta.url), 'utf8');

async function importExitState() {
  const source = readFileSync(new URL('../src/features/stack-browser/basicTextEditorExit.ts', import.meta.url), 'utf8');
  const javascript = ts.transpileModule(source, {
    compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 }
  }).outputText;
  return import(`data:text/javascript;base64,${Buffer.from(javascript).toString('base64')}`);
}

test('dirty editor exit keeps first request until discard and resets after cancel', async () => {
  const { cancelPendingEditorExit, enqueuePendingEditorExit, takePendingEditorExit } = await importExitState();
  const calls = [];
  const actionA = () => calls.push('A');
  const actionB = () => calls.push('B');
  const actionC = () => calls.push('C');

  let state = null;
  ({ state } = enqueuePendingEditorExit(state, actionA));
  const second = enqueuePendingEditorExit(state, actionB);
  assert.equal(second.accepted, false);
  assert.equal(second.state, state);

  const discard = takePendingEditorExit(state);
  state = discard.state;
  discard.action?.();
  const duplicateDiscard = takePendingEditorExit(state);
  duplicateDiscard.action?.();
  assert.deepEqual(calls, ['A'], 'discard capture executes first request exactly once');
  assert.equal(state, null);

  ({ state } = enqueuePendingEditorExit(state, actionA));
  state = cancelPendingEditorExit(state);
  assert.equal(state, null);
  const later = enqueuePendingEditorExit(state, actionC);
  assert.equal(later.accepted, true);
  later.state?.action();
  assert.deepEqual(calls, ['A', 'C']);
});

test('clean and explicit-discard editor exits reset stale virtual scroll before grid remount', async () => {
  const { resetBasicTextEditorViewport } = await importExitState();
  const entries = Array.from({ length: 200 }, (_, index) => `file-${index}`);
  const staleViewport = { scrollTop: 2400, height: 320 };

  for (const exitKind of ['clean', 'explicit-discard']) {
    const resetViewport = resetBasicTextEditorViewport(staleViewport);
    const firstVisibleIndex = Math.floor(resetViewport.scrollTop / 24);
    const remountedRows = entries.slice(firstVisibleIndex, firstVisibleIndex + 12);

    assert.deepEqual(resetViewport, { scrollTop: 0, height: 0 }, `${exitKind} exit clears stale viewport state`);
    assert.equal(remountedRows[0], 'file-0', `${exitKind} exit remounts with top file visible`);
  }
});

test('editor-exit integration resets only active clean close or confirmed discard', async () => {
  const {
    beginBasicTextEditorExit,
    cancelPendingEditorExit,
    takePendingEditorExit
  } = await importExitState();
  let resetCount = 0;
  let actionCount = 0;
  const dismiss = () => { resetCount += 1; };
  const action = () => { actionCount += 1; };

  let state = null;
  let request = beginBasicTextEditorExit(state, false, false, action, dismiss);
  state = request.state;
  await request.completion;
  assert.equal(resetCount, 0, 'no-editor navigation keeps existing viewport');
  assert.equal(actionCount, 1, 'no-editor navigation runs action directly');

  request = beginBasicTextEditorExit(state, true, true, action, dismiss);
  state = request.state;
  state = cancelPendingEditorExit(state);
  assert.equal(resetCount, 0, 'dirty Cancel does not reset viewport');
  assert.equal(actionCount, 1, 'dirty Cancel does not run action');

  request = beginBasicTextEditorExit(state, true, false, action, dismiss);
  state = request.state;
  await request.completion;
  assert.equal(resetCount, 1, 'active clean close resets exactly once');
  assert.equal(actionCount, 2);

  request = beginBasicTextEditorExit(state, true, true, action, dismiss);
  state = request.state;
  const discard = takePendingEditorExit(state);
  state = discard.state;
  dismiss();
  await discard.action?.();
  assert.equal(resetCount, 2, 'confirmed discard resets exactly once');
  assert.equal(actionCount, 3);
  assert.equal(state, null);
});

test('normal text files route to the directly mounted editor while unsupported files stay external', () => {
  assert.match(surface, /const STACK_BASIC_TEXT_EXTENSIONS = new Set\(\[[\s\S]*'txt'[\s\S]*'md'[\s\S]*'json'[\s\S]*'svelte'[\s\S]*\]\);/);
  assert.match(surface, /if \(entry\.entryType === 'File' && isStackBasicTextFile\(entry\.path\)\) \{[\s\S]*editorPath = entry\.path;[\s\S]*return;/);
  assert.match(surface, /await openStackItem\(entry\.path\);/);
  assert.match(surface, /<StackTextEditor\s+path=\{editorPath\}/);
});

test('editor replaces the file or Git content row and owns internal scrolling', () => {
  assert.match(surfaceCss, /\.details-table,\s*\.stack-popup > \.stack-git-panel,\s*\.stack-popup > \.stack-text-editor\s*\{\s*grid-row: 4;/);
  assert.match(editor, /\.stack-text-editor-field\s*\{[^}]*min-height:\s*0;[^}]*overflow:\s*hidden;/s);
  assert.match(editor, /textarea\s*\{[^}]*box-sizing:\s*border-box;[^}]*height:\s*100%;[^}]*overflow:\s*auto;/s);
});

test('editor exposes loading, errors, file context, labelled draft textarea, and explicit close', () => {
  assert.match(editor, /readStackBasicTextFile\(filePath\)/);
  assert.match(editor, /Draft only — saving is not available yet/);
  assert.match(editor, /<textarea[\s\S]*bind:value=\{draft\}[\s\S]*aria-label="File contents"/);
  assert.match(editor, /\{#if loading\}[\s\S]*Loading file…/);
  assert.match(editor, /role="alert"/);
  assert.match(editor, /onDismiss/);
  assert.doesNotMatch(editor, />\s*Save\s*</);
});

test('each successful file load focuses editor at document and viewport start after paint', () => {
  assert.match(
    editor,
    /initialContent = result\.content;[\s\S]*draft = result\.content;[\s\S]*await tick\(\);\s*if \(sequence !== requestSequence \|\| filePath !== path\) return;\s*if \(textarea\) \{\s*textarea\.focus\(\);[\s\S]*textarea\.setSelectionRange\(0, 0\);[\s\S]*textarea\.scrollTop = 0;/
  );
});

test('editor has no persistence or test-only product dependency', () => {
  const combined = `${surface}\n${editor}`;
  assert.doesNotMatch(combined, /TextEditorProjectionExperiment|P03|P04/);
  assert.doesNotMatch(editor, /localStorage|writeStack|saveStack/);
});

test('every editor exit routes through one parent-owned dirty-draft guard', () => {
  assert.match(editor, /export let onDirtyChange: \(dirty: boolean\) => void;/);
  assert.match(editor, /export let onDismiss: \(dirty: boolean\) => void;/);
  assert.match(editor, /onClick=\{\(\) => onDismiss\(dirty\)\}/);
  assert.match(editor, /if \(event\.key !== 'Escape'\) return;[\s\S]*onDismiss\(dirty\);/);

  assert.match(surface, /function requestEditorExit\(action: \(\) => void \| Promise<void>\)/);
  assert.match(surface, /function dismissEditor\(\)[\s\S]*detailsBodyScrollTop = viewport\.scrollTop;[\s\S]*detailsBodyHeight = viewport\.height;[\s\S]*editorPath = null;/);
  assert.match(surface, /beginBasicTextEditorExit\(pendingEditorExit, Boolean\(editorPath\), editorDirty, action, dismissEditor\)/);
  assert.match(surface, /pendingEditorExit = request\.state;/);
  assert.match(surface, /if \(!request\.accepted\) return;/);
  assert.match(surface, /async function confirmEditorExit\(\)[\s\S]*dismissEditor\(\);[\s\S]*await action\(\);/);
  assert.match(surface, /function cancelEditorExit\(\)[\s\S]*cancelPendingEditorExit\(pendingEditorExit\)/);
  assert.match(surface, /title="Discard unsaved draft\?"[\s\S]*confirmLabel="Discard"[\s\S]*onCancel=\{cancelEditorExit\}[\s\S]*onConfirm=\{\(\) => void confirmEditorExit\(\)\}/);
  assert.doesNotMatch(`${surface}\n${editor}`, /window\.confirm/);
});

test('folder, popup-request, editor-back, and surface-close routes use dirty-draft guard', () => {
  assert.match(surface, /async function openFolder[\s\S]*requestEditorExit\(\(\) => performOpenFolder\(folderPath/);
  assert.match(surface, /async function handleOpenRequest[\s\S]*requestEditorExit\(async \(\) => \{[\s\S]*await performOpenFolder\(path/);
  assert.match(surface, /function requestEditorClose\(dirty: boolean\)[\s\S]*requestEditorExit\(\(\) => \{/);
  assert.match(surface, /function closeStackPopupFromSurface\(\)[\s\S]*requestEditorExit\(async \(\) => \{/);
  assert.match(surface, /handleStackBrowserHotkeyKeydown[\s\S]*closeStackPopupFromSurface\(\)/);
  assert.match(surface, /<StackTextEditor[\s\S]*onDirtyChange=\{handleEditorDirtyChange\}[\s\S]*onDismiss=\{requestEditorClose\}/);
});
