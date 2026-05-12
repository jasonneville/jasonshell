import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import {
  stackBrowserCreatedTextFileRenamePlan
} from '../dist-tests/lib/stackPopupViewModel.js';

const surface = readFileSync(new URL('../src/components/StackPopupSurface.svelte', import.meta.url), 'utf8');

function extractFunctionBody(source, functionName) {
  const start = source.indexOf(`async function ${functionName}()`);
  assert.notEqual(start, -1, `${functionName} function should exist`);
  const bodyStart = source.indexOf('{', start);
  assert.notEqual(bodyStart, -1, `${functionName} body should start`);

  let depth = 0;
  for (let index = bodyStart; index < source.length; index += 1) {
    const char = source[index];
    if (char === '{') {
      depth += 1;
    } else if (char === '}') {
      depth -= 1;
      if (depth === 0) {
        return source.slice(bodyStart + 1, index);
      }
    }
  }

  assert.fail(`${functionName} body should close`);
}

test('created text file rename plan selects created row and immediately enters rename editor', () => {
  assert.deepEqual(
    stackBrowserCreatedTextFileRenamePlan({
      path: 'C:\\Users\\me\\Documents\\New Text Document.txt',
      name: 'New Text Document.txt'
    }),
    {
      selectedPath: 'C:\\Users\\me\\Documents\\New Text Document.txt',
      renameDraft: 'New Text Document.txt',
      focusTarget: 'inline-editor'
    }
  );
});

test('new text file creation delegates rename behavior to the view-model helper', () => {
  const body = extractFunctionBody(surface, 'beginCreateTextFile');

  assert.match(body, /const renamePlan = stackBrowserCreatedTextFileRenamePlan\(created\);/);
  assert.match(body, /stackState = selectStackEntry\(stackState, renamePlan\.selectedPath\);/);
  assert.match(body, /renameDraft = renamePlan\.renameDraft;/);
  assert.match(body, /if \(renamePlan\.focusTarget === 'inline-editor'\) \{/);
});

test('new folder and ordinary rename still use the existing inline editor focus path', () => {
  assert.match(surface, /function beginCreateFolder\(\)[\s\S]*createFolderDraft = 'New Folder';[\s\S]*focusEditorInput\(\);/);
  assert.match(surface, /function beginRenameSelected\(\)[\s\S]*renameDraft = selectedEntry\.name;[\s\S]*focusEditorInput\(\);/);
});
