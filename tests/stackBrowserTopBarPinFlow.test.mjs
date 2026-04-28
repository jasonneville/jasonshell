import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';

const stackPopupSource = readFileSync(new URL('../src/lib/stackPopup.ts', import.meta.url), 'utf8');

const alpha = { id: 'C:\\Alpha', name: 'Alpha', path: 'C:\\Alpha' };
const beta = { id: 'C:\\Beta', name: 'Beta', path: 'C:\\Beta' };

function extractFunctionBody(source, functionName) {
  const start = source.indexOf(`export async function ${functionName}`);
  assert.notEqual(start, -1, `${functionName} must stay exported for Stack Browser pin actions`);

  const openBrace = source.indexOf('{', start);
  assert.notEqual(openBrace, -1, `${functionName} must have a function body`);

  let depth = 0;
  for (let index = openBrace; index < source.length; index += 1) {
    const char = source[index];
    if (char === '{') {
      depth += 1;
    } else if (char === '}') {
      depth -= 1;
      if (depth === 0) {
        return source.slice(openBrace + 1, index);
      }
    }
  }

  assert.fail(`${functionName} body did not close`);
}

function currentPinMutationEventPayload({ mutationReturnedPins, postMutationListPins }) {
  const pinStackFolderBody = extractFunctionBody(stackPopupSource, 'pinStackFolder');

  if (/const\s+pins\s*=\s*await\s+listStackPins\(\)/.test(pinStackFolderBody)) {
    return postMutationListPins;
  }

  if (/(pin_stack_folder|IPC_COMMANDS\.pinStackFolder)/.test(pinStackFolderBody) && /emitStackPinsUpdated\(pins\)/.test(pinStackFolderBody)) {
    return mutationReturnedPins;
  }

  return [];
}

function applyTopBarPinsUpdate(currentPins, eventPayload) {
  return Array.isArray(eventPayload) ? eventPayload : currentPins;
}

test('Stack Browser pin mutation updates the top-bar display immediately without startup reload', () => {
  const topBarBeforePin = [alpha];
  const mutationReturnedPins = [alpha, beta];

  // This models the bug report: the backend persisted Beta, so the next startup
  // list contains it, but the immediate mutation-side list/event path is stale.
  const staleImmediateListPins = [alpha];
  const startupReloadPins = [alpha, beta];

  const immediateEventPayload = currentPinMutationEventPayload({
    mutationReturnedPins,
    postMutationListPins: staleImmediateListPins
  });
  const topBarAfterPin = applyTopBarPinsUpdate(topBarBeforePin, immediateEventPayload);

  assert.deepEqual(
    topBarAfterPin.map((pin) => pin.path),
    startupReloadPins.map((pin) => pin.path),
    'Stack Browser Pin must publish the complete next top-bar pin model immediately; helper-only target-shape tests do not catch a stale listStackPins() publication that only recovers after app reload.'
  );
});
