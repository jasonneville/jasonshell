export type TerminalWorkbenchPaneModel = {
  paneId: string;
  sessionId: string;
  title: string;
  focused: boolean;
};

export type TerminalWorkbenchPaneTreeNode =
  | { kind: 'leaf'; pane: TerminalWorkbenchPaneModel }
  | { kind: 'split'; splitId: string; direction: 'right' | 'down'; first: TerminalWorkbenchPaneTreeNode; second: TerminalWorkbenchPaneTreeNode };

export type TerminalWorkbenchSessionSummary = {
  sessionId: string;
  title?: string;
};

export type TerminalTabWorkbenchSummary = {
  tabSessionId: string;
  tree: TerminalWorkbenchPaneTreeNode;
  activePaneId: string;
};

export type TerminalCreateTabWorkbenchInput = {
  activeTabSessionId: string | null;
  tabSessionIds: Iterable<string>;
  paneOnlySessionIds: Iterable<string>;
  workbenches: TerminalTabWorkbenchSummary[];
  nextSession: TerminalWorkbenchSessionSummary;
  nextPaneId: string;
};

export type TerminalActivateTabWorkbenchInput = {
  activeTabSessionId: string | null;
  targetTabSessionId: string;
  workbenches: TerminalTabWorkbenchSummary[];
};

export type TerminalCloseTabWorkbenchInput = {
  activeTabSessionId: string | null;
  closeTabSessionId: string;
  tabSessionIds: Iterable<string>;
  paneOnlySessionIds: Iterable<string>;
  workbenches: TerminalTabWorkbenchSummary[];
};

export type TerminalTabWorkbenchPlan = {
  activeTabSessionId: string | null;
  activePaneId: string;
  visibleTree: TerminalWorkbenchPaneTreeNode | null;
  workbenches: TerminalTabWorkbenchSummary[];
  nextTabSessionIds: string[];
  nextPaneOnlySessionIds: string[];
  stopBackendSessionIds: string[];
  clearStoppedSessionIds: string[];
};

function unique(values: Iterable<string>) {
  return [...new Set([...values].filter(Boolean))];
}

function insertAfter(values: string[], afterValue: string | null, insertedValue: string) {
  const withoutInserted = values.filter((value) => value !== insertedValue);
  if (!afterValue) return [...withoutInserted, insertedValue];
  const index = withoutInserted.indexOf(afterValue);
  if (index < 0) return [...withoutInserted, insertedValue];
  return [...withoutInserted.slice(0, index + 1), insertedValue, ...withoutInserted.slice(index + 1)];
}

function upsertWorkbench(workbenches: TerminalTabWorkbenchSummary[], next: TerminalTabWorkbenchSummary) {
  return [...workbenches.filter((workbench) => workbench.tabSessionId !== next.tabSessionId), next];
}

export function flattenTerminalWorkbenchSessionIds(node: TerminalWorkbenchPaneTreeNode | null): string[] {
  if (!node) return [];
  if (node.kind === 'leaf') return [node.pane.sessionId];
  return unique([...flattenTerminalWorkbenchSessionIds(node.first), ...flattenTerminalWorkbenchSessionIds(node.second)]);
}

export function planCreateTerminalTabWorkbench(input: TerminalCreateTabWorkbenchInput): TerminalTabWorkbenchPlan {
  const nextSessionId = input.nextSession.sessionId;
  const tabSessionIds = insertAfter(unique(input.tabSessionIds), input.activeTabSessionId, nextSessionId);
  const paneOnlySessionIds = unique(input.paneOnlySessionIds).filter((sessionId) => sessionId !== nextSessionId);
  const visibleTree: TerminalWorkbenchPaneTreeNode = {
    kind: 'leaf',
    pane: {
      paneId: input.nextPaneId,
      sessionId: nextSessionId,
      title: input.nextSession.title || 'Terminal',
      focused: true
    }
  };
  const nextWorkbench: TerminalTabWorkbenchSummary = {
    tabSessionId: nextSessionId,
    activePaneId: input.nextPaneId,
    tree: visibleTree
  };
  const workbenches = upsertWorkbench(input.workbenches, nextWorkbench);

  return {
    activeTabSessionId: nextSessionId,
    activePaneId: input.nextPaneId,
    visibleTree,
    workbenches,
    nextTabSessionIds: tabSessionIds,
    nextPaneOnlySessionIds: paneOnlySessionIds,
    stopBackendSessionIds: [],
    clearStoppedSessionIds: []
  };
}

export function planActivateTerminalTabWorkbench(input: TerminalActivateTabWorkbenchInput): TerminalTabWorkbenchPlan {
  const targetWorkbench = input.workbenches.find((workbench) => workbench.tabSessionId === input.targetTabSessionId) ?? null;
  if (!targetWorkbench) {
    return {
      activeTabSessionId: input.activeTabSessionId,
      activePaneId: '',
      visibleTree: null,
      workbenches: input.workbenches,
      nextTabSessionIds: [],
      nextPaneOnlySessionIds: [],
      stopBackendSessionIds: [],
      clearStoppedSessionIds: []
    };
  }
  return {
    activeTabSessionId: targetWorkbench.tabSessionId,
    activePaneId: targetWorkbench.activePaneId,
    visibleTree: targetWorkbench.tree,
    workbenches: input.workbenches,
    nextTabSessionIds: [],
    nextPaneOnlySessionIds: [],
    stopBackendSessionIds: [],
    clearStoppedSessionIds: []
  };
}

export function planCloseTerminalTabWorkbench(input: TerminalCloseTabWorkbenchInput): TerminalTabWorkbenchPlan {
  const tabSessionIds = unique(input.tabSessionIds);
  const paneOnlySessionIds = new Set(unique(input.paneOnlySessionIds));
  const closeWorkbench = input.workbenches.find((workbench) => workbench.tabSessionId === input.closeTabSessionId) ?? null;
  const closingSessionIds = flattenTerminalWorkbenchSessionIds(closeWorkbench?.tree ?? null);
  const nextTabSessionIds = tabSessionIds.filter((sessionId) => sessionId !== input.closeTabSessionId && !closingSessionIds.includes(sessionId));
  for (const sessionId of closingSessionIds) paneOnlySessionIds.delete(sessionId);
  paneOnlySessionIds.delete(input.closeTabSessionId);
  const nextWorkbenches = input.workbenches.filter((workbench) => workbench.tabSessionId !== input.closeTabSessionId);

  let activeTabSessionId = input.activeTabSessionId;
  if (activeTabSessionId === input.closeTabSessionId || (activeTabSessionId && closingSessionIds.includes(activeTabSessionId))) {
    const closeIndex = tabSessionIds.indexOf(input.closeTabSessionId);
    activeTabSessionId = nextTabSessionIds[Math.min(closeIndex, nextTabSessionIds.length - 1)] ?? nextTabSessionIds[closeIndex - 1] ?? nextTabSessionIds[0] ?? null;
  }
  const activeWorkbench = activeTabSessionId ? nextWorkbenches.find((workbench) => workbench.tabSessionId === activeTabSessionId) ?? null : null;

  return {
    activeTabSessionId,
    activePaneId: activeWorkbench?.activePaneId ?? '',
    visibleTree: activeWorkbench?.tree ?? null,
    workbenches: nextWorkbenches,
    nextTabSessionIds,
    nextPaneOnlySessionIds: unique(paneOnlySessionIds),
    stopBackendSessionIds: unique(closingSessionIds),
    clearStoppedSessionIds: unique(closingSessionIds)
  };
}
