import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import ts from 'typescript';

const parserUrl = new URL('../src/features/stack-browser/stackMarkdownPreview.ts', import.meta.url);
const componentUrl = new URL('../src/components/StackMarkdownPreview.svelte', import.meta.url);
const editor = readFileSync(new URL('../src/components/StackTextEditor.svelte', import.meta.url), 'utf8');

async function importParser() {
  const source = readFileSync(parserUrl, 'utf8');
  const javascript = ts.transpileModule(source, {
    compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 }
  }).outputText;
  return import(`data:text/javascript;base64,${Buffer.from(javascript).toString('base64')}`);
}

test('Markdown parser supports document structures and inline emphasis', async () => {
  const { parseStackMarkdown } = await importParser();
  const blocks = parseStackMarkdown(`# Title\n\nIntro with **weight** and \`code\`.\n\n- one\n- two\n\n1) first\n2. second\n\n\`\`\`ts\nconst x = 1;\n\`\`\`\n\n| Name | Value |\n| --- | :---: |\n| A | B |`);

  assert.deepEqual(blocks.map((block) => block.type), ['heading', 'paragraph', 'list', 'list', 'code', 'table']);
  assert.equal(blocks[0].level, 1);
  assert.deepEqual(blocks[1].content.map((part) => part.type), ['text', 'strong', 'text', 'code', 'text']);
  assert.equal(blocks[3].ordered, true);
  assert.equal(blocks[4].language, 'ts');
  assert.deepEqual(blocks[5].alignments, [null, 'center']);
});

test('Markdown parser handles h1-h3, empty input, and unterminated fences', async () => {
  const { parseStackMarkdown } = await importParser();
  assert.deepEqual(parseStackMarkdown(''), []);
  assert.deepEqual(parseStackMarkdown('# A\n## B\n### C').map((block) => block.level), [1, 2, 3]);
  assert.deepEqual(parseStackMarkdown('```js\nalert(1)').at(0), {
    type: 'code', language: 'js', code: 'alert(1)'
  });
});

test('Markdown editor defaults to safe Preview and preserves draft across explicit mode switches', () => {
  const component = readFileSync(componentUrl, 'utf8');
  assert.match(editor, /markdown = isMarkdownPath\(filePath\)/);
  assert.match(editor, /editorMode = markdown \? 'preview' : 'edit'/);
  assert.match(editor, /aria-label="Markdown view"/);
  assert.match(editor, /aria-pressed=\{editorMode === 'preview'\}/);
  assert.match(editor, /aria-pressed=\{editorMode === 'edit'\}/);
  assert.match(editor, /<StackMarkdownPreview source=\{draft\}/);
  assert.match(editor, /content: draft/);
  assert.doesNotMatch(`${editor}\n${component}`, /\{@html\}|innerHTML|DOMParser|fetch\(/);
  assert.match(component, /<article[^>]*aria-label="Markdown preview"/);
  for (const element of ['<h1>', '<h2>', '<h3>', '<table>', '<pre><code>']) assert.ok(component.includes(element));
});

test('dirty Markdown Preview -> Edit -> Escape uses component dirty state, not remounted adapter baseline', () => {
  assert.match(
    editor,
    /async function mountDraftEditor[\s\S]*content: draft,[\s\S]*onDismiss: \(\) => onDismiss\(dirty\)/
  );
  assert.doesNotMatch(editor, /onDismiss: \(currentDirty\) => onDismiss\(currentDirty\)/);
  assert.match(editor, /\$: dirty = draft !== initialContent/);
  assert.match(editor, /onClick=\{\(\) => onDismiss\(dirty\)\}/);
  assert.match(editor, /editorMode = markdown \? 'preview' : 'edit'/);
});
