import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import ts from 'typescript';

const surface = readFileSync(new URL('../src/components/StackPopupSurface.svelte', import.meta.url), 'utf8');
const surfaceCss = readFileSync(new URL('../src/components/StackPopupSurface.css', import.meta.url), 'utf8');
const editor = readFileSync(new URL('../src/components/StackTextEditor.svelte', import.meta.url), 'utf8');
const adapter = readFileSync(new URL('../src/features/stack-browser/stackTextEditorAdapter.ts', import.meta.url), 'utf8');
const appCss = readFileSync(new URL('../src/app.css', import.meta.url), 'utf8');
const languageRegistryUrl = new URL('../src/features/stack-browser/stackTextEditorLanguages.ts', import.meta.url);
const languageRegistry = (() => {
  try { return readFileSync(languageRegistryUrl, 'utf8'); } catch { return ''; }
})();

async function importExitState() {
  const source = readFileSync(new URL('../src/features/stack-browser/basicTextEditorExit.ts', import.meta.url), 'utf8');
  const javascript = ts.transpileModule(source, {
    compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 }
  }).outputText;
  return import(`data:text/javascript;base64,${Buffer.from(javascript).toString('base64')}`);
}

async function importLanguageRegistry() {
  const javascript = ts.transpileModule(languageRegistry, {
    compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 }
  }).outputText;
  return import(`data:text/javascript;base64,${Buffer.from(javascript).toString('base64')}`);
}

async function importEditorAdapter() {
  let javascript = ts.transpileModule(adapter, {
    compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 }
  }).outputText;
  const languageStub = `data:text/javascript,${encodeURIComponent("export async function installStackTextEditorLanguage() {}\nexport function stackTextEditorExtension() { return ''; }")}`;
  const foldControlStub = `data:text/javascript,${encodeURIComponent('export function createStackFoldMarker() { return document.createElement("button"); }')}`;
  javascript = javascript.replace("'./stackTextEditorLanguages'", JSON.stringify(languageStub));
  javascript = javascript.replace("'./stackFoldControl'", JSON.stringify(foldControlStub));
  javascript = javascript.replace(/from '(@[^']+)'/g, (_match, specifier) => (
    `from ${JSON.stringify(import.meta.resolve(specifier))}`
  ));
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
  assert.match(editor, /\.stack-text-editor-host\s*\{[^}]*min-height:\s*0;[^}]*overflow:\s*hidden;/s);
});

