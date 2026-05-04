import assert from 'node:assert/strict';
import { existsSync, readdirSync, readFileSync, statSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';

const repoRoot = new URL('..', import.meta.url);

function source(path) {
  return readFileSync(new URL(path, repoRoot), 'utf8');
}

function collectUiFiles(dir) {
  if (!existsSync(dir)) {
    return [];
  }
  const entries = [];
  for (const name of readdirSync(dir)) {
    const path = join(dir, name);
    const stat = statSync(path);
    if (stat.isDirectory()) {
      entries.push(...collectUiFiles(path));
    } else if (/\.(css|svelte)$/.test(name)) {
      entries.push(path);
    }
  }
  return entries;
}

test('app and component UI sources do not use active gradients', () => {
  const files = [
    fileURLToPath(new URL('src/app.css', repoRoot)),
    fileURLToPath(new URL('src/App.svelte', repoRoot)),
    ...collectUiFiles(fileURLToPath(new URL('src/components', repoRoot)))
  ];
  const forbidden = /\b(?:linear-gradient|radial-gradient|conic-gradient|repeating-linear-gradient|repeating-radial-gradient|background-image)\b|--js-gradient-/;

  for (const file of files) {
    assert.doesNotMatch(readFileSync(file, 'utf8'), forbidden, file);
  }
});

test('stack browser exposes editable path input and clickable segment navigation', () => {
  const stackPopupSource = source('src/components/StackPopupSurface.svelte');

  assert.match(stackPopupSource, /class="stack-path-editor"[\s\S]*on:submit\|preventDefault=\{\(\) => void submitPathDraft\(\)\}/);
  assert.match(stackPopupSource, /const listing = await listStackFolder\(folderPath(?:, async \(page\) => \{[\s\S]*?\})?\)[\s\S]*commitValidatedStackFolderListing\(stackState, folderPath, listing\)/);
  assert.match(stackPopupSource, /<input[\s\S]*aria-label="Current folder path"[\s\S]*value=\{pathDraft\}[\s\S]*on:keydown=\{handlePathKeydown\}/);
  assert.match(stackPopupSource, /function handlePathKeydown\(event: KeyboardEvent\)[\s\S]*event\.key === 'Escape'[\s\S]*resetPathDraft\(\)/);
  assert.match(stackPopupSource, /<nav class="path-segments" aria-label="Path segments">/);
  assert.match(stackPopupSource, /<MeltActionButton class="path-segment"[\s\S]*onClick=\{\(\) => void openFolder\(crumb\.path\)\}/);
  assert.doesNotMatch(stackPopupSource, /<nav class="breadcrumbs"/);
});

test('stack browser sort headers expose active class helper and aria sort wiring', () => {
  const stackPopupSource = source('src/components/StackPopupSurface.svelte');

  assert.match(stackPopupSource, /stackSortHeaderState/);
  assert.match(stackPopupSource, /function sortHeader\(column: StackSortColumn\)/);
  for (const [column, index] of [['name', 1], ['type', 2], ['size', 3], ['modified', 4]]) {
    assert.match(
      stackPopupSource,
      new RegExp(`<MeltActionButton class=\\{sortHeader\\('${column}'\\)\\.className\\} role="columnheader" ariaColindex=\\{${index}\\} ariaSort=\\{sortHeader\\('${column}'\\)\\.ariaSort\\} onClick=\\{\\(\\) => sortBy\\('${column}'\\)\\}`)
    );
  }
  assert.match(stackPopupSource, /<span class="sort-indicator" aria-hidden="true">\{sortHeader\('name'\)\.indicator\}<\/span>/);
});
