import assert from 'node:assert/strict';
import { existsSync, readdirSync, readFileSync, statSync } from 'node:fs';
import { join } from 'node:path';
import { relative } from 'node:path';
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

// Leader-approved visual contracts are declaration-specific, never file-wide.
// The remaining content of these selectors/files still passes the same ban.
const approvedGradientDeclarations = {
  'src/components/TaskGallerySurface.css': { selector: '.task-gallery-panel', property: 'background', value:
    'var(--js-bg-surface) linear-gradient( color-mix(in srgb, var(--js-bg-surface) 88%, var(--js-color-accent)), var(--js-bg-surface) )' },
  'src/components/TaskPreviewSurface.css': { selector: '.preview-surface', property: '--preview-chrome', value:
    'linear-gradient( color-mix(in srgb, var(--js-bg-surface) 88%, var(--js-color-accent)), var(--js-bg-surface) )' },
  'src/components/SpeechIndicatorSurface.svelte': { selector: '.mic-glyph', property: 'background', value:
    'linear-gradient(to bottom, #162d49 0%, #335371 calc((1 - var(--mic-level)) * (1 - 0.2 * var(--mic-level)) * 100%), #a3c2e0 calc((1 - var(--mic-level)) * (1 + 0.08 * var(--mic-level)) * 100%), #d7ebfa 100%)' }
};
const normalizedWhitespace = value => value.replace(/\s+/g, ' ').trim();
function assertGradientPolicy(path, text) {
  const approved = approvedGradientDeclarations[path];
  if (approved) {
    const escaped = approved.selector.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
    let removed = 0;
    text = text.replace(new RegExp(`(^|\\n)(\\s*${escaped}\\s*\\{)([^{}]*)(\\})`), (whole, prefix, open, body, close) => {
      const remaining = body.replace(/([\w-]+)\s*:\s*([^;]+);/g, (declaration, property, value) => {
        if (property === approved.property && normalizedWhitespace(value) === approved.value) { removed++; return ''; }
        return declaration;
      });
      return prefix + open + remaining + close;
    });
    assert.equal(removed, 1, `${path}: approved declaration must retain its exact contract`);
  }
  assert.doesNotMatch(text, /\b(?:linear-gradient|radial-gradient|conic-gradient|repeating-linear-gradient|repeating-radial-gradient|background-image)\b|--js-gradient-/, path);
}

test('UI gradients are prohibited except exact approved outer chrome, preview and speech artwork declarations', () => {
  const files = [
    fileURLToPath(new URL('src/app.css', repoRoot)),
    fileURLToPath(new URL('src/App.svelte', repoRoot)),
    ...collectUiFiles(fileURLToPath(new URL('src/components', repoRoot)))
  ];
  for (const file of files) {
    const path = relative(fileURLToPath(repoRoot), file).replaceAll('\\', '/');
    assertGradientPolicy(path, readFileSync(file, 'utf8'));
  }
});

test('approved gradient selectors do not allow unknown extra gradients or altered approved declarations', () => {
  for (const path of Object.keys(approvedGradientDeclarations)) {
    const original = source(path);
    assertGradientPolicy(path, original);
    assert.throws(() => assertGradientPolicy(path, original + '\n.unapproved { background: linear-gradient(red, blue); }'));
    assert.throws(() => assertGradientPolicy(path, original.replace('linear-gradient(', 'radial-gradient(')));
    const selector = approvedGradientDeclarations[path].selector;
    assert.throws(() => assertGradientPolicy(path, original.replace(`${selector} {`, `${selector} { background-image: linear-gradient(red, blue);`)));
  }
  assert.throws(() => assertGradientPolicy('src/components/Unapproved.svelte', '.test { background: linear-gradient(red, blue); }'));
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