test('editor exposes loading, errors, file context, labelled CodeMirror host, and explicit close', () => {
  assert.match(editor, /readStackBasicTextFile\(filePath\)/);
  assert.match(editor, /Draft only — saving is not available yet/);
  assert.match(editor, /<div[\s\S]*bind:this=\{editorHost\}[\s\S]*class="stack-text-editor-host"[\s\S]*role="group"[\s\S]*aria-label="File contents"/);
  assert.match(editor, /\{#if loading\}[\s\S]*Loading file…/);
  assert.match(editor, /role="alert"/);
  assert.match(editor, /onDismiss/);
  assert.doesNotMatch(editor, />\s*Save\s*</);
});

test('each successful file load focuses editor at document and viewport start after paint', () => {
  assert.match(
    editor,
    /initialContent = result\.content;[\s\S]*await tick\(\);[\s\S]*const mountedAdapter = createStackTextEditorAdapter\([\s\S]*editorAdapter = mountedAdapter;[\s\S]*await new Promise<void>\(\(resolve\) => window\.requestAnimationFrame\(\(\) => resolve\(\)\)\);\s*if \(disposed \|\| sequence !== requestSequence \|\| filePath !== path \|\| editorAdapter !== mountedAdapter\) return;\s*mountedAdapter\.focusAtStart\(\);/
  );
});

test('component creates adapter only for current successful load and destroys every prior view', () => {
  assert.match(editor, /import \{[\s\S]*createStackTextEditorAdapter[\s\S]*stackTextEditorAdapter/);
  assert.match(editor, /function destroyEditor\(\)[\s\S]*editorAdapter\?\.destroy\(\);[\s\S]*editorAdapter = null;/);
  assert.match(editor, /const sequence = \+\+requestSequence;[\s\S]*destroyEditor\(\);/);
  assert.match(editor, /if \(disposed \|\| sequence !== requestSequence \|\| filePath !== path\) return;[\s\S]*createStackTextEditorAdapter/);
  assert.match(editor, /onDestroy\(\(\) => \{[\s\S]*disposed = true;[\s\S]*requestSequence \+= 1;[\s\S]*destroyEditor\(\);/);
});

test('adapter explicitly composes CodeMirror draft features and owns dirty Escape routing', () => {
  assert.match(adapter, /EditorState\.create/);
  assert.match(adapter, /new EditorView/);
  for (const extension of ['history()', 'lineNumbers()', 'highlightActiveLine()', 'drawSelection()', 'bracketMatching()']) {
    assert.ok(adapter.includes(extension), `missing explicit ${extension}`);
  }
  assert.match(adapter, /keymap\.of\(\[[\s\S]*\.\.\.defaultKeymap[\s\S]*\.\.\.historyKeymap[\s\S]*\.\.\.searchKeymap/);
  assert.match(adapter, /EditorView\.updateListener\.of\([\s\S]*update\.docChanged[\s\S]*onChange\(draft, dirty\)/);
  assert.match(adapter, /EditorView\.domEventHandlers\(\{[\s\S]*keydown\(event\)[\s\S]*event\.key !== 'Escape'[\s\S]*event\.preventDefault\(\)[\s\S]*event\.stopPropagation\(\)[\s\S]*onDismiss\(dirty\)/);
  assert.match(adapter, /destroy\(\) \{[\s\S]*view\.destroy\(\)/);
  assert.doesNotMatch(adapter, /basicSetup|autocompletion|linter/);
});

test('Stack editor font is preference-driven and reconfigures mounted view without recreation', () => {
  assert.match(adapter, /fontStack: string/);
  assert.match(adapter, /setFont\(fontStack: string\): void/);
  assert.match(adapter, /const fontCompartment = new Compartment\(\)/);
  assert.match(adapter, /fontCompartment\.of\(stackEditorFontTheme\(fontStack\)\)/);
  assert.match(adapter, /view\.dispatch\(\{ effects: fontCompartment\.reconfigure\(stackEditorFontTheme\(fontStack\)\) \}\)/);
  assert.doesNotMatch(adapter, /fontFamily:\s*'"Google Sans Code"/);
  assert.match(editor, /addShellPreferencesChangeListener/);
  assert.match(editor, /stackEditorFontById\(preferences\.stackEditorFontId\)\.stack/);
  assert.match(editor, /fontStack: stackEditorFontStack/);
  assert.match(editor, /editorAdapter\?\.setFont\(stackEditorFontStack\)/);
  assert.match(editor, /const removePreferencesListener = addShellPreferencesChangeListener/);
  assert.match(editor, /onDestroy\(\(\) => \{[\s\S]*removePreferencesListener\(\);[\s\S]*destroyEditor\(\);/);
});

test('Stack editor font theme targets CodeMirror rendered text scroller', async () => {
  const { stackEditorFontTheme } = await importEditorAdapter();
  const extension = stackEditorFontTheme('Regression Face, monospace');
  const rules = extension.flatMap((part) => part?.value?.rules ?? []);

  assert.ok(
    rules.some((rule) => /\.cm-scroller\s*\{font-family:\s*Regression Face, monospace;\}/.test(rule)),
    `expected actual CodeMirror theme rule to target .cm-scroller; got ${JSON.stringify(rules)}`
  );
});

test('Stack editor theme renders a themed terminal-style block cursor', async () => {
  const { stackEditorTheme } = await importEditorAdapter();
  assert.ok(stackEditorTheme, 'Stack editor theme must be inspectable');
  const rules = stackEditorTheme.flatMap((part) => part?.value?.rules ?? []);
  const cursorRule = rules.find((rule) => /\.cm-cursor\s*\{/.test(rule));

  assert.ok(cursorRule, `expected actual CodeMirror theme rule for .cm-cursor; got ${JSON.stringify(rules)}`);
  assert.match(cursorRule, /border-left:\s*0;/);
  assert.match(cursorRule, /background-color:\s*color-mix\(in srgb, var\(--js-color-text-strong\) 84%, var\(--js-bg-surface\)\);/);
  assert.match(cursorRule, /width:\s*0\.62em;/);
});

test('Stack editor uses dedicated stronger focused and quieter inactive selection backplates', async () => {
  const { stackEditorTheme } = await importEditorAdapter();
  const rules = stackEditorTheme.flatMap((part) => part?.value?.rules ?? []);
  const inactiveRule = rules.find((rule) => /\.cm-selectionBackground\s*\{/.test(rule) && !/cm-focused/.test(rule));
  const focusedRule = rules.find((rule) => /cm-focused[^{}]*\.cm-selectionBackground\s*\{/.test(rule));

  assert.match(inactiveRule ?? '', /background-color:\s*var\(--js-stack-editor-selection-inactive\);/);
  assert.match(focusedRule ?? '', /background-color:\s*var\(--js-stack-editor-selection\);/);
  assert.doesNotMatch(`${inactiveRule}\n${focusedRule}`, /--js-color-(?:accent-border|selected)/);
  assert.match(appCss, /--js-stack-editor-selection:\s*color-mix\(in srgb, var\(--js-bg-surface\) 70%, var\(--js-color-accent\)\);/);
  assert.match(appCss, /--js-stack-editor-selection-inactive:\s*color-mix\(in srgb, var\(--js-bg-surface\) 84%, var\(--js-color-accent\)\);/);
  assert.doesNotMatch(appCss, /--js-stack-editor-selection(?:-inactive)?:\s*[^;]*transparent/);
  assert.match(appCss, /@media \(prefers-contrast: more\), \(forced-colors: active\)[\s\S]*--js-stack-editor-selection:\s*Highlight;[\s\S]*--js-stack-editor-selection-inactive:\s*[^;]+;/);
  assert.doesNotMatch(adapter, /(?:\.cm-selectionBackground[^}]*color:|::selection)/s);
  assert.match(adapter, /drawSelection\(\)/);
});

test('editor derives calm local foregrounds for content, caret, and syntax without adding capabilities', () => {
  assert.match(adapter, /const calmEditorForeground = 'color-mix\(in srgb, var\(--js-color-text\) \d+%, var\(--js-bg-surface\)\)'/);
  assert.match(adapter, /const calmEditorCaret = 'color-mix\(in srgb, var\(--js-color-text-strong\) \d+%, var\(--js-bg-surface\)\)'/);
  assert.match(adapter, /const calmEditorAccent = 'color-mix\(in srgb, var\(--js-color-accent\) \d+%, var\(--js-color-text\)\)'/);
  assert.match(adapter, /'&': \{[\s\S]*color: calmEditorForeground/);
  assert.match(adapter, /'\.cm-content': \{ caretColor: calmEditorCaret/);
  assert.match(adapter, /tags\.keyword, color: calmEditorAccent/);
  assert.match(adapter, /tags\.name, tags\.propertyName, tags\.attributeName\], color: calmEditorForeground/);
  assert.match(adapter, /tags\.string, tags\.inserted\], color: calmEditorAccent/);
  assert.doesNotMatch(adapter, /autocompletion|linter|localStorage|writeStack|saveStack|invoke\(/);
});

test('language registry routes supported extensions case-insensitively and leaves plain text plain', () => {
  for (const extension of ['json', 'js', 'ts', 'svelte', 'css', 'html', 'xml', 'yaml', 'yml', 'md']) {
    assert.match(languageRegistry, new RegExp(`['"]${extension}['"]`), `missing ${extension} route`);
  }
  assert.match(languageRegistry, /toLowerCase\(\)/);
  assert.match(languageRegistry, /[\\/]\/|split\(\/\[\\\\\/\]\//);
  assert.doesNotMatch(languageRegistry, /['"](?:txt|csv|log)['"]\s*:/);
});

test('language registry executes normalized routes and plain fallback', async () => {
  const { stackTextEditorExtension, installStackTextEditorLanguage } = await importLanguageRegistry();
  assert.equal(stackTextEditorExtension('C:\\Mixed/Folder\\FILE.Ts'), 'ts');
  assert.equal(stackTextEditorExtension('C:/Folder/README'), '');

  const applied = [];
  let loaderCalls = 0;
  await installStackTextEditorLanguage('C:\\Mixed/Folder\\FILE.Ts', {
    load: async (extension) => {
      loaderCalls += 1;
      assert.equal(extension, 'ts');
      return 'typescript-extension';
    },
    apply: (extension) => applied.push(extension),
    isDestroyed: () => false
  });
  await installStackTextEditorLanguage('notes.LOG', {
    load: async () => { loaderCalls += 1; return 'unexpected'; },
    apply: (extension) => applied.push(extension),
    isDestroyed: () => false
  });
  assert.equal(loaderCalls, 1);
  assert.deepEqual(applied, ['typescript-extension']);
});

test('language install swallows loader rejection and ignores completion after destroy', async () => {
  const { installStackTextEditorLanguage } = await importLanguageRegistry();
  const applied = [];
  await assert.doesNotReject(installStackTextEditorLanguage('data.json', {
    load: async () => { throw new Error('optional parser unavailable'); },
    apply: (extension) => applied.push(extension),
    isDestroyed: () => false
  }));

  let resolveLanguage;
  let destroyed = false;
  const deferred = new Promise((resolve) => { resolveLanguage = resolve; });
  const installation = installStackTextEditorLanguage('view.SVELTE', {
    load: () => deferred,
    apply: (extension) => applied.push(extension),
    isDestroyed: () => destroyed
  });
  destroyed = true;
  resolveLanguage('svelte-extension');
  await installation;
  assert.deepEqual(applied, []);
});

test('adapter composes explicit token theme and installs language asynchronously with stale safety', () => {
  assert.match(adapter, /HighlightStyle\.define/);
  assert.match(adapter, /syntaxHighlighting\(/);
  assert.match(adapter, /languageCompartment/);
  assert.match(adapter, /installStackTextEditorLanguage\(path/);
  assert.match(adapter, /isDestroyed: \(\) => destroyed/);
  assert.match(adapter, /languageCompartment\.reconfigure/);
  assert.doesNotMatch(adapter, /--js-color-(?:success|warning|accent-strong|danger)/);
  assert.match(editor, /path: filePath/);
  assert.doesNotMatch(adapter, /await\s+loadStackTextEditorLanguage/);
});

test('structured editing excludes completion, save, and persistence capabilities', () => {
  const combined = `${editor}\n${adapter}\n${languageRegistry}`;
  assert.doesNotMatch(combined, /autocompletion|linter|localStorage|writeStack|saveStack|invoke\(/);
});

test('editor has no persistence or test-only product dependency', () => {
  const combined = `${surface}\n${editor}\n${adapter}`;
  assert.doesNotMatch(combined, /TextEditorProjectionExperiment|P03|P04|modern-editor-features-research-plan/);
  assert.doesNotMatch(combined, /localStorage|writeStack|saveStack|invoke\(/);
});

test('every editor exit routes through one parent-owned dirty-draft guard', () => {
  assert.match(editor, /export let onDirtyChange: \(dirty: boolean\) => void;/);
  assert.match(editor, /export let onDismiss: \(dirty: boolean\) => void;/);
  assert.match(editor, /onClick=\{\(\) => onDismiss\(dirty\)\}/);
  assert.match(editor, /onDismiss: \(\) => onDismiss\(dirty\)/);
  assert.doesNotMatch(editor, /onDismiss: \(currentDirty\) => onDismiss\(currentDirty\)/);

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
