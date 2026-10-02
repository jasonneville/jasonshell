import assert from 'node:assert/strict';
import test from 'node:test';

// Production contract: export createSpeechIndicatorMeter() from
// src/lib/speechIndicatorMeter.ts. update(level, nowMs) accepts normalized
// scalar input; snapshot(nowMs) returns { showBars, level }; reset() clears
// presentation immediately. Passing explicit clock values makes timing exact.
const { createSpeechIndicatorMeter } = await import('../dist-tests/lib/speechIndicatorMeter.js');

test('audible samples sustain bars, brief 50–250ms silence and 50ms zero packets do not reset expiry', () => {
  const meter = createSpeechIndicatorMeter();
  meter.update(0.6, 0);
  for (let ms = 50; ms <= 250; ms += 50) {
    meter.update(0, ms);
    assert.equal(meter.snapshot(ms).showBars, true, `bars persist through ${ms}ms word gap`);
  }
  meter.update(0, 299);
  assert.equal(meter.snapshot(299).showBars, true);
  assert.equal(meter.snapshot(300).showBars, false, 'silence expires 300ms after last audible sample');
  assert.equal(meter.snapshot(300).level, 0);
});

test('only fresh above-threshold voice resets silence hold; quiet events never extend it', () => {
  const meter = createSpeechIndicatorMeter();
  meter.update(0.5, 0);
  meter.update(0, 200);
  meter.update(0.4, 250);
  meter.update(0, 500);
  assert.equal(meter.snapshot(549).showBars, true);
  assert.equal(meter.snapshot(550).showBars, false);
  meter.update(0.7, 600);
  assert.equal(meter.snapshot(600).showBars, true, 'fresh speech rearms bars');
  meter.reset();
  assert.deepEqual(meter.snapshot(601), { showBars: false, level: 0 });
});

test('invalid and subthreshold levels cannot activate bars; finite levels clamp to normalized range', () => {
  const meter = createSpeechIndicatorMeter();
  for (const value of [NaN, Infinity, -Infinity, -1, 0, 0.01]) {
    meter.update(value, 0);
    assert.equal(meter.snapshot(0).showBars, false, `${value} must remain mic-only`);
  }
  meter.update(4, 10);
  assert.equal(meter.snapshot(10).showBars, true);
  assert.equal(meter.snapshot(10).level, 1);
  meter.reset();
  assert.deepEqual(meter.snapshot(10), { showBars: false, level: 0 });
});

test('exact 0.035 threshold is audible, immediately lower input is quiet and cannot extend expiry', () => {
  const meter = createSpeechIndicatorMeter();
  const below = 0.035 - Number.EPSILON;
  assert.equal(meter.update(below, 0), false);
  assert.deepEqual(meter.snapshot(0), { showBars: false, level: 0 });
  assert.equal(meter.update(0.035, 10), true);
  assert.deepEqual(meter.snapshot(10), { showBars: true, level: 0.035 });
  assert.equal(meter.update(below, 309), false);
  assert.equal(meter.snapshot(309).showBars, true);
  assert.deepEqual(meter.snapshot(310), { showBars: false, level: 0 });
});
