import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const pageUrl = new URL('../public/loading-spinner-concepts.html', import.meta.url);

const retainedConcepts = [
  'Prompt Pulse', 'Orbit Queue', 'Glyph Cascade', 'Terminal Wave', 'Pixel Furnace',
  'Compile Trace', 'Prompt Relay', 'Packet Matrix', 'Build Lattice', 'Shell Ticker',
  'Process Trace', 'Segment Register', 'Glyph Buffer', 'Commit Wave', 'Byte Conveyor',
  'Cursor Stack', 'Dependency Grid',
];

const circularAdditions = [
  ['Ring Register', 'ring-register'], ['Radar Sweep', 'radar-sweep'],
  ['Gear Clock', 'gear-clock'], ['Cursor Orbit', 'cursor-orbit'],
  ['Pulse Stack', 'pulse-stack'], ['Compile Carousel', 'compile-carousel'],
  ['Queue Loop', 'queue-loop'], ['Cache Head', 'cache-head'],
  ['Branch Halo', 'branch-halo'], ['Micro Arc', 'micro-arc'],
];

const rejectedConcepts = ['Workspace Sweep', 'Signal Rail', 'Twin Comets', 'Packet Steps', 'Beacon Bloom', 'Sync Aperture', 'Data Weave'];

function extractConceptCards(source) {
  return [...source.matchAll(/<article\s+class="concept-card">([\s\S]*?)<\/article>/g)].map((match) => match[0]);
}

test('standalone loading concepts page presents exactly twenty-seven curated, practical choices', () => {
  const source = readFileSync(pageUrl, 'utf8');

  assert.match(source, /<title>JasonShell Loading Concepts<\/title>/);
  assert.match(source, /<h1[^>]*>Loading, with intent\.<\/h1>/);
  assert.match(source, /Reply with the concept name/i);

  const cards = extractConceptCards(source);
  const expectedNames = [...retainedConcepts, ...circularAdditions.map(([name]) => name)];
  assert.equal(cards.length, 27);
  assert.deepEqual(cards.map((card) => card.match(/<p class="index">(\d{2})\s*\//)?.[1]), Array.from({ length: 27 }, (_, index) => String(index + 1).padStart(2, '0')));
  assert.deepEqual(cards.map((card) => card.match(/<h2>([^<]+)<\/h2>/)?.[1]), expectedNames);

  const renderedCards = cards.join('\n');
  for (const removed of rejectedConcepts) {
    assert.doesNotMatch(renderedCards, new RegExp(removed, 'i'));
  }
  for (const [offset, [name, previewClass]] of circularAdditions.entries()) {
    const card = cards[retainedConcepts.length + offset];
    assert.match(card, new RegExp(`<h2>${name}<\\/h2>`));
    assert.match(card, new RegExp(`<div class="spinner-stage" aria-hidden="true">[\\s\\S]*?class="${previewClass}"`));
  }

  for (const deadIdentifier of ['workspace-sweep', 'folder-tab', 'file-line', 'scan-beam', 'workspace-scan', 'signal-rail', 'rail-node', 'rail-light', 'rail-run', 'twin-comets', 'packet-steps', 'packet-hop', 'beacon-bloom', 'beacon-grow', 'sync-aperture', 'aperture-turn', 'data-weave', 'weave-slide']) {
    assert.doesNotMatch(source, new RegExp(`(?:\\.|@keyframes\\s+)${deadIdentifier}\\b`));
  }
  assert.match(source, /Best for quick shell actions/i);
  assert.match(source, /Best for progress-aware background work/i);
  assert.match(source, /Reply with the concept name[^<]*Orbit Queue/i);
  assert.match(source, /Twenty-seven purposeful signals/i);
  assert.match(source, /Best for package installs or compilation output/i);
  assert.match(source, /long-running remote discovery with unknown duration/i);
});

test('spinner previews are decorative and reduced motion freezes every animation', () => {
  const source = readFileSync(pageUrl, 'utf8');
  const cards = extractConceptCards(source);

  assert.equal(cards.filter((card) => /<div class="spinner-stage" aria-hidden="true">/.test(card)).length, 27);
  assert.match(source, /@media\s*\(prefers-reduced-motion:\s*reduce\)/);
  assert.match(source, /animation-play-state:\s*paused/);
  assert.match(source, /prefers-color-scheme:\s*light/);
  assert.match(source, /<meta name="color-scheme" content="dark light" \/>/);
  assert.match(source, /:root\s*{[\s\S]*?color-scheme:\s*dark/);
  assert.match(source, /@media\s*\(max-width:\s*520px\)/);
  for (const animationClass of ['compile-trace', 'prompt-relay', 'packet-matrix', 'build-lattice', 'shell-ticker', 'process-trace', 'segment-register', 'glyph-buffer', 'commit-wave', 'byte-conveyor', 'cursor-stack', 'dependency-grid', 'ring-register', 'radar-sweep', 'gear-clock', 'cursor-orbit', 'pulse-stack', 'compile-carousel', 'queue-loop', 'cache-head', 'branch-halo', 'micro-arc']) {
    assert.match(source, new RegExp(`\\.${animationClass}\\b[\\s\\S]*?animation:`));
  }
});
