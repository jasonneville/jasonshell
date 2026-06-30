<script lang="ts">
  import './TerminalPanelSurface.css';
  import '@xterm/xterm/css/xterm.css';
  import { emitTo, listen, type UnlistenFn } from '@tauri-apps/api/event';
  import { FitAddon } from '@xterm/addon-fit';
  import { SearchAddon } from '@xterm/addon-search';
  import { Terminal } from '@xterm/xterm';
  import { onMount, tick } from 'svelte';
  import {
    listStackTerminals,
    readStackTerminal,
    renameStackTerminal,
    resizeStackTerminal,
    startPersistentTerminal,
    startStackTerminal,
    stopStackTerminal,
    stopTerminalPanelSessions,
    writeStackTerminal,
    type StackTerminalOutputChunk,
    type StackTerminalSession
  } from '../lib/persistentTerminal';
  import { topBarWebviewWindowEventTarget } from '../lib/topBarPins';
  import { hideTerminalPanel } from '../lib/terminalPanel';
  import { positionScrollableContextMenuInViewport } from '../lib/contextMenuPosition';
  import { openStackFolderInVscode, openStackItem, openStackTerminalHere, revealStackItem } from '../lib/stackPopup';
  import {
    beginTerminalCommandRecord,
    createTerminalCommandState,
    parseTerminalShellSequence,
    reduceTerminalShellMarker,
    type TerminalCommandRecord,
    type TerminalCommandState
  } from '../features/stack-browser/terminalShellIntegration';
  import { getTerminalAction, terminalActions, type TerminalActionId, type TerminalActionState } from '../features/terminal/terminalActions';
  import { detectTerminalQuickSelectTargets, type TerminalQuickSelectTarget } from '../features/terminal/terminalQuickSelect';
  import { recentTerminalCommands, recentTerminalDirectories } from '../features/terminal/terminalHistory';
  import { buildTerminalTabTitle } from '../features/terminal/terminalTabTitle';
  import {
    flattenTerminalWorkbenchSessionIds,
    planActivateTerminalTabWorkbench,
    planCloseTerminalTabWorkbench,
    planCreateTerminalTabWorkbench,
    type TerminalTabWorkbenchSummary
  } from '../features/terminal/terminalWorkbenchState';
  import { shouldAnimateTerminalCommand } from '../features/top-bar/topBarUxState';

  type TerminalLifecycleState = 'not-started' | 'scheduled' | 'starting' | 'waiting' | 'running' | 'failed' | 'exited';
  type TerminalStartupIntent = 'idle-prewarm' | 'first-open' | 'user-action';
  type TerminalSplitDirection = 'right' | 'down';
  type TerminalSplitOrientation = 'single' | 'vertical' | 'horizontal' | 'mixed';
  type TerminalPaneModel = { paneId: string; sessionId: string; title: string; focused: boolean };
  type TerminalPaneTreeNode =
    | { kind: 'leaf'; pane: TerminalPaneModel }
    | { kind: 'split'; splitId: string; direction: TerminalSplitDirection; first: TerminalPaneTreeNode; second: TerminalPaneTreeNode };
  type TerminalPaneRuntime = {
    paneId: string;
    runtimeId: string;
    disposed: boolean;
    host: HTMLDivElement | null;
    terminal: Terminal | null;
    fitAddon: FitAddon | null;
    searchAddon: SearchAddon | null;
    resizeObserver: ResizeObserver | null;
    resizeFrame: number | null;
    session: StackTerminalSession;
    lifecycle: TerminalLifecycleState;
    status: string;
    outputReceived: boolean;
    startupTimer: number | null;
    pollTimer: number | null;
    pollInFlight: boolean;
    pollQueued: boolean;
    operationQueue: Promise<void>;
    writeQueue: Promise<void>;
    renderedSequences: Set<string>;
    lastResizeKey: string;
    visibleResizeSettled: boolean;
    visibleResizePromise: Promise<void> | null;
    currentInputText: string;
    currentInputSelectionActive: boolean;
    commandState: TerminalCommandState | null;
    selectedCommandIndex: number;
    shellCwdMarkerSeen: boolean;
    shellParserDisposers: Array<() => void>;
    recentOutputText: string;
    replayedSessionOutput: boolean;
  };
  type TerminalOutputPayload = StackTerminalOutputChunk;
  type TerminalClosedPayload = {
    sessionId: string;
    running?: boolean;
  };
  type TerminalCwdPayload = {
    sessionId: string;
    cwd: string;
  };
  type TerminalTitleState = {
    cwd?: string;
    currentInputText?: string;
    recentOutputText?: string;
    commandState?: TerminalCommandState | null;
  };

  const TERMINAL_PANEL_OPEN_EVENT = 'terminal-panel:open';
  const TOP_BAR_TERMINAL_ACTIVITY_EVENT = 'terminal-panel:activity';
  const TOP_BAR_EVENT_TARGET = topBarWebviewWindowEventTarget();
  const importantTerminalActivitySessions = new Set<string>();
  const manuallyRenamedTerminalSessions = new Set<string>();

  const TERMINAL_PANEL_FONT_FAMILY = '"Cascadia Mono", "Cascadia Code", Consolas, ui-monospace, "SFMono-Regular", monospace';
  const TERMINAL_PANEL_DEFAULT_FONT_SIZE = 13;
  const TERMINAL_PANEL_MIN_FONT_SIZE = 9;
  const TERMINAL_PANEL_MAX_FONT_SIZE = 28;
  const TERMINAL_IDLE_PREWARM_DELAY_MS = 5_000;

  let host: HTMLDivElement | null = null;
  let terminal: Terminal | null = null;
  let fitAddon: FitAddon | null = null;
  let searchAddon: SearchAddon | null = null;
  let resizeObserver: ResizeObserver | null = null;
  let resizeFrame: number | null = null;
  let session: StackTerminalSession | null = null;
  let paneRuntimes = new Map<string, TerminalPaneRuntime>();
  let terminalSessions: StackTerminalSession[] = [];
  let terminalTabSessionIds = new Set<string>();
  let paneOnlyTerminalSessionIds = new Set<string>();
  let terminalTabWorkbenches = new Map<string, TerminalTabWorkbenchSummary>();
  let visibleTerminalTabs: StackTerminalSession[] = [];
  let terminalPaneTree: TerminalPaneTreeNode | null = null;
  let terminalPanes: TerminalPaneModel[] = [];
  let terminalWorkbenchGeneration = 0;
  let terminalRuntimeCounter = 0;
  let activeTerminalTabSessionId: string | null = null;
  let terminalFontSize = TERMINAL_PANEL_DEFAULT_FONT_SIZE;
  let activePaneId = 'terminal-pane-primary';
  let splitOrientation: TerminalSplitOrientation = 'single';
  let status = 'Terminal will start when opened.';
  let lifecycle: TerminalLifecycleState = 'not-started';
  let outputReceived = false;
  let startupTimer: number | null = null;
  let pollTimer: number | null = null;
  let pollInFlight = false;
  let pollQueued = false;
  let operationQueue: Promise<void> = Promise.resolve();
  let writeQueue: Promise<void> = Promise.resolve();
  let renderedSequences = new Set<string>();
  let lastResizeKey = '';
  let unlisteners: Array<() => void> = [];
  let listenersDisposed = false;
  let contextMenu: { x: number; y: number } | null = null;
  let actionMenuOpen = false;
  let recentMenuOpen = false;
  let searchOpen = false;
  let searchQuery = '';
  let quickSelectOpen = false;
  let quickSelectTargets: TerminalQuickSelectTarget[] = [];
  let recentOutputText = '';
  let sessionReplayBuffers = new Map<string, string>();
  let terminalTitleStates = new Map<string, TerminalTitleState>();
  let renderedSequenceKeysBySession = new Map<string, Set<string>>();
  let idlePrewarmTimer: number | null = null;
  let terminalStartPromise: Promise<void> | null = null;
  let terminalSessionCreationInFlight = false;

  let currentInputText = '';
  let currentInputSelectionActive = false;
  let visibleResizeSettled = false;
  let visibleResizePromise: Promise<void> | null = null;
  let commandState: TerminalCommandState | null = null;
  let selectedCommandIndex = -1;
  let shellCwdMarkerSeen = false;
  let shellParserDisposers: Array<() => void> = [];

  $: terminalActionState = buildTerminalActionState(
    session,
    terminal,
    terminalPanes,
    terminalSessions,
    activePaneId,
    paneRuntimes,
    quickSelectTargets,
    commandState,
    selectedCommandIndex,
    terminalSessionCreationInFlight
  );
  $: visibleTerminalTabs = terminalSessions.filter((item) => terminalTabSessionIds.has(item.sessionId) && !paneOnlyTerminalSessionIds.has(item.sessionId));
  $: visibleRecentCommands = recentTerminalCommands(activeRuntime()?.commandState ?? commandState);
  $: visibleRecentDirectories = recentTerminalDirectories(activeRuntime()?.commandState ?? commandState);
  $: toolbarActions = terminalActions.filter((action) => ['search', 'newSession', 'renameSession', 'splitHorizontal', 'splitVertical', 'openCwdInFiles', 'openExternalTerminalHere', 'restartTerminal', 'stopTerminal'].includes(action.id));

  function nextTerminalPaneId() {
    if (!terminalPanes.length && !terminalPaneTree) return 'terminal-pane-primary';
    return `terminal-pane-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 7)}`;
  }

  function nextTerminalSplitId() {
    return `terminal-split-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 7)}`;
  }

  function nextTerminalRuntimeId() {
    terminalRuntimeCounter += 1;
    return `terminal-runtime-${terminalRuntimeCounter.toString(36)}`;
  }

  function paneDomKey(pane: TerminalPaneModel) {
    return `${pane.paneId}:${pane.sessionId}`;
  }

  function flattenPaneTree(node: TerminalPaneTreeNode | null = terminalPaneTree): TerminalPaneModel[] {
    if (!node) return [];
    if (node.kind === 'leaf') return [node.pane];
    return [...flattenPaneTree(node.first), ...flattenPaneTree(node.second)];
  }

  function syncTerminalPaneList() {
    terminalPanes = flattenPaneTree(terminalPaneTree);
    splitOrientation = terminalPanes.length > 1 ? 'mixed' : 'single';
  }

  function terminalTabWorkbenchesList() {
    return [...terminalTabWorkbenches.values()];
  }

  function replaceTerminalTabWorkbenches(workbenches: TerminalTabWorkbenchSummary[]) {
    terminalTabWorkbenches = new Map(workbenches.map((workbench) => [workbench.tabSessionId, workbench]));
  }

  function saveCurrentTerminalWorkbench(tabSessionId = activeTerminalTabSessionId) {
    if (!tabSessionId || !terminalPaneTree) return;
    terminalTabWorkbenches = new Map(terminalTabWorkbenches).set(tabSessionId, {
      tabSessionId,
      tree: terminalPaneTree,
      activePaneId
    });
  }

  function setTerminalPaneTree(nextTree: TerminalPaneTreeNode | null, bumpWorkbenchGeneration = true, saveWorkbench = true) {
    terminalPaneTree = nextTree;
    syncTerminalPaneList();
    if (bumpWorkbenchGeneration) terminalWorkbenchGeneration += 1;
    if (saveWorkbench) saveCurrentTerminalWorkbench();
  }

  function markTerminalSessionAsTab(sessionId: string) {
    paneOnlyTerminalSessionIds.delete(sessionId);
    terminalTabSessionIds.add(sessionId);
    paneOnlyTerminalSessionIds = new Set(paneOnlyTerminalSessionIds);
    terminalTabSessionIds = new Set(terminalTabSessionIds);
  }

  function markTerminalSessionAsPaneOnly(sessionId: string) {
    terminalTabSessionIds.delete(sessionId);
    paneOnlyTerminalSessionIds.add(sessionId);
    terminalTabSessionIds = new Set(terminalTabSessionIds);
    paneOnlyTerminalSessionIds = new Set(paneOnlyTerminalSessionIds);
  }

  function forgetTerminalSessionOwnership(sessionId: string) {
    terminalTabSessionIds.delete(sessionId);
    paneOnlyTerminalSessionIds.delete(sessionId);
    terminalTabSessionIds = new Set(terminalTabSessionIds);
    paneOnlyTerminalSessionIds = new Set(paneOnlyTerminalSessionIds);
  }

  function clearStoppedTerminalSessionState(sessionId: string) {
    forgetTerminalSessionOwnership(sessionId);
    sessionReplayBuffers.delete(sessionId);
    renderedSequenceKeysBySession.delete(sessionId);
    terminalTitleStates.delete(sessionId);
    importantTerminalActivitySessions.delete(sessionId);
    manuallyRenamedTerminalSessions.delete(sessionId);
    sessionReplayBuffers = new Map(sessionReplayBuffers);
    renderedSequenceKeysBySession = new Map(renderedSequenceKeysBySession);
    terminalTitleStates = new Map(terminalTitleStates);
  }

  function currentVisibleTerminalTabs() {
    return terminalSessions.filter((item) => terminalTabSessionIds.has(item.sessionId) && !paneOnlyTerminalSessionIds.has(item.sessionId));
  }

  function orderTerminalSessionsByTabIds(tabSessionIds: string[]) {
    const order = new Map(tabSessionIds.map((sessionId, index) => [sessionId, index]));
    terminalSessions = [...terminalSessions].sort((left, right) => {
      const leftOrder = order.get(left.sessionId);
      const rightOrder = order.get(right.sessionId);
      if (leftOrder !== undefined && rightOrder !== undefined) return leftOrder - rightOrder;
      if (leftOrder !== undefined) return -1;
      if (rightOrder !== undefined) return 1;
      return 0;
    });
  }

  function mapTerminalPaneTree(
    node: TerminalPaneTreeNode | null,
    mapper: (pane: TerminalPaneModel) => TerminalPaneModel
  ): TerminalPaneTreeNode | null {
    if (!node) return null;
    if (node.kind === 'leaf') return { ...node, pane: mapper(node.pane) };
    return {
      ...node,
      first: mapTerminalPaneTree(node.first, mapper) ?? node.first,
      second: mapTerminalPaneTree(node.second, mapper) ?? node.second
    };
  }

  function removePaneFromTree(node: TerminalPaneTreeNode | null, paneId: string): TerminalPaneTreeNode | null {
    if (!node) return null;
    if (node.kind === 'leaf') return node.pane.paneId === paneId ? null : node;
    const first = removePaneFromTree(node.first, paneId);
    const second = removePaneFromTree(node.second, paneId);
    if (!first) return second;
    if (!second) return first;
    if (first === node.first && second === node.second) return node;
    return { ...node, first, second };
  }

  function splitPaneTreeAtLeaf(
    node: TerminalPaneTreeNode | null,
    targetPaneId: string,
    nextPane: TerminalPaneModel,
    direction: TerminalSplitDirection
  ): TerminalPaneTreeNode {
    if (!node) return { kind: 'leaf', pane: nextPane };
    if (node.kind === 'leaf') {
      if (node.pane.paneId !== targetPaneId) return node;
      return {
        kind: 'split',
        splitId: nextTerminalSplitId(),
        direction,
        first: { kind: 'leaf', pane: { ...node.pane, focused: false } },
        second: { kind: 'leaf', pane: nextPane }
      };
    }
    return {
      ...node,
      first: splitPaneTreeAtLeaf(node.first, targetPaneId, nextPane, direction),
      second: splitPaneTreeAtLeaf(node.second, targetPaneId, nextPane, direction)
    };
  }

  function replacePaneSessionInTree(
    node: TerminalPaneTreeNode | null,
    paneId: string,
    nextSession: StackTerminalSession,
    title: string
  ): TerminalPaneTreeNode | null {
    return mapTerminalPaneTree(node, (pane) => pane.paneId === paneId ? { ...pane, sessionId: nextSession.sessionId, title } : pane);
  }

  function currentWorkbenchTabSessionId() {
    if (activeTerminalTabSessionId && terminalTabSessionIds.has(activeTerminalTabSessionId) && !paneOnlyTerminalSessionIds.has(activeTerminalTabSessionId)) {
      return activeTerminalTabSessionId;
    }
    if (session && terminalTabSessionIds.has(session.sessionId) && !paneOnlyTerminalSessionIds.has(session.sessionId)) return session.sessionId;
    return currentVisibleTerminalTabs().find((item) => item.running)?.sessionId ?? currentVisibleTerminalTabs()[0]?.sessionId ?? null;
  }

  function runtimeMatchesCurrentPane(runtime: TerminalPaneRuntime) {
    return terminalPanes.some((pane) => pane.paneId === runtime.paneId && pane.sessionId === runtime.session.sessionId);
  }

  function isRuntimeCurrent(runtime: TerminalPaneRuntime) {
    if (runtime.disposed || !runtimeMatchesCurrentPane(runtime)) return false;
    const currentRuntime = paneRuntimes.get(runtime.paneId);
    return !currentRuntime || currentRuntime.runtimeId === runtime.runtimeId || currentRuntime.disposed;
  }

  function activeRuntime() {
    const runtime = paneRuntimes.get(activePaneId) ?? null;
    return runtime && isRuntimeCurrent(runtime) ? runtime : null;
  }

  function runtimeForSession(sessionId: string) {
    for (const runtime of paneRuntimes.values()) {
      if (runtime.session.sessionId === sessionId && isRuntimeCurrent(runtime)) {
        return runtime;
      }
    }
    return null;
  }

  function runtimeForPane(pane: TerminalPaneModel) {
    const runtime = paneRuntimes.get(pane.paneId);
    return runtime && runtime.session.sessionId === pane.sessionId && isRuntimeCurrent(runtime) ? runtime : null;
  }

  function setActiveRuntime(runtime: TerminalPaneRuntime | null) {
    const currentRuntime = runtime && isRuntimeCurrent(runtime) ? runtime : null;
    session = currentRuntime?.session ?? null;
    terminal = currentRuntime?.terminal ?? null;
    fitAddon = currentRuntime?.fitAddon ?? null;
    searchAddon = currentRuntime?.searchAddon ?? null;
    host = currentRuntime?.host ?? null;
    lifecycle = currentRuntime?.lifecycle ?? 'starting';
    status = currentRuntime?.status ?? 'Starting terminal...';
    outputReceived = Boolean(currentRuntime?.outputReceived);
    commandState = currentRuntime?.commandState ?? null;
    selectedCommandIndex = currentRuntime?.selectedCommandIndex ?? -1;
    shellCwdMarkerSeen = Boolean(currentRuntime?.shellCwdMarkerSeen);
    currentInputText = currentRuntime?.currentInputText ?? '';
    currentInputSelectionActive = Boolean(currentRuntime?.currentInputSelectionActive);
    recentOutputText = currentRuntime?.recentOutputText ?? '';
  }

  function commitRuntime(runtime: TerminalPaneRuntime) {
    if (!isRuntimeCurrent(runtime)) return false;
    rememberTerminalTitleStateForRuntime(runtime);
    paneRuntimes = new Map(paneRuntimes).set(runtime.paneId, runtime);
    if (runtime.paneId === activePaneId) {
      setActiveRuntime(runtime);
    }
    return true;
  }

  function markPaneRuntimeDisposed(runtime: TerminalPaneRuntime) {
    if (runtime.disposed) return;
    rememberTerminalTitleStateForRuntime(runtime);
    runtime.disposed = true;
    runtime.host = null;
    runtime.visibleResizeSettled = false;
    runtime.visibleResizePromise = null;
    runtime.pollQueued = false;
  }

  function rememberTerminalTitleStateForRuntime(runtime: TerminalPaneRuntime) {
    terminalTitleStates = new Map(terminalTitleStates).set(runtime.session.sessionId, {
      cwd: runtime.session.cwd,
      currentInputText: runtime.currentInputText,
      recentOutputText: runtime.recentOutputText,
      commandState: runtime.commandState
    });
  }

  function rememberTerminalTitleOutput(sessionId: string, output: string) {
    if (!output) return;
    const previous = terminalTitleStates.get(sessionId) ?? {};
    terminalTitleStates = new Map(terminalTitleStates).set(sessionId, {
      ...previous,
      recentOutputText: `${previous.recentOutputText ?? ''}${stripTerminalAnsiControls(output)}`.slice(-20000)
    });
  }

  onMount(() => {
    listenersDisposed = false;
    document.addEventListener('pointerdown', closeTerminalMenusOnOutsidePointer, true);
    window.addEventListener('focus', handlePanelOpen);
    void initializeTerminalListeners();
    scheduleIdlePrewarm();
    return () => {
      listenersDisposed = true;
      document.removeEventListener('pointerdown', closeTerminalMenusOnOutsidePointer, true);
      window.removeEventListener('focus', handlePanelOpen);
      cancelIdlePrewarm();
      terminalStartPromise = null;
      for (const runtime of paneRuntimes.values()) {
        markPaneRuntimeDisposed(runtime);
        stopPollingForRuntime(runtime);
        clearStartupTimerForRuntime(runtime);
        disposePaneRuntime(runtime);
      }
      paneRuntimes = new Map();
      stopPolling();
      clearStartupTimer();
      disposeTerminalView();
      for (const unlisten of unlisteners.splice(0)) {
        unlisten();
      }
    };
  });

  async function initializeTerminalListeners() {
    const register = (promise: Promise<UnlistenFn>) => {
      promise
        .then((unlisten) => {
          if (listenersDisposed) {
            unlisten();
            return;
          }
          unlisteners.push(unlisten);
        })
        .catch((error) => {
          console.error('Failed to initialize persistent terminal listener', error);
        });
    };

    register(
      listen(TERMINAL_PANEL_OPEN_EVENT, () => {
        void handlePanelOpen();
      })
    );

    register(
      listen<TerminalOutputPayload>('stack-terminal:output', (event) => {
        notifyTopBarForImportantTerminalOutput(event.payload.sessionId);
        const runtime = runtimeForSession(event.payload.sessionId);
        if (!runtime) {
          rememberTerminalChunkForSession(event.payload);
          rememberTerminalTitleOutput(event.payload.sessionId, event.payload.text);
          return;
        }
        writeTerminalChunkForRuntime(runtime, event.payload);
      })
    );

    register(
      listen<TerminalClosedPayload>('stack-terminal:closed', (event) => {
        clearImportantTerminalActivity(event.payload.sessionId);
        const runtime = runtimeForSession(event.payload.sessionId);
        terminalSessions = terminalSessions.map((item) => item.sessionId === event.payload.sessionId ? { ...item, running: false } : item);
        if (!runtime) {
          void refreshTerminalSessionList();
          return;
        }
        stopPollingForRuntime(runtime);
        clearStartupTimerForRuntime(runtime);
        runtime.lifecycle = runtime.outputReceived ? 'exited' : 'failed';
        runtime.status = runtime.outputReceived ? 'Terminal exited' : 'Terminal exited before output';
        runtime.session = { ...runtime.session, running: false };
        commitRuntime(runtime);
      })
    );

    register(
      listen<TerminalCwdPayload>('stack-terminal:cwd', (event) => {
        const runtime = runtimeForSession(event.payload.sessionId);
        if (!runtime || !event.payload.cwd) return;
        if (!runtime.shellCwdMarkerSeen) applyAuthoritativeTerminalCwdForRuntime(runtime, event.payload.cwd);
      })
    );
  }

  function scheduleIdlePrewarm() {
    if (listenersDisposed || idlePrewarmTimer !== null || terminalStartPromise || session || terminalSessions.length) return;
    lifecycle = 'scheduled';
    status = 'Terminal idle prewarm scheduled.';
    idlePrewarmTimer = window.setTimeout(() => {
      idlePrewarmTimer = null;
      if (listenersDisposed || session || terminalStartPromise) return;
      void startTerminal('idle-prewarm');
    }, TERMINAL_IDLE_PREWARM_DELAY_MS);
  }

  function cancelIdlePrewarm() {
    if (idlePrewarmTimer !== null) {
      window.clearTimeout(idlePrewarmTimer);
      idlePrewarmTimer = null;
    }
  }

  async function startTerminal(intent: TerminalStartupIntent = 'user-action') {
    cancelIdlePrewarm();
    if (terminalStartPromise) {
      await terminalStartPromise;
      if (intent !== 'idle-prewarm') handlePanelOpen();
      return;
    }
    terminalStartPromise = startTerminalOnce(intent).finally(() => {
      terminalStartPromise = null;
    });
    await terminalStartPromise;
  }

  async function startTerminalOnce(intent: TerminalStartupIntent) {
    status = intent === 'idle-prewarm' ? 'Prewarming terminal in the background...' : 'Starting terminal...';
    lifecycle = 'starting';
    outputReceived = false;
    renderedSequences = new Set<string>();
    if (intent !== 'idle-prewarm') ensureTerminalView();
    startStartupTimer();
    try {
      await refreshTerminalSessionList();
      const tabs = currentVisibleTerminalTabs();
      session = tabs.find((candidate) => candidate.running) ?? tabs[0] ?? await startPersistentTerminal();
      markTerminalSessionAsTab(session.sessionId);
      activeTerminalTabSessionId = session.sessionId;
      await refreshTerminalSessionList();
      const runtime = ensurePrimaryPaneForSession(session);
      if (intent !== 'idle-prewarm') ensureTerminalViewForPane(runtime);
      startStartupTimerForRuntime(runtime);
      commandState = runtime.commandState;
      selectedCommandIndex = -1;
      shellCwdMarkerSeen = false;
      lifecycle = 'waiting';
      status = 'Waiting for terminal output...';
      if (intent !== 'idle-prewarm') handlePanelOpen();
      startPollingForRuntime(runtime);
      void pollTerminalOutputForRuntime(runtime);
      if (intent !== 'idle-prewarm') focusTerminal();
    } catch (error) {
      clearStartupTimer();
      lifecycle = 'failed';
      status = errorMessage(error, 'Terminal failed to start');
      console.error('Failed to start persistent terminal', error);
    }
  }

  async function refreshTerminalSessionList() {
    const backendSessions = await listStackTerminals('terminal-panel').catch(() => session ? [session] : []);
    const backendSessionIds = new Set(backendSessions.map((item) => item.sessionId));
    const knownOwnedSessionIds = new Set([...terminalTabSessionIds, ...paneOnlyTerminalSessionIds]);
    const nextTabSessionIds = new Set(terminalTabSessionIds);
    for (const backendSession of backendSessions) {
      if (!knownOwnedSessionIds.has(backendSession.sessionId) && !paneOnlyTerminalSessionIds.has(backendSession.sessionId)) {
        nextTabSessionIds.add(backendSession.sessionId);
      }
    }
    terminalTabSessionIds = new Set([...nextTabSessionIds].filter((sessionId) => backendSessionIds.has(sessionId)));
    paneOnlyTerminalSessionIds = new Set([...paneOnlyTerminalSessionIds].filter((sessionId) => backendSessionIds.has(sessionId)));
    terminalTabWorkbenches = new Map([...terminalTabWorkbenches.entries()].filter(([tabSessionId, workbench]) => {
      if (!backendSessionIds.has(tabSessionId)) return false;
      return flattenTerminalWorkbenchSessionIds(workbench.tree).every((sessionId) => backendSessionIds.has(sessionId));
    }));
    if (activeTerminalTabSessionId && !terminalTabSessionIds.has(activeTerminalTabSessionId)) {
      activeTerminalTabSessionId = null;
    }
    terminalSessions = backendSessions;
    for (const runtime of [...paneRuntimes.values()]) {
      if (!isRuntimeCurrent(runtime)) continue;
      const backendSession = backendSessions.find((item) => item.sessionId === runtime.session.sessionId);
      if (backendSession) {
        runtime.session = backendSession;
        commitRuntime(runtime);
      }
    }
    const orphanPanes = terminalPanes.filter((pane) => !backendSessionIds.has(pane.sessionId));
    for (const pane of orphanPanes) {
      removePaneRuntime(pane.paneId, false, true);
    }
    if (session && backendSessionIds.has(session.sessionId)) {
      session = backendSessions.find((item) => item.sessionId === session?.sessionId) ?? session;
    } else if (!session && currentVisibleTerminalTabs().length) {
      session = currentVisibleTerminalTabs()[0];
    }
  }

  function ensurePrimaryPaneForSession(nextSession: StackTerminalSession): TerminalPaneRuntime {
    if (!activeTerminalTabSessionId) activeTerminalTabSessionId = nextSession.sessionId;
    const existing = terminalPanes.find((pane) => pane.sessionId === nextSession.sessionId);
    if (existing) {
      activatePane(existing.paneId);
      const existingRuntime = paneRuntimes.get(existing.paneId);
      if (existingRuntime && isRuntimeCurrent(existingRuntime)) return existingRuntime;
    }
    const pane: TerminalPaneModel = {
      paneId: terminalPaneTree ? nextTerminalPaneId() : 'terminal-pane-primary',
      sessionId: nextSession.sessionId,
      title: nextSession.title || `Session ${terminalPanes.length + 1}`,
      focused: true
    };
    const previousActivePaneId = activePaneId;
    activePaneId = pane.paneId;
    if (!terminalPaneTree) {
      setTerminalPaneTree({ kind: 'leaf', pane });
    } else {
      const targetPaneId = terminalPanes.some((item) => item.paneId === previousActivePaneId)
        ? previousActivePaneId
        : terminalPanes[0]?.paneId;
      setTerminalPaneTree(splitPaneTreeAtLeaf(terminalPaneTree, targetPaneId ?? pane.paneId, pane, 'right'));
    }
    const runtime = createPaneRuntime(nextSession, pane.paneId);
    commitRuntime(runtime);
    saveCurrentTerminalWorkbench();
    setActiveRuntime(runtime);
    return runtime;
  }

  function activatePane(paneId: string) {
    activePaneId = paneId;
    setTerminalPaneTree(mapTerminalPaneTree(terminalPaneTree, (pane) => ({ ...pane, focused: pane.paneId === activePaneId })), false);
    setActiveRuntime(activeRuntime());
  }

  function activateTerminalSession(nextSession: StackTerminalSession) {
    activateTerminalTabWorkbench(nextSession);
  }

  function terminalPanelStartupCwd() {
    const runtime = activeRuntime();
    return runtime?.session.cwd || session?.cwd || commandState?.cwd || selectedCommand()?.cwd || '';
  }

  function terminalPanelStartupProfile() {
    return activeRuntime()?.session.profile ?? session?.profile ?? 'windowsTerminal';
  }

  async function startTerminalPanelSessionInActiveCwd() {
    const cwd = terminalPanelStartupCwd();
    if (!cwd) return startPersistentTerminal();
    return startStackTerminal(cwd, terminalPanelStartupProfile(), 'terminal-panel');
  }

  function isSplitStartStale(
    splitStartGeneration: number,
    splitStartTabSessionId: string | null,
    targetPaneId: string,
    targetSessionId: string
  ) {
    if (listenersDisposed) return true;
    if (splitStartGeneration !== terminalWorkbenchGeneration) return true;
    if (currentWorkbenchTabSessionId() !== splitStartTabSessionId) return true;
    return !terminalPanes.some((pane) => pane.paneId === targetPaneId && pane.sessionId === targetSessionId);
  }

  function detachPaneRuntimeViewForHiddenTab(runtime: TerminalPaneRuntime) {
    rememberTerminalTitleStateForRuntime(runtime);
    stopPollingForRuntime(runtime);
    clearStartupTimerForRuntime(runtime);
    disposePaneRuntime(runtime);
    runtime.host = null;
    runtime.visibleResizeSettled = false;
    runtime.visibleResizePromise = null;
    runtime.pollQueued = false;
    runtime.replayedSessionOutput = false;
  }

  function detachVisibleTerminalWorkbenchViewsForHiddenTab() {
    saveCurrentTerminalWorkbench();
    const visiblePaneIds = new Set(terminalPanes.map((pane) => pane.paneId));
    for (const runtime of paneRuntimes.values()) {
      if (!visiblePaneIds.has(runtime.paneId) || runtime.disposed) continue;
      detachPaneRuntimeViewForHiddenTab(runtime);
    }
    paneRuntimes = new Map(paneRuntimes);
  }

  function sessionForPane(pane: TerminalPaneModel) {
    return terminalSessions.find((item) => item.sessionId === pane.sessionId) ?? paneRuntimes.get(pane.paneId)?.session ?? null;
  }

  function ensureRuntimeForVisiblePane(pane: TerminalPaneModel) {
    const existing = paneRuntimes.get(pane.paneId);
    if (existing && !existing.disposed && existing.session.sessionId === pane.sessionId) {
      commitRuntime(existing);
      return existing;
    }
    const paneSession = sessionForPane(pane);
    if (!paneSession) return null;
    const runtime = createPaneRuntime({ ...paneSession, title: pane.title || paneSession.title }, pane.paneId);
    paneRuntimes = new Map(paneRuntimes).set(pane.paneId, runtime);
    commitRuntime(runtime);
    return runtime;
  }

  function restoreVisibleTerminalWorkbenchRuntimes() {
    let nextActiveRuntime: TerminalPaneRuntime | null = null;
    for (const pane of terminalPanes) {
      const runtime = ensureRuntimeForVisiblePane(pane);
      if (!runtime) continue;
      if (pane.paneId === activePaneId) nextActiveRuntime = runtime;
      ensureTerminalViewForPane(runtime);
      replayTerminalSessionOutput(runtime);
      startPollingForRuntime(runtime);
      void pollTerminalOutputForRuntime(runtime);
    }
    setActiveRuntime(nextActiveRuntime ?? activeRuntime() ?? (terminalPanes[0] ? ensureRuntimeForVisiblePane(terminalPanes[0]) : null));
  }

  function ensureWorkbenchForTabSession(nextSession: StackTerminalSession) {
    const existing = terminalTabWorkbenches.get(nextSession.sessionId);
    if (existing) return existing;
    const paneId = nextTerminalPaneId();
    const tree: TerminalPaneTreeNode = {
      kind: 'leaf',
      pane: {
        paneId,
        sessionId: nextSession.sessionId,
        title: nextSession.title || 'Terminal',
        focused: true
      }
    };
    const workbench = { tabSessionId: nextSession.sessionId, tree, activePaneId: paneId };
    terminalTabWorkbenches = new Map(terminalTabWorkbenches).set(nextSession.sessionId, workbench);
    return workbench;
  }

  function activateTerminalTabWorkbench(nextSession: StackTerminalSession) {
    if (activeTerminalTabSessionId === nextSession.sessionId) {
      focusTerminal();
      return activeRuntime();
    }
    saveCurrentTerminalWorkbench();
    ensureWorkbenchForTabSession(nextSession);
    const plan = planActivateTerminalTabWorkbench({
      activeTabSessionId: activeTerminalTabSessionId,
      targetTabSessionId: nextSession.sessionId,
      workbenches: terminalTabWorkbenchesList()
    });
    if (!plan.visibleTree || !plan.activeTabSessionId) return null;
    detachVisibleTerminalWorkbenchViewsForHiddenTab();
    activeTerminalTabSessionId = plan.activeTabSessionId;
    activePaneId = plan.activePaneId;
    setTerminalPaneTree(plan.visibleTree);
    restoreVisibleTerminalWorkbenchRuntimes();
    focusTerminal();
    return activeRuntime();
  }

  async function createTerminalSession() {
    if (terminalSessionCreationInFlight) return;
    terminalSessionCreationInFlight = true;
    const tabCreationWorkbenchSessionId = currentWorkbenchTabSessionId();
    saveCurrentTerminalWorkbench(tabCreationWorkbenchSessionId);
    try {
      const nextSession = await startTerminalPanelSessionInActiveCwd();
      markTerminalSessionAsTab(nextSession.sessionId);
      terminalSessions = [...terminalSessions.filter((item) => item.sessionId !== nextSession.sessionId), nextSession];
      if (listenersDisposed) {
        await stopStackTerminal(nextSession.sessionId).catch((error) => console.debug('Persistent terminal stale tab cleanup unavailable', error));
        clearStoppedTerminalSessionState(nextSession.sessionId);
        terminalSessions = terminalSessions.filter((item) => item.sessionId !== nextSession.sessionId);
        return;
      }
      const plan = planCreateTerminalTabWorkbench({
        activeTabSessionId: tabCreationWorkbenchSessionId,
        tabSessionIds: terminalTabSessionIds,
        paneOnlySessionIds: paneOnlyTerminalSessionIds,
        workbenches: terminalTabWorkbenchesList(),
        nextSession,
        nextPaneId: nextTerminalPaneId()
      });
      terminalTabSessionIds = new Set(plan.nextTabSessionIds);
      paneOnlyTerminalSessionIds = new Set(plan.nextPaneOnlySessionIds);
      replaceTerminalTabWorkbenches(plan.workbenches);
      orderTerminalSessionsByTabIds(plan.nextTabSessionIds);
      detachVisibleTerminalWorkbenchViewsForHiddenTab();
      activeTerminalTabSessionId = plan.activeTabSessionId;
      activePaneId = plan.activePaneId;
      setTerminalPaneTree(plan.visibleTree);
      const pane = terminalPanes.find((item) => item.paneId === plan.activePaneId && item.sessionId === nextSession.sessionId);
      const runtime = pane ? ensureRuntimeForVisiblePane(pane) : null;
      await tick();
      restoreVisibleTerminalWorkbenchRuntimes();
      if (runtime && isRuntimeCurrent(runtime)) {
        ensureTerminalViewForPane(runtime);
        replayTerminalSessionOutput(runtime);
        scheduleFitForRuntime(runtime);
        startStartupTimerForRuntime(runtime);
      }
      focusTerminal();
    } catch (error) {
      status = errorMessage(error, 'Terminal tab failed to start');
      console.error('Failed to create persistent terminal tab', error);
    } finally {
      terminalSessionCreationInFlight = false;
    }
  }

  async function createSplitPaneSession(direction: TerminalSplitDirection) {
    const targetPane = terminalPanes.find((pane) => pane.paneId === activePaneId) ?? terminalPanes[0] ?? null;
    if (!targetPane) return;
    const targetPaneId = targetPane.paneId;
    const targetSessionId = targetPane.sessionId;
    const splitStartGeneration = terminalWorkbenchGeneration;
    const splitStartTabSessionId = currentWorkbenchTabSessionId();
    const nextSession = await startTerminalPanelSessionInActiveCwd();
    if (isSplitStartStale(splitStartGeneration, splitStartTabSessionId, targetPaneId, targetSessionId)) {
      await stopStackTerminal(nextSession.sessionId).catch((error) => console.debug('Persistent terminal stale split cleanup unavailable', error));
      clearStoppedTerminalSessionState(nextSession.sessionId);
      terminalSessions = terminalSessions.filter((item) => item.sessionId !== nextSession.sessionId);
      return;
    }
    markTerminalSessionAsPaneOnly(nextSession.sessionId);
    terminalSessions = [...terminalSessions.filter((item) => item.sessionId !== nextSession.sessionId), nextSession];
    const pane: TerminalPaneModel = {
      paneId: nextTerminalPaneId(),
      sessionId: nextSession.sessionId,
      title: nextSession.title || `Session ${terminalPanes.length + 1}`,
      focused: true
    };
    activePaneId = pane.paneId;
    if (!terminalPaneTree || !targetPaneId) {
      setTerminalPaneTree({ kind: 'leaf', pane });
    } else {
      setTerminalPaneTree(splitPaneTreeAtLeaf(terminalPaneTree, targetPaneId, pane, direction));
    }
    const runtime = createPaneRuntime(nextSession, pane.paneId);
    commitRuntime(runtime);
    setActiveRuntime(runtime);
    ensureTerminalViewForPane(runtime);
    await tick();
    if (!isRuntimeCurrent(runtime)) return;
    ensureTerminalViewForPane(runtime);
    replayTerminalSessionOutput(runtime);
    scheduleFitForRuntime(runtime);
    startStartupTimerForRuntime(runtime);
    startPollingForRuntime(runtime);
    void pollTerminalOutputForRuntime(runtime);
    focusTerminal();
  }

  async function renameActiveSession() {
    if (!session) return;
    const title = window.prompt('Rename terminal session', terminalDisplayTitle(session));
    if (!title) return;
    const renamed = await renameStackTerminal(session.sessionId, title);
    manuallyRenamedTerminalSessions.add(renamed.sessionId);
    session = renamed;
    terminalSessions = terminalSessions.map((item) => item.sessionId === renamed.sessionId ? renamed : item);
    setTerminalPaneTree(mapTerminalPaneTree(terminalPaneTree, (pane) => pane.sessionId === renamed.sessionId ? { ...pane, title: renamed.title || title } : pane), false);
  }

  async function splitTerminal(orientation: Exclude<TerminalSplitOrientation, 'single' | 'mixed'>) {
    const direction: TerminalSplitDirection = orientation === 'vertical' ? 'right' : 'down';
    splitOrientation = orientation;
    if (!terminalPanes.length) {
      await startTerminal('user-action');
    }
    await createSplitPaneSession(direction);
  }

  function focusNextPane(direction = 1) {
    if (!terminalPanes.length) return;
    const currentIndex = terminalPanes.findIndex((pane) => pane.paneId === activePaneId);
    const nextIndex = (Math.max(0, currentIndex) + direction + terminalPanes.length) % terminalPanes.length;
    const pane = terminalPanes[nextIndex];
    if (pane) {
      activatePane(pane.paneId);
      focusTerminal();
    }
  }

  function isAltBackquoteHotkey(event: KeyboardEvent) {
    return event.altKey && !event.ctrlKey && !event.metaKey && (event.key === '`' || event.code === 'Backquote');
  }

  function closeFromTerminalHotkey(event: KeyboardEvent) {
    event.preventDefault();
    event.stopPropagation();
    void hideTerminalPanel().catch((error) => {
      console.error('Failed to hide terminal panel from hotkey', error);
    });
  }

  function createPaneRuntime(nextSession: StackTerminalSession, paneId: string): TerminalPaneRuntime {
    return {
      paneId,
      runtimeId: nextTerminalRuntimeId(),
      disposed: false,
      host: null,
      terminal: null,
      fitAddon: null,
      searchAddon: null,
      resizeObserver: null,
      resizeFrame: null,
      session: nextSession,
      lifecycle: 'waiting',
      status: 'Waiting for terminal output...',
      outputReceived: false,
      startupTimer: null,
      pollTimer: null,
      pollInFlight: false,
      pollQueued: false,
      operationQueue: Promise.resolve(),
      writeQueue: Promise.resolve(),
      renderedSequences: new Set<string>(renderedSequenceKeysBySession.get(nextSession.sessionId) ?? []),
      lastResizeKey: '',
      visibleResizeSettled: false,
      visibleResizePromise: null,
      currentInputText: '',
      currentInputSelectionActive: false,
      commandState: createTerminalCommandState(nextSession.sessionId, nextSession.cwd),
      selectedCommandIndex: -1,
      shellCwdMarkerSeen: false,
      shellParserDisposers: [],
      recentOutputText: '',
      replayedSessionOutput: false
    };
  }

  function ensureTerminalView() {
    const runtime = activeRuntime();
    if (runtime) {
      ensureTerminalViewForPane(runtime);
      return;
    }
    if (!host) {
      return;
    }
    if (terminal && terminal.element && host.contains(terminal.element)) {
      return;
    }
    disposeTerminalView();
    terminal = new Terminal({
      allowProposedApi: false,
      convertEol: false,
      cursorBlink: true,
      fontFamily: TERMINAL_PANEL_FONT_FAMILY,
      fontSize: terminalFontSize,
      lineHeight: 1.25,
      scrollback: 8000,
      letterSpacing: 0,
      screenReaderMode: false,
      windowsPty: { backend: 'conpty' }
    });
    fitAddon = new FitAddon();
    searchAddon = new SearchAddon();
    terminal.loadAddon(fitAddon);
    terminal.loadAddon(searchAddon);
    terminal.onData((data) => {
      trackTerminalInput(data);
      void writeTerminalData(data);
    });
    registerShellIntegrationParser(terminal);
    terminal.attachCustomKeyEventHandler((event) => {
      if (event.type === 'keydown' && isAltBackquoteHotkey(event)) {
        closeFromTerminalHotkey(event);
        return false;
      }
      if (event.type === 'keyup' && (event.key === '`' || event.code === 'Backquote')) {
        event.preventDefault();
        event.stopPropagation();
        return false;
      }
      if (event.type === 'keydown' && quickSelectOpen) {
        if (event.key === 'Escape') {
          quickSelectOpen = false;
          return false;
        }
        const target = quickSelectTargets.find((candidate) => candidate.label === event.key.toLowerCase());
        if (target) {
          void openQuickSelectTarget(target);
          return false;
        }
        return false;
      }
      if (event.type === 'keydown' && isTerminalFontZoomKey(event)) {
        event.preventDefault();
        event.stopPropagation();
        zoomTerminalFont(event.key === '-' ? -1 : 1);
        return false;
      }
      if (event.type === 'keydown' && event.ctrlKey && event.key.toLowerCase() === 'f') {
        openSearch();
        return false;
      }
      if (event.type === 'keydown' && searchOpen && event.key === 'Escape') {
        closeSearch();
        return false;
      }
      if (event.type === 'keydown' && currentInputSelectionActive && (event.key === 'Backspace' || event.key === 'Delete')) {
        event.preventDefault();
        event.stopPropagation();
        void deleteSelectedCurrentInput();
        return false;
      }
      if (event.type === 'keydown' && event.ctrlKey && event.key.toLowerCase() === 'c' && terminal?.hasSelection()) {
        event.preventDefault();
        event.stopPropagation();
        void copySelection();
        return false;
      }
      if (event.type === 'keydown' && event.altKey && event.key === 'ArrowUp') {
        jumpToCommand(-1);
        return false;
      }
      if (event.type === 'keydown' && event.altKey && event.key === 'ArrowDown') {
        jumpToCommand(1);
        return false;
      }
      if (event.type === 'keydown' && event.ctrlKey && event.shiftKey && event.key.toLowerCase() === 'c') {
        void copySelectedCommandOutput();
        return false;
      }
      if (event.type === 'keydown' && event.ctrlKey && event.key.toLowerCase() === 'v') {
        event.preventDefault();
        event.stopPropagation();
        void pasteClipboard();
        return false;
      }
      return true;
    });
    terminal.open(host);
    resizeObserver = new ResizeObserver(() => scheduleFit());
    resizeObserver.observe(host);
    void tick().then(() => {
      fitTerminal();
      scheduleFit();
    });
  }

  function ensureTerminalViewForPane(runtime: TerminalPaneRuntime) {
    if (!isRuntimeCurrent(runtime) || !runtime.host) return;
    if (runtime.terminal?.element && runtime.host.contains(runtime.terminal.element)) return;
    disposePaneRuntime(runtime);
    const paneTerminal = new Terminal({
      allowProposedApi: false,
      convertEol: false,
      cursorBlink: true,
      fontFamily: TERMINAL_PANEL_FONT_FAMILY,
      fontSize: terminalFontSize,
      lineHeight: 1.25,
      scrollback: 8000,
      letterSpacing: 0,
      screenReaderMode: false,
      windowsPty: { backend: 'conpty' }
    });
    runtime.terminal = paneTerminal;
    runtime.fitAddon = new FitAddon();
    runtime.searchAddon = new SearchAddon();
    paneTerminal.loadAddon(runtime.fitAddon);
    paneTerminal.loadAddon(runtime.searchAddon);
    paneTerminal.onData((data) => {
      if (!isRuntimeCurrent(runtime)) return;
      trackTerminalInputForRuntime(runtime, data);
      void writeTerminalDataForRuntime(runtime, data);
    });
    registerShellIntegrationParserForRuntime(runtime, paneTerminal);
    paneTerminal.attachCustomKeyEventHandler((event) => handleTerminalKeyForRuntime(runtime, event));
    paneTerminal.open(runtime.host);
    runtime.resizeObserver = new ResizeObserver(() => scheduleFitForRuntime(runtime));
    runtime.resizeObserver.observe(runtime.host);
    commitRuntime(runtime);
    void tick().then(() => {
      if (!isRuntimeCurrent(runtime)) return;
      void resizeTerminalToFitForRuntime(runtime);
      scheduleFitForRuntime(runtime);
      replayTerminalSessionOutput(runtime);
    });
  }

  function handleTerminalKeyForRuntime(runtime: TerminalPaneRuntime, event: KeyboardEvent) {
    if (!isRuntimeCurrent(runtime)) return false;
    if (event.type === 'keydown' && isAltBackquoteHotkey(event)) {
      closeFromTerminalHotkey(event);
      return false;
    }
    if (event.type === 'keyup' && (event.key === '`' || event.code === 'Backquote')) {
      event.preventDefault(); event.stopPropagation(); return false;
    }
    if (event.type === 'keydown' && event.altKey && event.key === 'ArrowRight') { focusNextPane(1); return false; }
    if (event.type === 'keydown' && event.altKey && event.key === 'ArrowLeft') { focusNextPane(-1); return false; }
    if (runtime.paneId !== activePaneId) {
      activatePane(runtime.paneId);
    }
    return terminal?.attachCustomKeyEventHandler ? (() => {
      // Mirror the active-pane handler logic without recursively installing a handler.
      if (event.type === 'keydown' && quickSelectOpen) {
        if (event.key === 'Escape') { quickSelectOpen = false; return false; }
        const target = quickSelectTargets.find((candidate) => candidate.label === event.key.toLowerCase());
        if (target) { void openQuickSelectTarget(target); return false; }
        return false;
      }
      if (event.type === 'keydown' && isTerminalFontZoomKey(event)) { event.preventDefault(); event.stopPropagation(); zoomTerminalFont(event.key === '-' ? -1 : 1); return false; }
      if (event.type === 'keydown' && event.ctrlKey && event.key.toLowerCase() === 'f') { openSearch(); return false; }
      if (event.type === 'keydown' && searchOpen && event.key === 'Escape') { closeSearch(); return false; }
      if (event.type === 'keydown' && runtime.currentInputSelectionActive && (event.key === 'Backspace' || event.key === 'Delete')) { event.preventDefault(); event.stopPropagation(); void deleteSelectedCurrentInputForRuntime(runtime); return false; }
      if (event.type === 'keydown' && event.ctrlKey && event.key.toLowerCase() === 'c' && runtime.terminal?.hasSelection()) { event.preventDefault(); event.stopPropagation(); void copySelectionForRuntime(runtime); return false; }
      if (event.type === 'keydown' && event.altKey && event.key === 'ArrowUp') { jumpToCommandForRuntime(runtime, -1); return false; }
      if (event.type === 'keydown' && event.altKey && event.key === 'ArrowDown') { jumpToCommandForRuntime(runtime, 1); return false; }
      if (event.type === 'keydown' && event.ctrlKey && event.shiftKey && event.key.toLowerCase() === 'c') { void copySelectedCommandOutput(); return false; }
      if (event.type === 'keydown' && event.ctrlKey && event.key.toLowerCase() === 'v') { event.preventDefault(); event.stopPropagation(); void pasteClipboardForRuntime(runtime); return false; }
      return true;
    })() : true;
  }

  function registerShellIntegrationParser(xterm: Terminal) {
    const parser = (xterm as unknown as { parser?: { registerOscHandler?: (id: number, handler: (data: string) => boolean) => { dispose: () => void } } }).parser;
    shellParserDisposers.forEach((dispose) => dispose());
    shellParserDisposers = [];
    const handlers = [
      parser?.registerOscHandler?.(133, (data) => handleShellIntegrationSequence(data)),
      parser?.registerOscHandler?.(1337, (data) => handleShellIntegrationSequence(data)),
      parser?.registerOscHandler?.(633, (data) => handleShellIntegrationSequence(data))
    ];
    shellParserDisposers = handlers
      .filter((handler): handler is { dispose: () => void } => Boolean(handler))
      .map((handler) => () => handler.dispose());
  }

  function registerShellIntegrationParserForRuntime(runtime: TerminalPaneRuntime, xterm: Terminal) {
    const parser = (xterm as unknown as { parser?: { registerOscHandler?: (id: number, handler: (data: string) => boolean) => { dispose: () => void } } }).parser;
    runtime.shellParserDisposers.forEach((dispose) => dispose());
    const handlers = [
      parser?.registerOscHandler?.(133, (data) => handleShellIntegrationSequenceForRuntime(runtime, data)),
      parser?.registerOscHandler?.(1337, (data) => handleShellIntegrationSequenceForRuntime(runtime, data)),
      parser?.registerOscHandler?.(633, (data) => handleShellIntegrationSequenceForRuntime(runtime, data))
    ];
    runtime.shellParserDisposers = handlers
      .filter((handler): handler is { dispose: () => void } => Boolean(handler))
      .map((handler) => () => handler.dispose());
    commitRuntime(runtime);
  }

  function handleShellIntegrationSequenceForRuntime(runtime: TerminalPaneRuntime, data: string) {
    if (!isRuntimeCurrent(runtime) || !runtime.commandState) return false;
    const marker = parseTerminalShellSequence(data);
    if (!marker) return false;
    const line = runtime.terminal ? runtime.terminal.buffer.active.baseY + runtime.terminal.buffer.active.cursorY : undefined;
    runtime.commandState = reduceTerminalShellMarker(runtime.commandState, marker, line);
    runtime.selectedCommandIndex = runtime.commandState.records.length - 1;
    if (marker.kind === 'cwd' && marker.cwd) {
      runtime.shellCwdMarkerSeen = true;
      applyAuthoritativeTerminalCwdForRuntime(runtime, marker.cwd);
    }
    if (marker.kind === 'end') {
      clearImportantTerminalActivity(runtime.session.sessionId, true);
    }
    commitRuntime(runtime);
    return true;
  }

  function handleShellIntegrationSequence(data: string) {
    if (!commandState) {
      return false;
    }
    const marker = parseTerminalShellSequence(data);
    if (!marker) {
      return false;
    }
    const line = terminal ? terminal.buffer.active.baseY + terminal.buffer.active.cursorY : undefined;
    commandState = reduceTerminalShellMarker(commandState, marker, line);
    selectedCommandIndex = commandState.records.length - 1;
    if (marker.kind === 'cwd' && marker.cwd) {
      shellCwdMarkerSeen = true;
      applyAuthoritativeTerminalCwd(marker.cwd);
    }
    if (marker.kind === 'end' && session?.sessionId) {
      clearImportantTerminalActivity(session.sessionId, true);
    }
    return true;
  }

  function applyAuthoritativeTerminalCwdForRuntime(runtime: TerminalPaneRuntime, cwd: string) {
    if (!isRuntimeCurrent(runtime) || !cwd) return;
    runtime.session = { ...runtime.session, cwd };
    if (runtime.commandState) runtime.commandState = { ...runtime.commandState, cwd };
    terminalSessions = terminalSessions.map((item) => item.sessionId === runtime.session.sessionId ? runtime.session : item);
    commitRuntime(runtime);
  }

  function applyAuthoritativeTerminalCwd(cwd: string) {
    if (!session || !cwd) {
      return;
    }
    session = { ...session, cwd };
    if (commandState) {
      commandState = { ...commandState, cwd };
    }
  }

  function emitTopBarTerminalActivity(sessionId: string, active: boolean, completed = false) {
    void emitTo(TOP_BAR_EVENT_TARGET, TOP_BAR_TERMINAL_ACTIVITY_EVENT, { sessionId, active, completed });
  }

  function clearImportantTerminalActivity(sessionId: string | undefined, completed = false) {
    if (!sessionId || !importantTerminalActivitySessions.delete(sessionId)) return;
    emitTopBarTerminalActivity(sessionId, false, completed);
  }

  function notifyTopBarForSubmittedCommand(sessionId: string | undefined, commandText: string) {
    if (!sessionId || !shouldAnimateTerminalCommand(commandText)) return;
    importantTerminalActivitySessions.add(sessionId);
    emitTopBarTerminalActivity(sessionId, true);
  }

  function notifyTopBarForImportantTerminalOutput(sessionId: string | undefined) {
    if (!sessionId || !importantTerminalActivitySessions.has(sessionId)) return;
    emitTopBarTerminalActivity(sessionId, true);
  }

  function trackTerminalInputForRuntime(runtime: TerminalPaneRuntime, data: string) {
    if (!isRuntimeCurrent(runtime)) return;
    runtime.currentInputSelectionActive = false;
    for (const ch of data) {
      if (ch === '\r' || ch === '\n' || ch === '\u0003') {
        if ((ch === '\r' || ch === '\n') && runtime.currentInputText.trim()) {
          notifyTopBarForSubmittedCommand(runtime.session.sessionId, runtime.currentInputText);
          if (runtime.commandState) {
            const line = runtime.terminal ? runtime.terminal.buffer.active.baseY + runtime.terminal.buffer.active.cursorY : undefined;
            runtime.commandState = beginTerminalCommandRecord(runtime.commandState, runtime.currentInputText.trim(), line);
            runtime.selectedCommandIndex = runtime.commandState.records.length - 1;
          }
        }
        runtime.currentInputText = '';
      } else if (ch === '\b' || ch === '\u007f') {
        runtime.currentInputText = runtime.currentInputText.slice(0, -1);
      } else if (!/^[\u0000-\u001f\u007f]$/.test(ch)) {
        runtime.currentInputText += ch;
      }
    }
    commitRuntime(runtime);
  }

  function trackTerminalInput(data: string) {
    currentInputSelectionActive = false;
    for (const ch of data) {
      if (ch === '\r' || ch === '\n' || ch === '\u0003') {
        if ((ch === '\r' || ch === '\n') && currentInputText.trim()) {
          notifyTopBarForSubmittedCommand(session?.sessionId, currentInputText);
          if (commandState) {
            const line = terminal ? terminal.buffer.active.baseY + terminal.buffer.active.cursorY : undefined;
            commandState = beginTerminalCommandRecord(commandState, currentInputText.trim(), line);
            selectedCommandIndex = commandState.records.length - 1;
          }
        }
        currentInputText = '';
      } else if (ch === '\b' || ch === '\u007f') {
        currentInputText = currentInputText.slice(0, -1);
      } else if (!/^[\u0000-\u001f\u007f]$/.test(ch)) {
        currentInputText += ch;
      }
    }
  }

  function selectCurrentInputTextForRuntime(runtime: TerminalPaneRuntime) {
    if (!isRuntimeCurrent(runtime) || !runtime.terminal || runtime.currentInputText.length === 0) return;
    const buffer = runtime.terminal.buffer.active;
    const row = buffer.baseY + buffer.cursorY;
    const startColumn = Math.max(0, buffer.cursorX - runtime.currentInputText.length);
    runtime.terminal.select(startColumn, row, runtime.currentInputText.length);
    runtime.currentInputSelectionActive = true;
    commitRuntime(runtime);
  }

  function selectCurrentInputText() {
    if (!terminal || currentInputText.length === 0) {
      return;
    }
    const buffer = terminal.buffer.active;
    const row = buffer.baseY + buffer.cursorY;
    const startColumn = Math.max(0, buffer.cursorX - currentInputText.length);
    terminal.select(startColumn, row, currentInputText.length);
    currentInputSelectionActive = true;
  }

  async function deleteSelectedCurrentInputForRuntime(runtime: TerminalPaneRuntime) {
    if (!isRuntimeCurrent(runtime)) return;
    const length = runtime.currentInputText.length;
    if (length <= 0) { runtime.currentInputSelectionActive = false; commitRuntime(runtime); return; }
    const eraseInput = '\u007f'.repeat(length);
    runtime.currentInputText = '';
    runtime.currentInputSelectionActive = false;
    runtime.terminal?.clearSelection();
    commitRuntime(runtime);
    await writeTerminalDataForRuntime(runtime, eraseInput);
  }

  async function deleteSelectedCurrentInput() {
    const length = currentInputText.length;
    if (length <= 0) {
      currentInputSelectionActive = false;
      return;
    }
    const eraseInput = '\u007f'.repeat(length);
    currentInputText = '';
    currentInputSelectionActive = false;
    terminal?.clearSelection();
    await writeTerminalData(eraseInput);
  }

  function handleTerminalMouseDown(event: MouseEvent) {
    if (event.detail < 3) {
      return;
    }
    event.preventDefault();
    event.stopPropagation();
    void tick().then(selectCurrentInputText);
  }

  function removePaneRuntime(paneId: string, stopBackendSession: boolean, keepSessionTab = false) {
    const runtime = paneRuntimes.get(paneId);
    const paneSessionId = runtime?.session.sessionId ?? terminalPanes.find((pane) => pane.paneId === paneId)?.sessionId;
    if (!runtime) {
      setTerminalPaneTree(removePaneFromTree(terminalPaneTree, paneId));
      if (paneSessionId && !keepSessionTab) {
        terminalSessions = terminalSessions.filter((item) => item.sessionId !== paneSessionId);
        clearStoppedTerminalSessionState(paneSessionId);
      }
      if (paneSessionId && stopBackendSession) {
        void stopStackTerminal(paneSessionId).catch((error) => console.debug('Persistent terminal orphan cleanup unavailable', error));
      }
      return;
    }
    markPaneRuntimeDisposed(runtime);
    stopPollingForRuntime(runtime);
    clearStartupTimerForRuntime(runtime);
    disposePaneRuntime(runtime);
    if (paneRuntimes.get(paneId)?.runtimeId === runtime.runtimeId) {
      paneRuntimes.delete(paneId);
    }
    paneRuntimes = new Map(paneRuntimes);
    setTerminalPaneTree(removePaneFromTree(terminalPaneTree, paneId));
    if (!keepSessionTab) {
      terminalSessions = terminalSessions.filter((item) => item.sessionId !== runtime.session.sessionId);
      clearStoppedTerminalSessionState(runtime.session.sessionId);
    }
    if (stopBackendSession) {
      void stopStackTerminal(runtime.session.sessionId).catch((error) => console.debug('Persistent terminal orphan cleanup unavailable', error));
    }
  }

  function disposePaneRuntime(runtime: TerminalPaneRuntime) {
    const shouldReplayRetainedOutputOnReattach = !runtime.disposed && Boolean(runtime.terminal);
    if (runtime.resizeFrame !== null) {
      window.cancelAnimationFrame(runtime.resizeFrame);
      runtime.resizeFrame = null;
    }
    runtime.resizeObserver?.disconnect();
    runtime.resizeObserver = null;
    runtime.shellParserDisposers.forEach((dispose) => dispose());
    runtime.shellParserDisposers = [];
    runtime.fitAddon?.dispose();
    runtime.fitAddon = null;
    runtime.searchAddon?.dispose();
    runtime.searchAddon = null;
    runtime.terminal?.dispose();
    if (shouldReplayRetainedOutputOnReattach) runtime.replayedSessionOutput = false;
    runtime.terminal = null;
    runtime.lastResizeKey = '';
  }

  function disposeTerminalView() {
    if (resizeFrame !== null) {
      window.cancelAnimationFrame(resizeFrame);
      resizeFrame = null;
    }
    resizeObserver?.disconnect();
    resizeObserver = null;
    shellParserDisposers.forEach((dispose) => dispose());
    shellParserDisposers = [];
    fitAddon?.dispose();
    fitAddon = null;
    searchAddon?.dispose();
    searchAddon = null;
    terminal?.dispose();
    terminal = null;
    lastResizeKey = '';
    for (const runtime of paneRuntimes.values()) {
      disposePaneRuntime(runtime);
    }
  }

  function handlePanelOpen() {
    cancelIdlePrewarm();
    if (!session) {
      void startTerminal('first-open');
    }
    visibleResizeSettled = false;
    for (const runtime of paneRuntimes.values()) {
      if (!isRuntimeCurrent(runtime)) continue;
      runtime.visibleResizeSettled = false;
      ensureTerminalViewForPane(runtime);
      replayTerminalSessionOutput(runtime);
    }
    void scheduleFitAfterPanelOpen();
  }

  async function scheduleFitAfterPanelOpen() {
    await tick();
    focusTerminal();
    visibleResizePromise = resizeAllVisiblePanes()
      .catch((error) => {
        console.debug('Persistent terminal visible resize unavailable', error);
      })
      .finally(() => {
        visibleResizePromise = null;
      });
    await visibleResizePromise;
    scheduleFit();
    window.setTimeout(() => scheduleFit(), 60);
  }

  function scheduleFitForRuntime(runtime: TerminalPaneRuntime) {
    if (runtime.resizeFrame !== null) window.cancelAnimationFrame(runtime.resizeFrame);
    if (!isRuntimeCurrent(runtime)) {
      runtime.resizeFrame = null;
      return;
    }
    runtime.resizeFrame = window.requestAnimationFrame(() => {
      runtime.resizeFrame = null;
      if (!isRuntimeCurrent(runtime)) return;
      void resizeTerminalToFitForRuntime(runtime);
    });
    commitRuntime(runtime);
  }

  function scheduleFit() {
    if (resizeFrame !== null) {
      window.cancelAnimationFrame(resizeFrame);
    }
    resizeFrame = window.requestAnimationFrame(() => {
      resizeFrame = null;
      void resizeTerminalToFit();
    });
  }

  function fitTerminal() {
    void resizeAllVisiblePanes();
  }

  function clampTerminalFontSize(value: number) {
    return Math.min(TERMINAL_PANEL_MAX_FONT_SIZE, Math.max(TERMINAL_PANEL_MIN_FONT_SIZE, Math.round(value)));
  }

  function isTerminalFontZoomKey(event: KeyboardEvent) {
    if (!event.ctrlKey || event.altKey || event.metaKey) return false;
    if (event.key === '-' && !event.shiftKey) return true;
    return event.key === '+' || event.key === '=';
  }

  function zoomTerminalFont(delta: number) {
    setTerminalFontSize(terminalFontSize + delta);
  }

  function applyTerminalFontSizeToXterm(xterm: Terminal) {
    xterm.options.fontSize = terminalFontSize;
  }

  function setTerminalFontSize(nextSize: number) {
    const clamped = clampTerminalFontSize(nextSize);
    if (clamped === terminalFontSize) return;
    terminalFontSize = clamped;
    if (terminal) applyTerminalFontSizeToXterm(terminal);
    for (const runtime of paneRuntimes.values()) {
      if (!isRuntimeCurrent(runtime)) continue;
      if (runtime.terminal) applyTerminalFontSizeToXterm(runtime.terminal);
      runtime.lastResizeKey = '';
      runtime.visibleResizeSettled = false;
      scheduleFitForRuntime(runtime);
    }
    lastResizeKey = '';
    visibleResizeSettled = false;
    scheduleFit();
    void resizeAllVisiblePanes();
  }

  function handleTerminalFontZoomWheel(event: WheelEvent) {
    if (!event.ctrlKey) return;
    event.preventDefault();
    event.stopImmediatePropagation?.();
    event.stopPropagation();
    zoomTerminalFont(event.deltaY > 0 ? -1 : 1);
  }

  async function resizeAllVisiblePanes() {
    await Promise.all(Array.from(paneRuntimes.values()).filter((runtime) => isRuntimeCurrent(runtime)).map((runtime) => resizeTerminalToFitForRuntime(runtime).catch((error) => console.debug('Persistent terminal pane resize unavailable', error))));
  }

  async function resizeTerminalToFitForRuntime(runtime: TerminalPaneRuntime) {
    if (!isRuntimeCurrent(runtime) || !runtime.terminal || !runtime.fitAddon || !runtime.session) return;
    try {
      runtime.fitAddon.fit();
      const width = runtime.host?.clientWidth ?? 0;
      const height = runtime.host?.clientHeight ?? 0;
      const cols = runtime.terminal.cols;
      const rows = runtime.terminal.rows;
      const sessionId = runtime.session.sessionId;
      const resizeKey = `${sessionId}:${cols}:${rows}:${width}:${height}`;
      if (resizeKey === runtime.lastResizeKey || cols <= 0 || rows <= 0) {
        runtime.visibleResizeSettled = resizeKey === runtime.lastResizeKey && cols > 0 && rows > 0;
        commitRuntime(runtime);
        return;
      }
      runtime.lastResizeKey = resizeKey;
      await enqueueTerminalOperationForRuntime(runtime, () => resizeStackTerminal(sessionId, cols, rows, width, height));
      if (!isRuntimeCurrent(runtime)) return;
      runtime.visibleResizeSettled = true;
      commitRuntime(runtime);
    } catch (error) {
      if (!isRuntimeCurrent(runtime)) return;
      runtime.visibleResizeSettled = false;
      runtime.lastResizeKey = '';
      commitRuntime(runtime);
      console.debug('Persistent terminal pane fit unavailable', error);
      throw error;
    }
  }

  async function resizeTerminalToFit() {
    if (!terminal || !fitAddon || !session) {
      return;
    }
    try {
      fitAddon.fit();
      const width = host?.clientWidth ?? 0;
      const height = host?.clientHeight ?? 0;
      const cols = terminal.cols;
      const rows = terminal.rows;
      const sessionId = session.sessionId;
      const resizeKey = `${sessionId}:${cols}:${rows}:${width}:${height}`;
      if (resizeKey === lastResizeKey || cols <= 0 || rows <= 0) {
        visibleResizeSettled = resizeKey === lastResizeKey && cols > 0 && rows > 0;
        return;
      }
      lastResizeKey = resizeKey;
      await enqueueTerminalOperation(() => resizeStackTerminal(sessionId, cols, rows, width, height));
      visibleResizeSettled = true;
    } catch (error) {
      visibleResizeSettled = false;
      lastResizeKey = '';
      console.debug('Persistent terminal fit unavailable', error);
      throw error;
    }
  }

  async function ensureVisibleResizeBeforeInputForRuntime(runtime: TerminalPaneRuntime) {
    if (!isRuntimeCurrent(runtime)) return;
    if (runtime.visibleResizeSettled) return;
    if (runtime.visibleResizePromise) { await runtime.visibleResizePromise; return; }
    runtime.visibleResizePromise = resizeTerminalToFitForRuntime(runtime)
      .catch((error) => console.debug('Persistent terminal pane input proceeding without visible resize', error))
      .finally(() => { if (isRuntimeCurrent(runtime)) { runtime.visibleResizePromise = null; commitRuntime(runtime); } });
    commitRuntime(runtime);
    await runtime.visibleResizePromise;
  }

  async function ensureVisibleResizeBeforeInput() {
    if (visibleResizeSettled) {
      return;
    }
    if (visibleResizePromise) {
      await visibleResizePromise;
      return;
    }
    await resizeTerminalToFit().catch((error) => {
      console.debug('Persistent terminal input proceeding without visible resize', error);
    });
  }

  async function writeTerminalDataForRuntime(runtime: TerminalPaneRuntime, data: string) {
    if (!isRuntimeCurrent(runtime) || !runtime.session || !data) return;
    const sessionId = runtime.session.sessionId;
    await ensureVisibleResizeBeforeInputForRuntime(runtime);
    if (!isRuntimeCurrent(runtime)) return;
    await enqueueTerminalWriteForRuntime(runtime, () => writeStackTerminal(sessionId, data));
  }

  async function writeTerminalData(data: string) {
    if (!session || !data) {
      return;
    }
    const sessionId = session.sessionId;
    await ensureVisibleResizeBeforeInput();
    await enqueueTerminalWrite(() => writeStackTerminal(sessionId, data));
  }

  function enqueueTerminalWriteForRuntime<T>(runtime: TerminalPaneRuntime, operation: () => Promise<T>): Promise<T> {
    const runOperation = () => isRuntimeCurrent(runtime) ? operation() : Promise.resolve(undefined as T);
    const queued = runtime.writeQueue.then(runOperation, runOperation);
    runtime.writeQueue = queued.then(() => undefined, () => undefined);
    commitRuntime(runtime);
    return queued;
  }

  function enqueueTerminalWrite<T>(operation: () => Promise<T>): Promise<T> {
    const queued = writeQueue.then(operation, operation);
    writeQueue = queued.then(
      () => undefined,
      () => undefined
    );
    return queued;
  }

  function enqueueTerminalOperationForRuntime<T>(runtime: TerminalPaneRuntime, operation: () => Promise<T>): Promise<T> {
    const runOperation = () => isRuntimeCurrent(runtime) ? operation() : Promise.resolve(undefined as T);
    const queued = runtime.operationQueue.then(runOperation, runOperation);
    runtime.operationQueue = queued.then(() => undefined, () => undefined);
    commitRuntime(runtime);
    return queued;
  }

  function enqueueTerminalOperation<T>(operation: () => Promise<T>): Promise<T> {
    const queued = operationQueue.then(operation, operation);
    operationQueue = queued.then(
      () => undefined,
      () => undefined
    );
    return queued;
  }

  async function copySelectionForRuntime(runtime: TerminalPaneRuntime) {
    if (!isRuntimeCurrent(runtime)) return;
    const selection = runtime.terminal?.getSelection() ?? '';
    if (selection) await navigator.clipboard?.writeText(selection).catch((error) => console.debug('Persistent terminal selection copy unavailable', error));
  }

  async function copySelection() {
    const selection = terminal?.getSelection() ?? '';
    if (!selection) {
      return;
    }
    await navigator.clipboard?.writeText(selection).catch((error) => {
      console.debug('Persistent terminal selection copy unavailable', error);
    });
  }

  async function pasteClipboard() {
    const text = await navigator.clipboard?.readText().catch((error) => {
      console.debug('Persistent terminal clipboard paste unavailable', error);
      return '';
    });
    if (text) {
      await writeTerminalData(text);
    }
  }

  async function pasteClipboardForRuntime(runtime: TerminalPaneRuntime) {
    if (!isRuntimeCurrent(runtime)) return;
    const text = await navigator.clipboard?.readText().catch((error) => {
      console.debug('Persistent terminal clipboard paste unavailable', error);
      return '';
    });
    if (text) {
      await writeTerminalDataForRuntime(runtime, text);
    }
  }

  function commandRecordsForRuntime(runtime: TerminalPaneRuntime | null) {
    return runtime?.commandState?.records ?? [];
  }

  function commandRecords() {
    return commandState?.records ?? [];
  }

  function selectedCommand(): TerminalCommandRecord | null {
    const records = commandRecords();
    if (!records.length) {
      return null;
    }
    const index = selectedCommandIndex >= 0 ? selectedCommandIndex : records.length - 1;
    return records[Math.max(0, Math.min(records.length - 1, index))] ?? null;
  }

  function jumpToCommandForRuntime(runtime: TerminalPaneRuntime, direction: -1 | 1) {
    if (!isRuntimeCurrent(runtime)) return;
    const records = commandRecordsForRuntime(runtime);
    if (!runtime.terminal || !records.length) return;
    const current = runtime.selectedCommandIndex >= 0 ? runtime.selectedCommandIndex : records.length - 1;
    runtime.selectedCommandIndex = Math.max(0, Math.min(records.length - 1, current + direction));
    const line = records[runtime.selectedCommandIndex]?.startMarker ?? records[runtime.selectedCommandIndex]?.outputStartMarker;
    if (typeof line === 'number') runtime.terminal.scrollToLine(line);
    commitRuntime(runtime);
  }

  function jumpToCommand(direction: -1 | 1) {
    const records = commandRecords();
    if (!terminal || !records.length) {
      return;
    }
    const current = selectedCommandIndex >= 0 ? selectedCommandIndex : records.length - 1;
    selectedCommandIndex = Math.max(0, Math.min(records.length - 1, current + direction));
    const line = records[selectedCommandIndex]?.startMarker ?? records[selectedCommandIndex]?.outputStartMarker;
    if (typeof line === 'number') {
      terminal.scrollToLine(line);
    }
  }

  function commandOutputText(record: TerminalCommandRecord) {
    if (!terminal) {
      return '';
    }
    const buffer = terminal.buffer.active;
    const start = Math.max(0, record.outputStartMarker ?? record.startMarker ?? 0);
    const end = Math.max(start, record.endMarker ?? buffer.baseY + buffer.length - 1);
    const lines: string[] = [];
    for (let index = start; index <= end; index += 1) {
      const line = buffer.getLine(index);
      if (line) {
        lines.push(line.translateToString(true));
      }
    }
    return lines.join('\n').trim();
  }

  async function copySelectedCommandOutput() {
    const record = selectedCommand();
    if (!record) {
      return;
    }
    const output = commandOutputText(record);
    if (output) {
      await navigator.clipboard?.writeText(output).catch((error) => {
        console.debug('Persistent terminal command output copy unavailable', error);
      });
    }
  }

  async function copySelectedCommand() {
    const command = selectedCommand()?.commandText?.trim();
    if (command) {
      await navigator.clipboard?.writeText(command).catch((error) => console.debug('Persistent terminal command copy unavailable', error));
    }
  }

  async function rerunSelectedCommand(commandText = selectedCommand()?.commandText) {
    const command = commandText?.trim();
    if (command) {
      await writeTerminalData(`${command}\r`);
    }
  }

  function cdCommandForDirectory(dir: string) {
    return `cd ${quoteShellPath(dir)}`;
  }

  function quoteShellPath(path: string) {
    return `'${path.replace(/'/g, "''")}'`;
  }

  function openTerminalContextMenu(event: MouseEvent) {
    event.preventDefault();
    const position = positionScrollableContextMenuInViewport(
      { x: event.clientX, y: event.clientY },
      { width: 210, height: 320 },
      { width: window.innerWidth, height: window.innerHeight },
      8
    );
    contextMenu = { x: position.x, y: position.y };
    actionMenuOpen = false;
    recentMenuOpen = false;
    terminal?.focus();
  }

  function closeTerminalContextMenu() {
    contextMenu = null;
  }

  function closeTerminalMenus() {
    contextMenu = null;
    actionMenuOpen = false;
    recentMenuOpen = false;
    quickSelectOpen = false;
    searchOpen = false;
  }

  function closeTerminalMenusOnOutsidePointer(event: PointerEvent) {
    const target = event.target;
    if (!(target instanceof Element)) {
      return;
    }
    if (target.closest('.terminal-toolbar, .terminal-toolbar-menu, .terminal-panel-context-menu, .terminal-search, .terminal-quick-select')) {
      return;
    }
    closeTerminalMenus();
  }

  function effectiveTerminalCwd() {
    const runtime = activeRuntime();
    return runtime?.session.cwd || session?.cwd || runtime?.commandState?.cwd || commandState?.cwd || selectedCommand()?.cwd || '';
  }

  function selectedCommandFromState(state: TerminalCommandState | null, index: number): TerminalCommandRecord | null {
    const records = state?.records ?? [];
    if (!records.length) {
      return null;
    }
    const selectedIndex = index >= 0 ? index : records.length - 1;
    return records[Math.max(0, Math.min(records.length - 1, selectedIndex))] ?? null;
  }

  function buildTerminalActionState(
    currentSession: StackTerminalSession | null,
    currentTerminal: Terminal | null,
    panes: TerminalPaneModel[],
    backendSessions: StackTerminalSession[],
    currentActivePaneId: string,
    runtimes: Map<string, TerminalPaneRuntime>,
    detectedTargets: TerminalQuickSelectTarget[],
    currentCommandState: TerminalCommandState | null,
    currentSelectedCommandIndex: number,
    sessionCreationInFlight: boolean
  ): TerminalActionState {
    const candidateActivePaneRuntime = runtimes.get(currentActivePaneId) ?? null;
    const activePaneRuntime = candidateActivePaneRuntime && !candidateActivePaneRuntime.disposed ? candidateActivePaneRuntime : null;
    const record = selectedCommandFromState(currentCommandState, currentSelectedCommandIndex);
    const cwd = currentSession?.cwd || currentCommandState?.cwd || record?.cwd || '';
    const hasVisiblePaneSession = panes.some((pane) => Boolean(pane.sessionId));
    const hasActiveSession = Boolean(currentSession || activePaneRuntime?.session || hasVisiblePaneSession || backendSessions.length > 0);
    const selectedTerminal = currentTerminal ?? activePaneRuntime?.terminal ?? null;
    return {
      hasTerminal: Boolean(selectedTerminal || hasActiveSession),
      hasSession: hasActiveSession,
      hasSelection: Boolean(selectedTerminal?.hasSelection()),
      hasCommand: Boolean(record?.commandText),
      hasCommandOutput: Boolean(record && commandRecordHasOutput(record)),
      hasCwd: Boolean(cwd),
      hasDetectedTarget: detectedTargets.length > 0,
      hasRepo: Boolean(cwd),
      canCreateSession: !sessionCreationInFlight,
      canSplit: hasActiveSession,
      hasMultiplePanes: panes.length > 1
    };
  }

  function commandRecordHasOutput(record: TerminalCommandRecord) {
    return typeof record.outputStartMarker === 'number' || typeof record.endMarker === 'number';
  }

  function actionEnabled(id: TerminalActionId) {
    return Boolean(getTerminalAction(id)?.isEnabled(terminalActionState));
  }

  async function runTerminalAction(id: TerminalActionId) {
    closeTerminalMenus();
    switch (id) {
      case 'copySelection': return copySelection();
      case 'copyCommand': return copySelectedCommand();
      case 'copyCommandOutput': return copySelectedCommandOutput();
      case 'rerunCommand': return rerunSelectedCommand();
      case 'search': return openSearch();
      case 'clear': terminal?.clear(); return;
      case 'paste': return pasteClipboard();
      case 'openCwdInFiles': return openCwdInFiles();
      case 'openExternalTerminalHere': return openExternalTerminalHere();
      case 'openInVscode': return openCwdInVscode();
      case 'restartTerminal': return restartTerminal();
      case 'stopTerminal': return stopTerminal();
      case 'openDetectedTarget': return quickSelectTargets[0] ? openQuickSelectTarget(quickSelectTargets[0]) : undefined;
      case 'copyDetectedTarget': return quickSelectTargets[0] ? copyQuickSelectTarget(quickSelectTargets[0]) : undefined;
      case 'openGitWorkbench': return rerunSelectedCommand('git status --short');
      case 'newSession': return createTerminalSession();
      case 'renameSession': return renameActiveSession();
      case 'splitHorizontal': return splitTerminal('horizontal');
      case 'splitVertical': return splitTerminal('vertical');
      case 'closePane': return stopTerminal();
      case 'focusNextPane': focusNextPane(1); return;
      case 'focusPreviousPane': focusNextPane(-1); return;
    }
  }

  function openSearch() {
    searchOpen = true;
    void tick().then(() => document.querySelector<HTMLInputElement>('.terminal-search-input')?.focus());
  }

  function closeSearch() {
    searchOpen = false;
    searchQuery = '';
    focusTerminal();
  }

  function searchNext() {
    if (searchQuery) searchAddon?.findNext(searchQuery);
  }

  function searchPrevious() {
    if (searchQuery) searchAddon?.findPrevious(searchQuery);
  }

  function updateQuickSelectTargets() {
    const text = visibleTerminalText();
    quickSelectTargets = detectTerminalQuickSelectTargets(text, effectiveTerminalCwd());
  }

  function toggleQuickSelect() {
    updateQuickSelectTargets();
    quickSelectOpen = !quickSelectOpen;
    focusTerminal();
  }

  async function openQuickSelectTarget(target: TerminalQuickSelectTarget) {
    quickSelectOpen = false;
    if (target.kind === 'url' || target.kind === 'localhost') {
      window.open(target.target, '_blank', 'noopener,noreferrer');
    } else if (target.kind === 'windowsPath' || target.kind === 'relativePath' || target.kind === 'fileLine') {
      await openStackItem(target.target).catch((error) => console.debug('Persistent terminal target open unavailable', error));
    } else if (target.kind === 'gitHash' || target.kind === 'branch') {
      await navigator.clipboard?.writeText(target.target).catch(() => undefined);
    }
    focusTerminal();
  }

  async function copyQuickSelectTarget(target: TerminalQuickSelectTarget) {
    await navigator.clipboard?.writeText(target.target).catch(() => undefined);
    quickSelectOpen = false;
    focusTerminal();
  }

  function visibleTerminalText() {
    if (!terminal) return recentOutputText;
    const buffer = terminal.buffer.active;
    const start = Math.max(0, buffer.baseY);
    const end = Math.min(buffer.baseY + buffer.length - 1, buffer.baseY + terminal.rows + 80);
    const lines: string[] = [];
    for (let index = start; index <= end; index += 1) {
      const line = buffer.getLine(index);
      if (line) lines.push(line.translateToString(true));
    }
    return lines.join('\n').slice(-16000);
  }

  async function openCwdInFiles() {
    const cwd = effectiveTerminalCwd();
    if (cwd) await revealStackItem(cwd).catch((error) => console.debug('Persistent terminal cwd reveal unavailable', error));
  }

  async function openExternalTerminalHere() {
    const cwd = effectiveTerminalCwd();
    if (cwd) await openStackTerminalHere(cwd).catch((error) => console.debug('Persistent terminal external open unavailable', error));
  }

  async function openCwdInVscode() {
    const cwd = effectiveTerminalCwd();
    if (cwd) await openStackFolderInVscode(cwd).catch((error) => console.debug('Persistent terminal VS Code open unavailable', error));
  }

  function stopAndForgetTerminalSessionsInBackground(sessionIds: Iterable<string>) {
    const stoppedSessionIds = new Set(sessionIds);
    if (!stoppedSessionIds.size) return;
    for (const runtime of [...paneRuntimes.values()]) {
      if (!stoppedSessionIds.has(runtime.session.sessionId)) continue;
      markPaneRuntimeDisposed(runtime);
      stopPollingForRuntime(runtime);
      clearStartupTimerForRuntime(runtime);
      disposePaneRuntime(runtime);
      if (paneRuntimes.get(runtime.paneId)?.runtimeId === runtime.runtimeId) {
        paneRuntimes.delete(runtime.paneId);
      }
    }
    paneRuntimes = new Map(paneRuntimes);
    terminalSessions = terminalSessions.filter((item) => !stoppedSessionIds.has(item.sessionId));
    for (const stoppedSessionId of stoppedSessionIds) {
      clearStoppedTerminalSessionState(stoppedSessionId);
    }
    void Promise.all([...stoppedSessionIds].map((stoppedSessionId) => stopStackTerminal(stoppedSessionId).catch((error) => console.debug('Persistent terminal tab close unavailable', error))));
  }

  async function closeTerminalSessionTab(sessionId: string) {
    saveCurrentTerminalWorkbench();
    const plan = planCloseTerminalTabWorkbench({
      activeTabSessionId: activeTerminalTabSessionId,
      closeTabSessionId: sessionId,
      tabSessionIds: terminalTabSessionIds,
      paneOnlySessionIds: paneOnlyTerminalSessionIds,
      workbenches: terminalTabWorkbenchesList()
    });
    const closingActiveTab = activeTerminalTabSessionId === sessionId;
    terminalTabSessionIds = new Set(plan.nextTabSessionIds);
    paneOnlyTerminalSessionIds = new Set(plan.nextPaneOnlySessionIds);
    replaceTerminalTabWorkbenches(plan.workbenches);
    if (closingActiveTab) {
      activeTerminalTabSessionId = plan.activeTabSessionId;
      activePaneId = plan.activePaneId || 'terminal-pane-primary';
      setTerminalPaneTree(plan.visibleTree, true, false);
      if (plan.visibleTree) {
        restoreVisibleTerminalWorkbenchRuntimes();
        focusTerminal();
      } else {
        session = null;
        setActiveRuntime(null);
      }
    }
    stopAndForgetTerminalSessionsInBackground(plan.stopBackendSessionIds.length ? plan.stopBackendSessionIds : [sessionId]);
    orderTerminalSessionsByTabIds(plan.nextTabSessionIds);
  }

  async function stopTerminal() {
    const runtime = activeRuntime();
    if (!runtime) return;
    const sessionId = runtime.session.sessionId;
    if (terminalTabSessionIds.has(sessionId)) {
      await closeTerminalSessionTab(sessionId);
      return;
    }
    stopPollingForRuntime(runtime);
    clearStartupTimerForRuntime(runtime);
    runtime.lifecycle = 'exited';
    runtime.status = 'Terminal stopped';
    runtime.session = { ...runtime.session, running: false };
    markPaneRuntimeDisposed(runtime);
    await stopStackTerminal(sessionId).catch((error) => console.debug('Persistent terminal stop unavailable', error));
    disposePaneRuntime(runtime);
    if (paneRuntimes.get(runtime.paneId)?.runtimeId === runtime.runtimeId) {
      paneRuntimes.delete(runtime.paneId);
    }
    paneRuntimes = new Map(paneRuntimes);
    setTerminalPaneTree(removePaneFromTree(terminalPaneTree, runtime.paneId));
    terminalSessions = terminalSessions.filter((item) => item.sessionId !== sessionId);
    clearStoppedTerminalSessionState(sessionId);
    if (!terminalPanes.length) {
      splitOrientation = 'single';
      const nextSession = currentVisibleTerminalTabs().find((item) => item.running) ?? currentVisibleTerminalTabs()[0] ?? null;
      if (nextSession) {
        activateTerminalTabWorkbench(nextSession);
      } else {
        session = null;
        setActiveRuntime(null);
      }
      return;
    }
    activePaneId = terminalPanes[0].paneId;
    activatePane(activePaneId);
  }

  async function copySelectionFromContextMenu() {
    closeTerminalContextMenu();
    await copySelection();
  }

  async function pasteClipboardFromContextMenu() {
    closeTerminalContextMenu();
    await pasteClipboard();
  }

  function startPollingForRuntime(runtime: TerminalPaneRuntime) {
    stopPollingForRuntime(runtime);
    if (!isRuntimeCurrent(runtime)) return;
    runtime.pollTimer = window.setInterval(() => {
      if (!isRuntimeCurrent(runtime)) return;
      void pollTerminalOutputForRuntime(runtime);
    }, 1000);
    commitRuntime(runtime);
  }

  function stopPollingForRuntime(runtime: TerminalPaneRuntime) {
    if (runtime.pollTimer !== null) {
      window.clearInterval(runtime.pollTimer);
      runtime.pollTimer = null;
      commitRuntime(runtime);
    }
  }

  function startPolling() {
    stopPolling();
    pollTimer = window.setInterval(() => {
      void pollTerminalOutput();
    }, 1000);
  }

  function stopPolling() {
    if (pollTimer !== null) {
      window.clearInterval(pollTimer);
      pollTimer = null;
    }
  }

  async function pollTerminalOutputForRuntime(runtime: TerminalPaneRuntime) {
    if (!isRuntimeCurrent(runtime) || !runtime.session) { stopPollingForRuntime(runtime); return; }
    if (runtime.pollInFlight) { runtime.pollQueued = true; commitRuntime(runtime); return; }
    runtime.pollInFlight = true;
    commitRuntime(runtime);
    try {
      const sessionId = runtime.session.sessionId;
      const result = await enqueueTerminalOperationForRuntime(runtime, () => readStackTerminal(sessionId));
      if (!isRuntimeCurrent(runtime) || !result || result.sessionId !== runtime.session.sessionId) return;
      for (const chunk of result.chunks ?? []) writeTerminalChunkForRuntime(runtime, chunk);
      if (result.exited) {
        stopPollingForRuntime(runtime);
        clearStartupTimerForRuntime(runtime);
        runtime.lifecycle = runtime.outputReceived ? 'exited' : 'failed';
        runtime.status = runtime.outputReceived ? 'Terminal exited' : 'Terminal exited before output';
        runtime.session = { ...runtime.session, running: false };
        commitRuntime(runtime);
      }
    } catch (error) {
      if (!isRuntimeCurrent(runtime)) return;
      stopPollingForRuntime(runtime);
      runtime.lifecycle = runtime.outputReceived ? 'exited' : 'failed';
      runtime.status = errorMessage(error, 'Terminal output unavailable');
      commitRuntime(runtime);
      console.error('Failed to poll persistent terminal pane', error);
    } finally {
      if (!isRuntimeCurrent(runtime)) return;
      runtime.pollInFlight = false;
      if (runtime.pollQueued) { runtime.pollQueued = false; void pollTerminalOutputForRuntime(runtime); }
      commitRuntime(runtime);
    }
  }

  async function pollTerminalOutput() {
    if (!session) {
      stopPolling();
      return;
    }
    if (pollInFlight) {
      pollQueued = true;
      return;
    }
    pollInFlight = true;
    try {
      const sessionId = session.sessionId;
      const result = await enqueueTerminalOperation(() => readStackTerminal(sessionId));
      if (result.sessionId !== session?.sessionId) {
        return;
      }
      if (result.chunks?.length) {
        for (const chunk of result.chunks) {
          writeTerminalChunk(chunk);
        }
      }
      if (result.exited) {
        stopPolling();
        clearStartupTimer();
        lifecycle = outputReceived ? 'exited' : 'failed';
        status = outputReceived ? 'Terminal exited' : 'Terminal exited before output';
      }
    } catch (error) {
      stopPolling();
      lifecycle = outputReceived ? 'exited' : 'failed';
      status = errorMessage(error, 'Terminal output unavailable');
      console.error('Failed to poll persistent terminal', error);
    } finally {
      pollInFlight = false;
      if (pollQueued) {
        pollQueued = false;
        void pollTerminalOutput();
      }
    }
  }

  function rememberTerminalChunkForSession(chunk: StackTerminalOutputChunk) {
    if (typeof chunk.sequence === 'number') {
      const sequenceKey = `${chunk.sessionId}:${chunk.stream ?? 'stdout'}:${chunk.sequence}`;
      const sessionKeys = new Set(renderedSequenceKeysBySession.get(chunk.sessionId) ?? []);
      if (sessionKeys.has(sequenceKey)) return;
      sessionKeys.add(sequenceKey);
      renderedSequenceKeysBySession = new Map(renderedSequenceKeysBySession).set(chunk.sessionId, sessionKeys);
    }
    appendSessionReplayBuffer(chunk.sessionId, chunk.text);
  }

  function appendSessionReplayBuffer(sessionId: string, output: string) {
    if (!output) return;
    const next = `${sessionReplayBuffers.get(sessionId) ?? ''}${output}`.slice(-262144);
    sessionReplayBuffers = new Map(sessionReplayBuffers).set(sessionId, next);
  }

  function runtimeHasAttachedTerminal(runtime: TerminalPaneRuntime) {
    const element = runtime.terminal?.element;
    return Boolean(element && runtime.host?.contains(element));
  }

  function replayTerminalSessionOutput(runtime: TerminalPaneRuntime) {
    if (!isRuntimeCurrent(runtime) || runtime.replayedSessionOutput || !runtimeHasAttachedTerminal(runtime)) return;
    const replay = sessionReplayBuffers.get(runtime.session.sessionId) ?? '';
    if (!replay) return;
    runtime.replayedSessionOutput = true;
    if (terminalOutputHasVisibleText(replay)) {
      runtime.outputReceived = true;
      runtime.lifecycle = 'running';
      runtime.status = '';
      clearStartupTimerForRuntime(runtime);
    }
    runtime.terminal?.write(replay);
    commitRuntime(runtime);
  }

  function writeTerminalChunkForRuntime(runtime: TerminalPaneRuntime, chunk: StackTerminalOutputChunk) {
    if (!isRuntimeCurrent(runtime) || chunk.sessionId !== runtime.session.sessionId) return;
    if (typeof chunk.sequence === 'number') {
      const sequenceKey = `${chunk.sessionId}:${chunk.stream ?? 'stdout'}:${chunk.sequence}`;
      if (runtime.renderedSequences.has(sequenceKey)) return;
      runtime.renderedSequences.add(sequenceKey);
      renderedSequenceKeysBySession = new Map(renderedSequenceKeysBySession).set(chunk.sessionId, new Set(runtime.renderedSequences));
    }
    writeTerminalOutputForRuntime(runtime, chunk.text);
  }

  function writeTerminalChunk(chunk: StackTerminalOutputChunk) {
    if (typeof chunk.sequence === 'number') {
      const sequenceKey = `${chunk.sessionId}:${chunk.stream ?? 'stdout'}:${chunk.sequence}`;
      if (renderedSequences.has(sequenceKey)) {
        return;
      }
      renderedSequences.add(sequenceKey);
    }
    writeTerminalOutput(chunk.text);
  }

  function writeTerminalOutputForRuntime(runtime: TerminalPaneRuntime, output: string) {
    if (!isRuntimeCurrent(runtime)) return;
    const hasVisibleOutput = Boolean(output && terminalOutputHasVisibleText(output));
    const hasAttachedTerminal = runtimeHasAttachedTerminal(runtime);
    if (hasVisibleOutput && hasAttachedTerminal) {
      runtime.outputReceived = true;
      runtime.lifecycle = 'running';
      runtime.status = '';
      runtime.session = { ...runtime.session, lastOutputAt: Date.now(), running: true };
      clearStartupTimerForRuntime(runtime);
    } else if (hasVisibleOutput && !hasAttachedTerminal && runtime.lifecycle !== 'failed' && runtime.lifecycle !== 'exited') {
      runtime.status = 'Attaching terminal view...';
    }
    if (output) {
      appendSessionReplayBuffer(runtime.session.sessionId, output);
      runtime.recentOutputText = `${runtime.recentOutputText}${stripTerminalAnsiControls(output)}`.slice(-20000);
    }
    if (hasAttachedTerminal) runtime.terminal?.write(output);
    commitRuntime(runtime);
  }

  function writeTerminalOutput(output: string) {
    if (output && terminalOutputHasVisibleText(output)) {
      outputReceived = true;
      lifecycle = 'running';
      status = '';
      clearStartupTimer();
    }
    if (output) {
      recentOutputText = `${recentOutputText}${stripTerminalAnsiControls(output)}`.slice(-20000);

    }
    terminal?.write(output);
  }

  function terminalOutputHasVisibleText(output: string) {
    return stripTerminalAnsiControls(output).trim().length > 0;
  }

  function stripTerminalAnsiControls(output: string) {
    return output
      .replace(/\x1b\][^\x07]*(?:\x07|\x1b\\)/g, '')
      .replace(/\x1b(?:[@-Z\\-_]|\[[0-?]*[ -/]*[@-~])/g, '')
      .replace(/[\x00-\x08\x0b\x0c\x0e-\x1f\x7f]/g, '');
  }

  function attachPaneHost(node: HTMLDivElement, pane: TerminalPaneModel) {
    const runtime = paneRuntimes.get(pane.paneId);
    if (!runtime || runtime.session.sessionId !== pane.sessionId || !isRuntimeCurrent(runtime)) return null;
    runtime.host = node;
    commitRuntime(runtime);
    ensureTerminalViewForPane(runtime);
    replayTerminalSessionOutput(runtime);
    scheduleFitForRuntime(runtime);
    return runtime;
  }

  function bindPaneHost(node: HTMLDivElement, pane: TerminalPaneModel) {
    let boundPane = pane;
    let boundRuntimeId: string | null = null;
    const clearBoundHost = () => {
      if (!boundRuntimeId) return;
      const previousRuntime = [...paneRuntimes.values()].find((runtime) => runtime.runtimeId === boundRuntimeId);
      if (previousRuntime?.host === node) {
        previousRuntime.host = null;
        commitRuntime(previousRuntime);
      }
      boundRuntimeId = null;
    };
    boundRuntimeId = attachPaneHost(node, boundPane)?.runtimeId ?? null;
    return {
      update(nextPane: TerminalPaneModel) {
        const previousPane = boundPane;
        boundPane = nextPane;
        if (previousPane.paneId !== nextPane.paneId || previousPane.sessionId !== nextPane.sessionId) {
          clearBoundHost();
        }
        boundRuntimeId = attachPaneHost(node, boundPane)?.runtimeId ?? null;
      },
      destroy() {
        clearBoundHost();
      }
    };
  }

  function focusTerminal() {
    void tick().then(() => {
      const runtime = activeRuntime();
      if (runtime) {
        ensureTerminalViewForPane(runtime);
        runtime.terminal?.focus();
        return;
      }
      ensureTerminalView();
      terminal?.focus();
    });
  }

  function startStartupTimerForRuntime(runtime: TerminalPaneRuntime) {
    clearStartupTimerForRuntime(runtime);
    if (!isRuntimeCurrent(runtime)) return;
    runtime.startupTimer = window.setTimeout(() => {
      if (!isRuntimeCurrent(runtime)) return;
      if (!runtime.outputReceived && (runtime.lifecycle === 'starting' || runtime.lifecycle === 'waiting')) {
        runtime.status = 'Still waiting for terminal output...';
        commitRuntime(runtime);
      }
    }, 1200);
    commitRuntime(runtime);
  }

  function clearStartupTimerForRuntime(runtime: TerminalPaneRuntime) {
    if (runtime.startupTimer !== null) {
      window.clearTimeout(runtime.startupTimer);
      runtime.startupTimer = null;
      commitRuntime(runtime);
    }
  }

  function startStartupTimer() {
    clearStartupTimer();
    startupTimer = window.setTimeout(() => {
      if (!outputReceived && (lifecycle === 'starting' || lifecycle === 'waiting')) {
        status = 'Still waiting for terminal output...';
      }
    }, 1200);
  }

  function clearStartupTimer() {
    if (startupTimer !== null) {
      window.clearTimeout(startupTimer);
      startupTimer = null;
    }
  }

  async function restartTerminal() {
    const runtime = activeRuntime();
    if (!runtime) {
      await startTerminal();
      return;
    }
    const oldSession = runtime.session.sessionId;
    const restartWasPaneOnly = paneOnlyTerminalSessionIds.has(oldSession);
    const paneId = runtime.paneId;
    const oldTitle = terminalPanes.find((pane) => pane.paneId === paneId)?.title;
    const oldHost = runtime.host;
    const restartGeneration = terminalWorkbenchGeneration;
    const clearOldStoppedSession = () => {
      terminalSessions = terminalSessions.filter((item) => item.sessionId !== oldSession);
      clearStoppedTerminalSessionState(oldSession);
    };
    stopPollingForRuntime(runtime);
    clearStartupTimerForRuntime(runtime);
    markPaneRuntimeDisposed(runtime);
    await stopStackTerminal(oldSession).catch(() => undefined);
    disposePaneRuntime(runtime);
    if (paneRuntimes.get(paneId)?.runtimeId === runtime.runtimeId) {
      paneRuntimes.delete(paneId);
      paneRuntimes = new Map(paneRuntimes);
    }
    if (listenersDisposed || restartGeneration !== terminalWorkbenchGeneration || !terminalPanes.some((pane) => pane.paneId === paneId && pane.sessionId === oldSession)) {
      clearOldStoppedSession();
      return;
    }
    const nextSession = await startTerminalPanelSessionInActiveCwd();
    if (listenersDisposed || restartGeneration !== terminalWorkbenchGeneration || !terminalPanes.some((pane) => pane.paneId === paneId && pane.sessionId === oldSession)) {
      await stopStackTerminal(nextSession.sessionId).catch((error) => console.debug('Persistent terminal stale restart cleanup unavailable', error));
      clearStoppedTerminalSessionState(nextSession.sessionId);
      terminalSessions = terminalSessions.filter((item) => item.sessionId !== nextSession.sessionId);
      clearOldStoppedSession();
      return;
    }
    clearOldStoppedSession();
    if (restartWasPaneOnly) {
      markTerminalSessionAsPaneOnly(nextSession.sessionId);
    } else {
      markTerminalSessionAsTab(nextSession.sessionId);
      terminalTabWorkbenches.delete(oldSession);
      terminalTabWorkbenches = new Map(terminalTabWorkbenches);
      activeTerminalTabSessionId = nextSession.sessionId;
    }
    const nextRuntime = createPaneRuntime({ ...nextSession, title: oldTitle || nextSession.title }, paneId);
    nextRuntime.host = oldHost;
    setTerminalPaneTree(replacePaneSessionInTree(terminalPaneTree, paneId, nextSession, oldTitle || nextSession.title || 'Terminal'));
    commitRuntime(nextRuntime);
    terminalSessions = terminalSessions.filter((item) => item.sessionId !== nextSession.sessionId).concat(nextSession);
    activePaneId = paneId;
    setActiveRuntime(nextRuntime);
    ensureTerminalViewForPane(nextRuntime);
    startStartupTimerForRuntime(nextRuntime);
    startPollingForRuntime(nextRuntime);
    void pollTerminalOutputForRuntime(nextRuntime);
  }

  function terminalDisplayTitle(terminalSession: StackTerminalSession) {
    const runtime = runtimeForSession(terminalSession.sessionId);
    const titleState = terminalTitleStates.get(terminalSession.sessionId);
    return buildTerminalTabTitle({
      profileTitle: terminalSession.title,
      manualTitle: terminalManualTitle(terminalSession.sessionId, terminalSession.title),
      cwd: runtime?.session.cwd || titleState?.cwd || terminalSession.cwd,
      currentInputText: runtime?.currentInputText ?? titleState?.currentInputText,
      recentOutputText: runtime?.recentOutputText ?? titleState?.recentOutputText,
      commandState: runtime?.commandState ?? titleState?.commandState ?? null
    });
  }

  function paneDisplayTitle(pane: TerminalPaneModel) {
    const terminalSession = terminalSessions.find((item) => item.sessionId === pane.sessionId);
    if (terminalSession) return terminalDisplayTitle(terminalSession);
    const candidateRuntime = paneRuntimes.get(pane.paneId);
    const runtime = candidateRuntime && isRuntimeCurrent(candidateRuntime) ? candidateRuntime : null;
    const titleState = terminalTitleStates.get(pane.sessionId);
    return buildTerminalTabTitle({
      profileTitle: pane.title,
      manualTitle: terminalManualTitle(pane.sessionId, pane.title),
      cwd: runtime?.session.cwd || titleState?.cwd,
      currentInputText: runtime?.currentInputText ?? titleState?.currentInputText,
      recentOutputText: runtime?.recentOutputText ?? titleState?.recentOutputText,
      commandState: runtime?.commandState ?? titleState?.commandState ?? null
    });
  }

  function terminalManualTitle(sessionId: string, title?: string) {
    if (!title) return undefined;
    if (manuallyRenamedTerminalSessions.has(sessionId)) return title;
    return isDefaultTerminalProfileTitle(title) ? undefined : title;
  }

  function isDefaultTerminalProfileTitle(title: string) {
    return /^(Terminal|Windows Terminal|PowerShell|Git Bash)$/i.test(title.trim());
  }

  function errorMessage(error: unknown, fallback: string) {
    if (error instanceof Error && error.message) {
      return error.message;
    }
    if (typeof error === 'string' && error) {
      return error;
    }
    return fallback;
  }
