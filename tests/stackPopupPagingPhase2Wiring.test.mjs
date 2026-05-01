import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';

const stackPopupSource = readFileSync(new URL('../src/lib/stackPopup.ts', import.meta.url), 'utf8');

test('stack folder paging uses steady chunk sizes and no 500-row follow-up burst', () => {
  const initialMatch = stackPopupSource.match(/STACK_FOLDER_INITIAL_PAGE_LIMIT = (\d+);/);
  const subsequentMatch = stackPopupSource.match(/STACK_FOLDER_SUBSEQUENT_PAGE_LIMIT = (\d+);/);

  assert.ok(initialMatch, 'initial page limit constant missing');
  assert.ok(subsequentMatch, 'subsequent page limit constant missing');
  assert.notEqual(Number(subsequentMatch[1]), 500);
  assert.ok(Number(subsequentMatch[1]) <= 200);
  assert.ok(Number(initialMatch[1]) < Number(subsequentMatch[1]));
});

test('stack folder paging forwards a listing session id between page requests', () => {
  assert.match(stackPopupSource, /sessionId\?: string;/);
  assert.match(stackPopupSource, /let sessionId: string \| undefined;/);
  assert.match(stackPopupSource, /sessionId: sessionId \?\? null/);
  assert.match(stackPopupSource, /sessionId = page\.sessionId \?\? sessionId;/);
});
