import { writeFileSync } from 'node:fs';
import { resolve, join } from 'node:path';
import { pngPixels } from '../task-gallery/png-pixels.mjs';

// Exact pre-interaction bar/chrome pixels, including an 8px shadow allowance.
// Overflow references the corrected, resize-entered baseline; never rewrite it.
const root = resolve('test-results/bottom-edge-final');
const overflowRoot = resolve('test-results/bottom-edge-overflow-corrected');
const results = [];
for (const height of [24, 32.4, 48]) for (const theme of ['dark', 'light']) {
  const samples = [960, 320].map(width => ({ name: `${height}-${theme}-${width}`, base: root }));
  samples.push({ name: `overflow-${height}-${theme}`, base: overflowRoot });
  for (const sample of samples) {
    const before = pngPixels(join(sample.base, `baseline-${sample.name}.png`));
    const after = pngPixels(join(root, `current-${sample.name}.png`));
    if (before.width !== after.width || before.height !== after.height) throw new Error(`Screenshot size changed: ${sample.name}`);
    let differingPixels = 0, maxChannelDelta = 0;
    let left = before.width, top = before.height, right = -1, bottom = -1;
    const startY = Math.max(0, Math.floor(before.height - height) - 8);
    for (let y = startY; y < before.height; y++) for (let x = 0; x < before.width; x++) {
      const a = before.pixel(x, y), b = after.pixel(x, y);
      const delta = Math.max(...a.map((value, i) => Math.abs(value - b[i])));
      if (!delta) continue;
      differingPixels++; maxChannelDelta = Math.max(maxChannelDelta, delta);
      left = Math.min(left, x); top = Math.min(top, y); right = Math.max(right, x); bottom = Math.max(bottom, y);
    }
    results.push({ name: sample.name, pass: differingPixels === 0, differingPixels, maxChannelDelta,
      bounds: differingPixels ? { left, top, right, bottom } : null, inspected: { startY, endY: before.height - 1, width: before.width } });
  }
}
const report = { total: results.length, results, failed: results.filter(r => !r.pass),
  evidenceBoundary: 'Exact Chromium screenshot RGB equality for all 18 pre-interaction bar/chrome samples, including the bottom inset and an 8px top shadow allowance. No native/DWM claim.' };
writeFileSync(join(root, 'pixel-comparison.json'), JSON.stringify(report, null, 2));
console.log(JSON.stringify(report, null, 2));
process.exitCode = report.failed.length ? 1 : 0;