</script>

<svelte:window on:keydown|capture={(event) => isAltBackquoteHotkey(event) && closeFromTerminalHotkey(event)} on:keyup|capture={(event) => {
  if (event.key === '`' || event.code === 'Backquote') {
    event.preventDefault();
    event.stopPropagation();
  }
}} />

{#snippet renderPaneTree(node: TerminalPaneTreeNode)}
  {#if node.kind === 'split'}
    {#key node.splitId}
      <div class="terminal-pane-split" data-split-direction={node.direction} data-split-id={node.splitId}>
        {@render renderPaneTree(node.first)}
        {@render renderPaneTree(node.second)}
      </div>
    {/key}
  {:else}
    {@const pane = node.pane}
    {#key paneDomKey(pane)}
      {@const paneRuntime = runtimeForPane(pane)}
      {@const displayTitle = paneDisplayTitle(pane)}
      <section class="terminal-pane" class:focused={pane.focused} data-pane-id={pane.paneId} aria-label={`Terminal pane ${displayTitle}`}>
        <div class="terminal-pane-chrome" title={displayTitle}>
          <span>{displayTitle}</span>
        </div>
        <!-- svelte-ignore a11y_no_noninteractive_element_interactions: xterm owns terminal interaction semantics; this handler only narrows triple-click selection. -->
        <div
          use:bindPaneHost={pane}
          class="terminal-panel-output"
          role="log"
          aria-label="Terminal output"
          data-pane-id={pane.paneId}
          on:mousedown|capture={(event) => { activatePane(pane.paneId); handleTerminalMouseDown(event); }}
          on:wheel|capture|nonpassive={handleTerminalFontZoomWheel}
          on:contextmenu={(event) => { activatePane(pane.paneId); openTerminalContextMenu(event); }}
        ></div>
        {#if paneRuntime && !paneRuntime.outputReceived && paneRuntime.lifecycle !== 'running'}
          <div class="terminal-panel-status" class:error={paneRuntime.lifecycle === 'failed'} role={paneRuntime.lifecycle === 'failed' ? 'alert' : 'status'} aria-live="polite">
            <strong>{paneRuntime.status}</strong>
            {#if paneRuntime.lifecycle === 'failed'}
              <span>Restart terminal or check PowerShell startup.</span>
            {:else}
              <span>JasonShell terminal starts with the app and stays alive while hidden.</span>
            {/if}
          </div>
        {/if}
      </section>
    {/key}
  {/if}
{/snippet}

<div class="terminal-panel" role="dialog" aria-label="Persistent terminal">
  <header class="terminal-panel-header">
    <div class="terminal-session-tabs" role="tablist" aria-label="Terminal sessions">
      {#each visibleTerminalTabs as terminalSession, index (terminalSession.sessionId)}
        {@const displayTitle = terminalDisplayTitle(terminalSession) || `Session ${index + 1}`}
        <div class="terminal-tab-shell" class:active={terminalSession.sessionId === activeTerminalTabSessionId} role="presentation">
          <button
            type="button"
            class="terminal-tab-button"
            role="tab"
            aria-selected={terminalSession.sessionId === activeTerminalTabSessionId}
            aria-label={`Switch to terminal session ${displayTitle}`}
            on:click={() => activateTerminalSession(terminalSession)}
            title={displayTitle}
          >
            <span>{displayTitle}</span>
          </button>
          <small class="terminal-tab-status" aria-hidden="true">{terminalSession.running ? '●' : '○'}</small>
          <button
            type="button"
            class="terminal-tab-close"
            aria-label={`Close terminal session ${displayTitle}`}
            title="Close terminal tab"
            on:click|stopPropagation={() => void closeTerminalSessionTab(terminalSession.sessionId)}
          >×</button>
        </div>
      {/each}
      <button
        type="button"
        class="terminal-tab-new"
        title="New terminal tab"
        aria-label="New terminal tab"
        disabled={!actionEnabled('newSession')}
        on:click={() => void runTerminalAction('newSession')}
      >＋</button>
    </div>
    <nav class="terminal-toolbar" aria-label="Terminal actions">
      <button type="button" title="Search" aria-label="Search terminal" disabled={!actionEnabled('search')} on:click={() => void runTerminalAction('search')}>⌕</button>
      <button type="button" title="Split pane right" aria-label="Split terminal pane right" on:click={() => void runTerminalAction('splitVertical')}>Split right</button>
      <button type="button" title="Split pane down" aria-label="Split terminal pane down" on:click={() => void runTerminalAction('splitHorizontal')}>Split down</button>
      <button type="button" title="Rename session" aria-label="Rename terminal session" disabled={!actionEnabled('renameSession')} on:click={() => void runTerminalAction('renameSession')}>Rename</button>
      <button type="button" title="Quick Select" aria-label="Open terminal Quick Select" on:click={toggleQuickSelect}>QS</button>
      <button type="button" title="Recent commands" aria-label="Open recent terminal commands" on:click={() => { recentMenuOpen = !recentMenuOpen; actionMenuOpen = false; }}>⌘</button>
      <button type="button" title="Reveal cwd in Files" aria-label="Reveal current terminal directory in Files" disabled={!actionEnabled('openCwdInFiles')} on:click={() => void runTerminalAction('openCwdInFiles')}>Files</button>
      <button type="button" title="Open external terminal here" aria-label="Open external terminal here" disabled={!actionEnabled('openExternalTerminalHere')} on:click={() => void runTerminalAction('openExternalTerminalHere')}>Ext</button>
      <button type="button" title="Terminal menu" aria-label="Open terminal actions menu" on:click={() => { actionMenuOpen = !actionMenuOpen; recentMenuOpen = false; }}>⋯</button>
      {#if actionMenuOpen}
        <div class="terminal-toolbar-menu" role="menu" tabindex="-1" on:pointerdown|stopPropagation>
          {#each toolbarActions as action}
            <button type="button" role="menuitem" class:danger={action.destructive} disabled={!actionEnabled(action.id)} on:click={() => void runTerminalAction(action.id)}>{action.label}</button>
          {/each}
          <button type="button" role="menuitem" disabled={!actionEnabled('openInVscode')} on:click={() => void runTerminalAction('openInVscode')}>Open cwd in VS Code</button>
          <button type="button" role="menuitem" disabled={!actionEnabled('openGitWorkbench')} on:click={() => void runTerminalAction('openGitWorkbench')}>Run Git status</button>
        </div>
      {/if}
      {#if recentMenuOpen}
        <div class="terminal-toolbar-menu terminal-recent-menu" role="menu" tabindex="-1" on:pointerdown|stopPropagation>
          <strong>Recent commands</strong>
          {#each visibleRecentCommands as recent}
            <button type="button" role="menuitem" on:click={() => void rerunSelectedCommand(recent.commandText)}>{recent.commandText}</button>
          {:else}
            <span>No commands yet</span>
          {/each}
          <strong>Recent dirs</strong>
          {#each visibleRecentDirectories as dir}
            <button type="button" role="menuitem" on:click={() => void rerunSelectedCommand(cdCommandForDirectory(dir))}>{dir}</button>
          {/each}
        </div>
      {/if}
    </nav>
  </header>
  <section class="terminal-panel-body" class:split-vertical={splitOrientation === 'vertical'} class:split-horizontal={splitOrientation === 'horizontal'}>
    <div class="terminal-pane-tree" data-split-orientation={splitOrientation}>
      {#if terminalPaneTree}
        {@render renderPaneTree(terminalPaneTree)}
      {/if}
    </div>
    {#if searchOpen}
      <form class="terminal-search" on:pointerdown|stopPropagation on:submit|preventDefault={searchNext}>
        <input class="terminal-search-input" bind:value={searchQuery} placeholder="Search" aria-label="Search terminal" />
        <button type="submit" aria-label="Find next terminal search match">Next</button>
        <button type="button" aria-label="Find previous terminal search match" on:click={searchPrevious}>Prev</button>
        <button type="button" aria-label="Close terminal search" on:click={closeSearch}>Esc</button>
      </form>
    {/if}
    {#if quickSelectOpen}
      <div class="terminal-quick-select" role="listbox" tabindex="-1" aria-label="Quick Select targets" on:pointerdown|stopPropagation>
        {#each quickSelectTargets as target}
          <button type="button" aria-label={`Open Quick Select target ${target.text}`} on:click={() => void openQuickSelectTarget(target)}><kbd>{target.label}</kbd>{target.text}</button>
        {:else}
          <span>No targets</span>
        {/each}
      </div>
    {/if}
    {#if contextMenu}
      <div
        class="terminal-panel-context-menu"
        role="menu"
        tabindex="-1"
        style={`left: ${contextMenu.x}px; top: ${contextMenu.y}px;`}
        on:pointerdown|stopPropagation
      >
        <button type="button" role="menuitem" disabled={!actionEnabled('copySelection')} on:click={() => void copySelectionFromContextMenu()}>Copy</button>
        <button type="button" role="menuitem" disabled={!actionEnabled('copyCommand')} on:click={() => void runTerminalAction('copyCommand')}>Copy command</button>
        <button type="button" role="menuitem" disabled={!actionEnabled('copyCommandOutput')} on:click={() => void copySelectedCommandOutput()}>Copy command output</button>
        <button type="button" role="menuitem" disabled={!actionEnabled('rerunCommand')} on:click={() => void runTerminalAction('rerunCommand')}>Rerun command</button>
        <button type="button" role="menuitem" on:click={() => { updateQuickSelectTargets(); quickSelectOpen = true; closeTerminalContextMenu(); }}>Quick Select target</button>
        <button type="button" role="menuitem" on:click={() => void pasteClipboardFromContextMenu()}>Paste</button>
        <button type="button" role="menuitem" disabled={!actionEnabled('openCwdInFiles')} on:click={() => void runTerminalAction('openCwdInFiles')}>Reveal cwd in Files</button>
        <button type="button" role="menuitem" disabled={!actionEnabled('openExternalTerminalHere')} on:click={() => void runTerminalAction('openExternalTerminalHere')}>Open external terminal</button>
        <button type="button" role="menuitem" disabled={!actionEnabled('openInVscode')} on:click={() => void runTerminalAction('openInVscode')}>Open cwd in VS Code</button>
      </div>
    {/if}
  </section>
</div>
