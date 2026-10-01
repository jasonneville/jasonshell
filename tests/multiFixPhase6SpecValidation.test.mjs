import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const masterSpec = readFileSync(new URL('../master_spec.md', import.meta.url), 'utf8');

test('master spec records durable settings and native safety boundaries without phase ledger', () => {
  assert.match(masterSpec, /settings\/tray\/audio\/calendar\/process manager/);
  assert.match(masterSpec, /File\/process\/native-picker actions need backend validation/);
  assert.doesNotMatch(masterSpec, /^## Change Ledger/m);
});

test('master spec links validation commands and active validation gaps', () => {
  assert.match(masterSpec, /Use `package\.json` as source of truth/);
  assert.match(masterSpec, /`npm run validate`/);
  assert.match(masterSpec, /Live smoke is consent-gated/);
});
