import assert from 'node:assert/strict';
import test from 'node:test';
import { declarations, sourceRules } from './support/cssRules.mjs';

// VIS-10. Chromium forces box-shadow to none in forced-colors mode, so any focus
// style made only of `outline: 0|none` + `box-shadow: var(--js-focus-ring)` is
// invisible there. Accepted fixes (CSS level, cheap, runs in node:test):
//   1. the base app.css :focus-visible rule paints a real `outline`, and
//   2. app.css has a `@media (forced-colors: active)` rule on `:focus-visible`
//      with a non-none outline declared `!important`, which beats every
//      component `outline: none` regardless of specificity, and
//   3. no component declares `outline: none|0 !important` on a focus selector
//      (that would beat the override).
// Real rendering is proven by tests/browser/forced-colors-focus/run.mjs.
const rules = sourceRules();
const noOutline = ({ name, value }) => (name === 'outline' && /^(?:none|0)(?:\s|$)/i.test(value))
  || (name === 'outline-style' && /^none/i.test(value))
  || (name === 'outline-width' && /^0(?:px)?(?:\s|$)/i.test(value));
const realOutline = ({ name, value }) => name === 'outline' && !/^(?:none|0)(?:\s|$)/i.test(value) && /(?:^|\s)[1-9][\d.]*px\b/.test(value);
const isForcedColors = (rule) => rule.media.some((query) => /forced-colors\s*:\s*active/.test(query));

// Rules that may hide the outline on a focus selector even under !important.
// Format: 'file :: selector' -> reason. Empty on purpose; add only programmatic
// focus containers that intentionally show no ring, with the reason.
const ALLOWED_OUTLINE_SUPPRESSION = new Map();

test('base :focus-visible rule paints a real outline (not only a box-shadow halo)', () => {
  const base = rules.filter((rule) => rule.file === 'src/app.css' && !isForcedColors(rule) && /:focus-visible/.test(rule.selector)
    && /(?:button|input|select|tabindex)/.test(rule.selector));
  assert.ok(base.length > 0, 'app.css base focus-visible rule not found');
  for (const rule of base) assert.ok(declarations(rule.body).some(realOutline), `${rule.selector} must declare outline: <n>px solid <color>`);
});

test('app.css forces a visible outline on :focus-visible in forced-colors mode', () => {
  const overrides = rules.filter((rule) => rule.file === 'src/app.css' && isForcedColors(rule) && /:focus-visible/.test(rule.selector));
  assert.ok(overrides.length > 0, 'no @media (forced-colors: active) :focus-visible override in app.css');
  const important = overrides.filter((rule) => declarations(rule.body).some((d) => d.name === 'outline' && realOutline(d) && /!important/.test(d.value)));
  assert.ok(important.length > 0, 'the override must set `outline: <n>px solid <system color> !important` so it beats component outline:none');
  const broad = important.some((rule) => /(^|,)\s*(?:\*|:where\([^)]*\)|:is\([^)]*\))?:focus-visible\s*(,|$)/.test(rule.selector)
    || /(^|,)\s*:focus-visible\s*(,|$)/.test(rule.selector));
  assert.ok(broad, 'the override selector must cover every element, i.e. a bare :focus-visible / *:focus-visible');
});

test('no component suppresses the outline with !important on a focus selector', () => {
  const offenders = rules
    .filter((rule) => /:focus/.test(rule.selector) && !isForcedColors(rule))
    .filter((rule) => declarations(rule.body).some((d) => noOutline(d) && /!important/.test(d.value)))
    .map((rule) => `${rule.file} :: ${rule.selector}`)
    .filter((key) => !ALLOWED_OUTLINE_SUPPRESSION.has(key));
  assert.deepEqual(offenders, []);
});

test('allow-list entries each name a real rule and carry a reason', () => {
  for (const [key, reason] of ALLOWED_OUTLINE_SUPPRESSION) {
    assert.ok(reason.trim().length > 10, `${key} needs a reason`);
    assert.ok(rules.some((rule) => `${rule.file} :: ${rule.selector}` === key), `${key} no longer exists`);
  }
});

test('regression: ring-only focus rules exist today, so the override is what keeps them visible', () => {
  // Documents the exposure the override protects; if all of these are migrated to a
  // real outline this list may legitimately become empty and the test can be removed.
  const ringOnly = rules.filter((rule) => /:focus-visible/.test(rule.selector) && !isForcedColors(rule)
    && declarations(rule.body).some((d) => d.name === 'box-shadow' && /--js-focus-ring/.test(d.value))
    && !declarations(rule.body).some(realOutline));
  const overridden = rules.some((rule) => rule.file === 'src/app.css' && isForcedColors(rule) && /:focus-visible/.test(rule.selector)
    && declarations(rule.body).some((d) => realOutline(d) && /!important/.test(d.value)));
  assert.ok(ringOnly.length === 0 || overridden, `${ringOnly.length} ring-only focus rules would vanish in forced-colors without the !important override`);
});
