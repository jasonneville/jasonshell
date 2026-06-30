import assert from 'node:assert/strict';
import test from 'node:test';

import {
  planActivateTerminalTabWorkbench,
  planCloseTerminalTabWorkbench,
  planCreateTerminalTabWorkbench
} from '../dist-tests/features/terminal/terminalWorkbenchState.js';

const splitTree = {
  kind: 'split',
  splitId: 'split-a',
  direction: 'right',
  first: { kind: 'leaf', pane: { paneId: 'pane-tab-a', sessionId: 'tab-a', title: 'A', focused: false } },
  second: { kind: 'leaf', pane: { paneId: 'pane-b', sessionId: 'pane-b', title: 'B', focused: true } }
};

const singleTreeC = {
  kind: 'leaf',
  pane: { paneId: 'pane-tab-c', sessionId: 'tab-c', title: 'C', focused: true }
};

test('new header tab from split workbench preserves the old tab workbench and stops no existing panes', () => {
  const result = planCreateTerminalTabWorkbench({
    activeTabSessionId: 'tab-a',
    tabSessionIds: ['tab-a'],
    paneOnlySessionIds: ['pane-b'],
    workbenches: [{ tabSessionId: 'tab-a', activePaneId: 'pane-b', tree: splitTree }],
    nextSession: { sessionId: 'tab-c', title: 'C' },
    nextPaneId: 'pane-tab-c'
  });

  assert.equal(result.activeTabSessionId, 'tab-c');
  assert.equal(result.activePaneId, 'pane-tab-c');
  assert.deepEqual(result.nextTabSessionIds, ['tab-a', 'tab-c']);
  assert.deepEqual(result.nextPaneOnlySessionIds, ['pane-b']);
  assert.deepEqual(result.stopBackendSessionIds, []);
  assert.deepEqual(result.clearStoppedSessionIds, []);
  assert.deepEqual(result.visibleTree, singleTreeC);

  const oldWorkbench = result.workbenches.find((workbench) => workbench.tabSessionId === 'tab-a');
  assert.deepEqual(oldWorkbench, { tabSessionId: 'tab-a', activePaneId: 'pane-b', tree: splitTree });
  const newWorkbench = result.workbenches.find((workbench) => workbench.tabSessionId === 'tab-c');
  assert.deepEqual(newWorkbench, { tabSessionId: 'tab-c', activePaneId: 'pane-tab-c', tree: singleTreeC });
});

test('switching back to a split tab restores its exact pane tree without stopping hidden panes', () => {
  const result = planActivateTerminalTabWorkbench({
    activeTabSessionId: 'tab-c',
    targetTabSessionId: 'tab-a',
    workbenches: [
      { tabSessionId: 'tab-a', activePaneId: 'pane-b', tree: splitTree },
      { tabSessionId: 'tab-c', activePaneId: 'pane-tab-c', tree: singleTreeC }
    ]
  });

  assert.equal(result.activeTabSessionId, 'tab-a');
  assert.equal(result.activePaneId, 'pane-b');
  assert.deepEqual(result.visibleTree, splitTree);
  assert.deepEqual(result.stopBackendSessionIds, []);
  assert.deepEqual(result.clearStoppedSessionIds, []);
});

test('closing a hidden split tab stops only sessions owned by that tab', () => {
  const result = planCloseTerminalTabWorkbench({
    activeTabSessionId: 'tab-c',
    closeTabSessionId: 'tab-a',
    tabSessionIds: ['tab-a', 'tab-c'],
    paneOnlySessionIds: ['pane-b'],
    workbenches: [
      { tabSessionId: 'tab-a', activePaneId: 'pane-b', tree: splitTree },
      { tabSessionId: 'tab-c', activePaneId: 'pane-tab-c', tree: singleTreeC }
    ]
  });

  assert.equal(result.activeTabSessionId, 'tab-c');
  assert.deepEqual(result.nextTabSessionIds, ['tab-c']);
  assert.deepEqual(result.nextPaneOnlySessionIds, []);
  assert.deepEqual(result.stopBackendSessionIds.sort(), ['pane-b', 'tab-a']);
  assert.deepEqual(result.clearStoppedSessionIds.sort(), ['pane-b', 'tab-a']);
  assert.deepEqual(result.visibleTree, singleTreeC);
});

test('closing the active split tab immediately activates the neighbor tab workbench', () => {
  const result = planCloseTerminalTabWorkbench({
    activeTabSessionId: 'tab-c',
    closeTabSessionId: 'tab-c',
    tabSessionIds: ['tab-a', 'tab-c'],
    paneOnlySessionIds: ['pane-d'],
    workbenches: [
      { tabSessionId: 'tab-a', activePaneId: 'pane-tab-a', tree: { kind: 'leaf', pane: { paneId: 'pane-tab-a', sessionId: 'tab-a', title: 'A', focused: true } } },
      {
        tabSessionId: 'tab-c',
        activePaneId: 'pane-d',
        tree: {
          kind: 'split',
          splitId: 'split-c',
          direction: 'right',
          first: { kind: 'leaf', pane: { paneId: 'pane-tab-c', sessionId: 'tab-c', title: 'C', focused: false } },
          second: { kind: 'leaf', pane: { paneId: 'pane-d', sessionId: 'pane-d', title: 'D', focused: true } }
        }
      }
    ]
  });

  assert.equal(result.activeTabSessionId, 'tab-a');
  assert.equal(result.activePaneId, 'pane-tab-a');
  assert.deepEqual(result.visibleTree, { kind: 'leaf', pane: { paneId: 'pane-tab-a', sessionId: 'tab-a', title: 'A', focused: true } });
  assert.deepEqual(result.nextTabSessionIds, ['tab-a']);
  assert.deepEqual(result.nextPaneOnlySessionIds, []);
  assert.deepEqual(result.stopBackendSessionIds.sort(), ['pane-d', 'tab-c']);
});
