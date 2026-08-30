import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const selectedVisualTests = [
  {
    file: 'taskPreviewTextPolish.test.mjs',
    brittleLiteral: /assert\.match\(closeButtonRule, \/height:\\s\*1\\\.35rem\//,
    replacementExpectation: /remDeclaration\(headerRule, 'padding-right'\)\s*>=\s*remDeclaration\(closeButtonRule, 'min-width'\)/
  },
  {
    file: 'taskbarUxState.test.mjs',
    brittleLiteral: /#ffd54f|inset 0 4px 0/,
    replacementExpectation: /attentionShadow[\s\S]*attention cue stays inside task button bounds/
  }
];

function testSource(file) {
  return readFileSync(new URL(file, import.meta.url), 'utf8');
}

test('selected brittle visual tests remove exact CSS literals', () => {
  for (const { file, brittleLiteral } of selectedVisualTests) {
    assert.doesNotMatch(testSource(file), brittleLiteral, `${file} must not freeze selected CSS literals`);
  }
});

test('selected brittle visual tests require behavioral replacement before literal visual assertions are removed', () => {
  for (const { file, replacementExpectation } of selectedVisualTests) {
    const source = testSource(file);
    assert.match(source, replacementExpectation, `${file} must keep semantic replacement evidence`);
  }
});

test('modernization policy keeps security event and capability invariants out of broad regex purge', () => {
  const registry = testSource('sourceContractIntentRegistry.test.mjs');

  assert.match(registry, /securityBoundary:\s*\[/);
  assert.match(registry, /registryParity:\s*\[/);
  assert.match(registry, /source-contract registry keeps boundary tests separate from behavior legacy guards/);
  assert.doesNotMatch(registry, /sourceLikeTestFiles\(\)\.filter[\s\S]{0,120}securityBoundary/);
});

test('modernization policy stays focused on selected visual tests rather than banning all source regex tests', () => {
  const thisSource = testSource('testModernizationPolicy.test.mjs');

  assert.match(thisSource, /taskPreviewTextPolish\.test\.mjs/);
  assert.match(thisSource, /taskbarUxState\.test\.mjs/);
  assert.doesNotMatch(thisSource, /readdirSync\(new URL\('\.', import\.meta\.url\)\)/);
});
