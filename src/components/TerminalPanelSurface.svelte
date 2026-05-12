<script lang="ts">
  import './TerminalPanelSurface.css';
  import '@xterm/xterm/css/xterm.css';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';
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
    stopStackTerminal,
    stopTerminalPanelSessions,
    writeStackTerminal,
    type StackTerminalOutputChunk,
    type StackTerminalSession
  } from '../lib/persistentTerminal';
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
  import { addShellSettingsChangeListener, loadShellSettings, type ShellSettings } from '../lib/settings';
  import { terminalThemeById, type TerminalTheme } from '../lib/terminalThemes.js';

  type TerminalLifecycleState = 'starting' | 'waiting' | 'running' | 'failed' | 'exited';
  type TerminalSplitOrientation = 'single' | 'vertical' | 'horizontal';
  type TerminalPaneModel = { paneId: string; sessionId: string; title: string; focused: boolean };
  type TerminalPaneRuntime = {
    paneId: string;
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

  const TERMINAL_PANEL_OPEN_EVENT = 'terminal-panel:open';

  const TERMINAL_PANEL_FONT_FAMILY = '"Cascadia Mono", "Cascadia Code", Consolas, ui-monospace, "SFMono-Regular", monospace';
  const TERMINAL_PANEL_DEFAULT_FONT_SIZE = 13;
  const TERMINAL_PANEL_MIN_FONT_SIZE = 9;
  const TERMINAL_PANEL_MAX_FONT_SIZE = 28;

  let host: HTMLDivElement | null = null;
  let terminal: Terminal | null = null;
  let fitAddon: FitAddon | null = null;
  let searchAddon: SearchAddon | null = null;
  let resizeObserver: ResizeObserver | null = null;
  let resizeFrame: number | null = null;
  let session: StackTerminalSession | null = null;
  let paneRuntimes = new Map<string, TerminalPaneRuntime>();
  let terminalSessions: StackTerminalSession[] = [];
  let terminalPanes: TerminalPaneModel[] = [];
  let terminalFontSize = TERMINAL_PANEL_DEFAULT_FONT_SIZE;
  let activePaneId = 'terminal-pane-primary';
  let splitOrientation: TerminalSplitOrientation = 'single';
  let status = 'Starting terminal...';
  let lifecycle: TerminalLifecycleState = 'starting';
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
  let renderedSequenceKeysBySession = new Map<string, Set<string>>();
  let currentTerminalTheme: TerminalTheme = terminalThemeById('base-dark');

  let currentInputText = '';
  let currentInputSelectionActive = false;
  let visibleResizeSettled = false;
  let visibleResizePromise: Promise<void> | null = null;
  let commandState: TerminalCommandState | null = null;
  let selectedCommandIndex = -1;
  let shellCwdMarkerSeen = false;
  let shellParserDisposers: Array<() => void> = [];

  $: terminalActionState = buildTerminalActionState();
  $: visibleRecentCommands = recentTerminalCommands(activeRuntime()?.commandState ?? commandState);
  $: visibleRecentDirectories = recentTerminalDirectories(activeRuntime()?.commandState ?? commandState);
  $: toolbarActions = terminalActions.filter((action) => ['search', 'newSession', 'renameSession', 'splitHorizontal', 'splitVertical', 'openCwdInFiles', 'openExternalTerminalHere', 'restartTerminal', 'stopTerminal'].includes(action.id));

  function activeRuntime() {
    return paneRuntimes.get(activePaneId) ?? null;
  }

  function runtimeForSession(sessionId: string) {
    for (const runtime of paneRuntimes.values()) {
      if (runtime.session.sessionId === sessionId) {
        return runtime;
      }
    }
    return null;
  }

  function setActiveRuntime(runtime: TerminalPaneRuntime | null) {
    session = runtime?.session ?? null;
    terminal = runtime?.terminal ?? null;
    fitAddon = runtime?.fitAddon ?? null;
    searchAddon = runtime?.searchAddon ?? null;
    host = runtime?.host ?? null;
    lifecycle = runtime?.lifecycle ?? 'starting';
    status = runtime?.status ?? 'Starting terminal...';
    outputReceived = Boolean(runtime?.outputReceived);
    commandState = runtime?.commandState ?? null;
    selectedCommandIndex = runtime?.selectedCommandIndex ?? -1;
    shellCwdMarkerSeen = Boolean(runtime?.shellCwdMarkerSeen);
    currentInputText = runtime?.currentInputText ?? '';
    currentInputSelectionActive = Boolean(runtime?.currentInputSelectionActive);
    recentOutputText = runtime?.recentOutputText ?? '';
  }

  function commitRuntime(runtime: TerminalPaneRuntime) {
    paneRuntimes = new Map(paneRuntimes).set(runtime.paneId, runtime);
    if (runtime.paneId === activePaneId) {
      setActiveRuntime(runtime);
    }
  }

  onMount(() => {
    listenersDisposed = false;
    document.addEventListener('pointerdown', closeTerminalMenusOnOutsidePointer, true);
    window.addEventListener('focus', handlePanelOpen);
    const disposeSettingsListener = addShellSettingsChangeListener(applyShellSettingsToTerminalTheme);
    unlisteners.push(disposeSettingsListener);
    void loadTerminalThemeSetting();
    void initializeTerminalListeners();
    void startTerminal();
    return () => {
      listenersDisposed = true;
      document.removeEventListener('pointerdown', closeTerminalMenusOnOutsidePointer, true);
      window.removeEventListener('focus', handlePanelOpen);
      for (const runtime of paneRuntimes.values()) {
        stopPollingForRuntime(runtime);
        clearStartupTimerForRuntime(runtime);
      }
      stopPolling();
      clearStartupTimer();
      disposeTerminalView();
      for (const unlisten of unlisteners.splice(0)) {
        unlisten();
      }
    };
  });

  async function loadTerminalThemeSetting() {
    try {
      applyShellSettingsToTerminalTheme(await loadShellSettings());
    } catch (error) {
      console.error('Failed to load terminal theme setting', error);
      applyTerminalTheme(terminalThemeById(currentTerminalTheme.id));
    }
  }

  function applyShellSettingsToTerminalTheme(settings: ShellSettings) {
    applyTerminalTheme(terminalThemeById(settings.stackBrowser?.terminalTheme));
  }

  function applyTerminalTheme(nextTheme: TerminalTheme) {
    currentTerminalTheme = nextTheme;
    const xtermTheme = { ...nextTheme.theme };
    if (terminal) {
      terminal.options.theme = { ...xtermTheme };
    }
    for (const runtime of paneRuntimes.values()) {
      if (runtime.terminal) {
        runtime.terminal.options.theme = { ...xtermTheme };
      }
    }
  }

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
        handlePanelOpen();
      })
    );

    register(
      listen<TerminalOutputPayload>('stack-terminal:output', (event) => {
        const runtime = runtimeForSession(event.payload.sessionId);
        if (!runtime) {
          rememberTerminalChunkForSession(event.payload);
          return;
        }
        writeTerminalChunkForRuntime(runtime, event.payload);
      })
    );

    register(
      listen<TerminalClosedPayload>('stack-terminal:closed', (event) => {
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

  async function startTerminal() {
    status = 'Starting terminal...';
    lifecycle = 'starting';
    outputReceived = false;
    renderedSequences = new Set<string>();
    ensureTerminalView();
    startStartupTimer();
    try {
      await refreshTerminalSessionList();
      session = terminalSessions.find((candidate) => candidate.running) ?? terminalSessions[0] ?? await startPersistentTerminal();
      await refreshTerminalSessionList();
      const runtime = ensurePrimaryPaneForSession(session);
      startStartupTimerForRuntime(runtime);
      commandState = runtime.commandState;
      selectedCommandIndex = -1;
      shellCwdMarkerSeen = false;
      lifecycle = 'waiting';
      status = 'Waiting for terminal output...';
      handlePanelOpen();
      startPollingForRuntime(runtime);
      void pollTerminalOutputForRuntime(runtime);
      focusTerminal();
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
    terminalSessions = backendSessions;
    for (const runtime of paneRuntimes.values()) {
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
    } else if (!session && backendSessions.length) {
      session = backendSessions[0];
    }
  }

  function ensurePrimaryPaneForSession(nextSession: StackTerminalSession): TerminalPaneRuntime {
    const existing = terminalPanes.find((pane) => pane.sessionId === nextSession.sessionId);
    if (existing) {
      activePaneId = existing.paneId;
      const existingRuntime = paneRuntimes.get(existing.paneId);
      if (existingRuntime) {
        setActiveRuntime(existingRuntime);
        return existingRuntime;
      }
    }
    if (terminalPanes.length >= 2) {
      const [removedPane] = terminalPanes;
      if (removedPane) {
        removePaneRuntime(removedPane.paneId, false, true);
      }
    }
    const paneCount = Math.min(terminalPanes.length + 1, 2);
    const pane: TerminalPaneModel = {
      paneId: terminalPanes.length ? `terminal-pane-${Date.now().toString(36)}` : 'terminal-pane-primary',
      sessionId: nextSession.sessionId,
      title: nextSession.title || `Session ${paneCount}`,
      focused: true
    };
    terminalPanes = [...terminalPanes.map((item) => ({ ...item, focused: false })), pane];
    activePaneId = pane.paneId;
    const runtime = createPaneRuntime(nextSession, pane.paneId);
    commitRuntime(runtime);
    setActiveRuntime(runtime);
    return runtime;
  }

  function activatePane(paneId: string) {
    activePaneId = paneId;
    terminalPanes = terminalPanes.map((pane) => ({ ...pane, focused: pane.paneId === activePaneId }));
    setActiveRuntime(activeRuntime());
  }

  function activateTerminalSession(nextSession: StackTerminalSession) {
    openSessionAsTab(nextSession);
    focusTerminal();
  }

  function openSessionAsTab(nextSession: StackTerminalSession): TerminalPaneRuntime {
    for (const pane of terminalPanes) {
      removePaneRuntime(pane.paneId, false, true);
    }
    splitOrientation = 'single';
    activePaneId = 'terminal-pane-primary';
    const pane: TerminalPaneModel = {
      paneId: activePaneId,
      sessionId: nextSession.sessionId,
      title: nextSession.title || 'Terminal',
      focused: true
    };
    terminalPanes = [pane];
    const runtime = createPaneRuntime(nextSession, pane.paneId);
    commitRuntime(runtime);
    setActiveRuntime(runtime);
    ensureTerminalViewForPane(runtime);
    replayTerminalSessionOutput(runtime);
    startPollingForRuntime(runtime);
    void pollTerminalOutputForRuntime(runtime);
    return runtime;
  }

  async function createTerminalSession() {
    const nextSession = await startPersistentTerminal();
    terminalSessions = [...terminalSessions, nextSession];
    const runtime = openSessionAsTab(nextSession);
    startStartupTimerForRuntime(runtime);
    focusTerminal();
  }

  async function createSplitPaneSession(orientation: Exclude<TerminalSplitOrientation, 'single'>) {
    const nextSession = await startPersistentTerminal();
    terminalSessions = [...terminalSessions, nextSession];
    const runtime = ensurePrimaryPaneForSession(nextSession);
    ensureTerminalViewForPane(runtime);
    startStartupTimerForRuntime(runtime);
    startPollingForRuntime(runtime);
    void pollTerminalOutputForRuntime(runtime);
    splitOrientation = orientation;
  }

  async function renameActiveSession() {
    if (!session) return;
    const title = window.prompt('Rename terminal session', session.title || 'Terminal');
    if (!title) return;
    const renamed = await renameStackTerminal(session.sessionId, title);
    session = renamed;
    terminalSessions = terminalSessions.map((item) => item.sessionId === renamed.sessionId ? renamed : item);
    terminalPanes = terminalPanes.map((pane) => pane.sessionId === renamed.sessionId ? { ...pane, title: renamed.title || title } : pane);
  }

  async function splitTerminal(orientation: Exclude<TerminalSplitOrientation, 'single'>) {
    splitOrientation = orientation;
    if (terminalPanes.length < 2) {
      await createSplitPaneSession(orientation);
    }
  }

  function focusNextPane(direction = 1) {
    if (!terminalPanes.length) return;
    const currentIndex = terminalPanes.findIndex((pane) => pane.paneId === activePaneId);
    const nextIndex = (Math.max(0, currentIndex) + direction + terminalPanes.length) % terminalPanes.length;
    const pane = terminalPanes[nextIndex];
    const nextSession = terminalSessions.find((item) => item.sessionId === pane.sessionId);
    if (nextSession) activateTerminalSession(nextSession);
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
      theme: { ...currentTerminalTheme.theme },
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
    if (!runtime.host) return;
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
      theme: { ...currentTerminalTheme.theme },
      windowsPty: { backend: 'conpty' }
    });
    runtime.terminal = paneTerminal;
    runtime.fitAddon = new FitAddon();
    runtime.searchAddon = new SearchAddon();
    paneTerminal.loadAddon(runtime.fitAddon);
    paneTerminal.loadAddon(runtime.searchAddon);
    paneTerminal.onData((data) => {
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
      void resizeTerminalToFitForRuntime(runtime);
      scheduleFitForRuntime(runtime);
    });
  }

  function handleTerminalKeyForRuntime(runtime: TerminalPaneRuntime, event: KeyboardEvent) {
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
      activePaneId = runtime.paneId;
      terminalPanes = terminalPanes.map((pane) => ({ ...pane, focused: pane.paneId === activePaneId }));
      setActiveRuntime(runtime);
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
      if (event.type === 'keydown' && event.ctrlKey && event.key.toLowerCase() === 'v') { void pasteClipboard(); return false; }
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
    if (!runtime.commandState) return false;
    const marker = parseTerminalShellSequence(data);
    if (!marker) return false;
    const line = runtime.terminal ? runtime.terminal.buffer.active.baseY + runtime.terminal.buffer.active.cursorY : undefined;
    runtime.commandState = reduceTerminalShellMarker(runtime.commandState, marker, line);
    runtime.selectedCommandIndex = runtime.commandState.records.length - 1;
    if (marker.kind === 'cwd' && marker.cwd) {
      runtime.shellCwdMarkerSeen = true;
      applyAuthoritativeTerminalCwdForRuntime(runtime, marker.cwd);
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
    return true;
  }

  function applyAuthoritativeTerminalCwdForRuntime(runtime: TerminalPaneRuntime, cwd: string) {
    if (!cwd) return;
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

  function trackTerminalInputForRuntime(runtime: TerminalPaneRuntime, data: string) {
    runtime.currentInputSelectionActive = false;
    for (const ch of data) {
      if (ch === '\r' || ch === '\n' || ch === '\u0003') {
        if ((ch === '\r' || ch === '\n') && runtime.commandState && runtime.currentInputText.trim()) {
          const line = runtime.terminal ? runtime.terminal.buffer.active.baseY + runtime.terminal.buffer.active.cursorY : undefined;
          runtime.commandState = beginTerminalCommandRecord(runtime.commandState, runtime.currentInputText.trim(), line);
          runtime.selectedCommandIndex = runtime.commandState.records.length - 1;
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
        if ((ch === '\r' || ch === '\n') && commandState && currentInputText.trim()) {
          const line = terminal ? terminal.buffer.active.baseY + terminal.buffer.active.cursorY : undefined;
          commandState = beginTerminalCommandRecord(commandState, currentInputText.trim(), line);
          selectedCommandIndex = commandState.records.length - 1;
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
    if (!runtime.terminal || runtime.currentInputText.length === 0) return;
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
    if (!runtime) return;
    stopPollingForRuntime(runtime);
    clearStartupTimerForRuntime(runtime);
    disposePaneRuntime(runtime);
    paneRuntimes.delete(paneId);
    paneRuntimes = new Map(paneRuntimes);
    terminalPanes = terminalPanes.filter((pane) => pane.paneId !== paneId);
    if (!keepSessionTab) {
      terminalSessions = terminalSessions.filter((item) => item.sessionId !== runtime.session.sessionId);
    }
    if (stopBackendSession) {
      void stopStackTerminal(runtime.session.sessionId).catch((error) => console.debug('Persistent terminal orphan cleanup unavailable', error));
    }
  }

  function disposePaneRuntime(runtime: TerminalPaneRuntime) {
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
    visibleResizeSettled = false;
    for (const runtime of paneRuntimes.values()) {
      runtime.visibleResizeSettled = false;
      ensureTerminalViewForPane(runtime);
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
    runtime.resizeFrame = window.requestAnimationFrame(() => {
      runtime.resizeFrame = null;
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
    await Promise.all(Array.from(paneRuntimes.values()).map((runtime) => resizeTerminalToFitForRuntime(runtime).catch((error) => console.debug('Persistent terminal pane resize unavailable', error))));
  }

  async function resizeTerminalToFitForRuntime(runtime: TerminalPaneRuntime) {
    if (!runtime.terminal || !runtime.fitAddon || !runtime.session) return;
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
      runtime.visibleResizeSettled = true;
      commitRuntime(runtime);
    } catch (error) {
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
    if (runtime.visibleResizeSettled) return;
    if (runtime.visibleResizePromise) { await runtime.visibleResizePromise; return; }
    runtime.visibleResizePromise = resizeTerminalToFitForRuntime(runtime)
      .catch((error) => console.debug('Persistent terminal pane input proceeding without visible resize', error))
      .finally(() => { runtime.visibleResizePromise = null; commitRuntime(runtime); });
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
    if (!runtime.session || !data) return;
    const sessionId = runtime.session.sessionId;
    await ensureVisibleResizeBeforeInputForRuntime(runtime);
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
    const queued = runtime.writeQueue.then(operation, operation);
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
    const queued = runtime.operationQueue.then(operation, operation);
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
    return session?.cwd || commandState?.cwd || selectedCommand()?.cwd || '';
  }

  function buildTerminalActionState(): TerminalActionState {
    const record = selectedCommand();
    const cwd = effectiveTerminalCwd();
    return {
      hasTerminal: Boolean(terminal),
      hasSession: Boolean(session),
      hasSelection: Boolean(terminal?.hasSelection()),
      hasCommand: Boolean(record?.commandText),
      hasCommandOutput: Boolean(record && commandRecordHasOutput(record)),
      hasCwd: Boolean(cwd),
      hasDetectedTarget: quickSelectTargets.length > 0,
      hasRepo: Boolean(cwd),
      canCreateSession: true,
      canSplit: terminalPanes.length < 2 || splitOrientation !== 'single',
      hasMultiplePanes: terminalPanes.length > 1
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

  async function closeTerminalSessionTab(sessionId: string) {
    const runtime = runtimeForSession(sessionId);
    if (runtime) {
      activatePane(runtime.paneId);
      await stopTerminal();
      return;
    }
    await stopStackTerminal(sessionId).catch((error) => console.debug('Persistent terminal tab close unavailable', error));
    terminalSessions = terminalSessions.filter((item) => item.sessionId !== sessionId);
    sessionReplayBuffers.delete(sessionId);
    sessionReplayBuffers = new Map(sessionReplayBuffers);
    renderedSequenceKeysBySession.delete(sessionId);
    renderedSequenceKeysBySession = new Map(renderedSequenceKeysBySession);
  }

  async function stopTerminal() {
    const runtime = activeRuntime();
    if (!runtime) return;
    const sessionId = runtime.session.sessionId;
    stopPollingForRuntime(runtime);
    await stopStackTerminal(sessionId).catch((error) => console.debug('Persistent terminal stop unavailable', error));
    runtime.lifecycle = 'exited';
    runtime.status = 'Terminal stopped';
    runtime.session = { ...runtime.session, running: false };
    disposePaneRuntime(runtime);
    paneRuntimes.delete(runtime.paneId);
    paneRuntimes = new Map(paneRuntimes);
    terminalPanes = terminalPanes.filter((pane) => pane.paneId !== runtime.paneId);
    terminalSessions = terminalSessions.filter((item) => item.sessionId !== sessionId);
    if (!terminalPanes.length) {
      splitOrientation = 'single';
      const nextSession = terminalSessions.find((item) => item.running) ?? terminalSessions[0] ?? null;
      if (nextSession) {
        openSessionAsTab(nextSession);
      } else {
        session = null;
        setActiveRuntime(null);
      }
      return;
    }
    activePaneId = terminalPanes[0].paneId;
    terminalPanes = terminalPanes.map((pane) => ({ ...pane, focused: pane.paneId === activePaneId }));
    splitOrientation = terminalPanes.length > 1 ? splitOrientation : 'single';
    setActiveRuntime(activeRuntime());
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
    runtime.pollTimer = window.setInterval(() => { void pollTerminalOutputForRuntime(runtime); }, 1000);
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
    if (!runtime.session) { stopPollingForRuntime(runtime); return; }
    if (runtime.pollInFlight) { runtime.pollQueued = true; commitRuntime(runtime); return; }
    runtime.pollInFlight = true;
    commitRuntime(runtime);
    try {
      const sessionId = runtime.session.sessionId;
      const result = await enqueueTerminalOperationForRuntime(runtime, () => readStackTerminal(sessionId));
      if (result.sessionId !== runtime.session.sessionId) return;
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
      stopPollingForRuntime(runtime);
      runtime.lifecycle = runtime.outputReceived ? 'exited' : 'failed';
      runtime.status = errorMessage(error, 'Terminal output unavailable');
      commitRuntime(runtime);
      console.error('Failed to poll persistent terminal pane', error);
    } finally {
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

  function replayTerminalSessionOutput(runtime: TerminalPaneRuntime) {
    if (runtime.replayedSessionOutput || !runtime.terminal) return;
    const replay = sessionReplayBuffers.get(runtime.session.sessionId) ?? '';
    if (!replay) return;
    runtime.replayedSessionOutput = true;
    if (terminalOutputHasVisibleText(replay)) {
      runtime.outputReceived = true;
      runtime.lifecycle = 'running';
      runtime.status = '';
      clearStartupTimerForRuntime(runtime);
    }
    runtime.terminal.write(replay);
    commitRuntime(runtime);
  }

  function writeTerminalChunkForRuntime(runtime: TerminalPaneRuntime, chunk: StackTerminalOutputChunk) {
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
    if (output && terminalOutputHasVisibleText(output)) {
      runtime.outputReceived = true;
      runtime.lifecycle = 'running';
      runtime.status = '';
      runtime.session = { ...runtime.session, lastOutputAt: Date.now(), running: true };
      clearStartupTimerForRuntime(runtime);
    }
    if (output) {
      appendSessionReplayBuffer(runtime.session.sessionId, output);
      runtime.recentOutputText = `${runtime.recentOutputText}${stripTerminalAnsiControls(output)}`.slice(-20000);
    }
    runtime.terminal?.write(output);
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
    if (!runtime) return;
    runtime.host = node;
    commitRuntime(runtime);
    ensureTerminalViewForPane(runtime);
    replayTerminalSessionOutput(runtime);
    scheduleFitForRuntime(runtime);
  }

  function bindPaneHost(node: HTMLDivElement, pane: TerminalPaneModel) {
    let boundPane = pane;
    attachPaneHost(node, boundPane);
    return {
      update(nextPane: TerminalPaneModel) {
        const previousPane = boundPane;
        boundPane = nextPane;
        if (previousPane.paneId !== nextPane.paneId || previousPane.sessionId !== nextPane.sessionId) {
          const previousRuntime = paneRuntimes.get(previousPane.paneId);
          if (previousRuntime?.host === node) {
            previousRuntime.host = null;
            commitRuntime(previousRuntime);
          }
        }
        attachPaneHost(node, boundPane);
      },
      destroy() {
        const latest = paneRuntimes.get(boundPane.paneId);
        if (latest?.host === node) {
          latest.host = null;
          commitRuntime(latest);
        }
      }
    };
  }

  function focusTerminal() {
    void tick().then(() => {
      ensureTerminalView();
      terminal?.focus();
    });
  }

  function startStartupTimerForRuntime(runtime: TerminalPaneRuntime) {
    clearStartupTimerForRuntime(runtime);
    runtime.startupTimer = window.setTimeout(() => {
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
    const paneId = runtime.paneId;
    const oldTitle = terminalPanes.find((pane) => pane.paneId === paneId)?.title;
    stopPollingForRuntime(runtime);
    clearStartupTimerForRuntime(runtime);
    await stopStackTerminal(oldSession).catch(() => undefined);
    disposePaneRuntime(runtime);
    const nextSession = await startPersistentTerminal();
    const nextRuntime = createPaneRuntime({ ...nextSession, title: oldTitle || nextSession.title }, paneId);
    nextRuntime.host = runtime.host;
    paneRuntimes = new Map(paneRuntimes).set(paneId, nextRuntime);
    terminalPanes = terminalPanes.map((pane) => pane.paneId === paneId ? { ...pane, sessionId: nextSession.sessionId, title: oldTitle || nextSession.title || pane.title } : pane);
    terminalSessions = terminalSessions.filter((item) => item.sessionId !== oldSession).concat(nextSession);
    activePaneId = paneId;
    setActiveRuntime(nextRuntime);
    ensureTerminalViewForPane(nextRuntime);
    startStartupTimerForRuntime(nextRuntime);
    startPollingForRuntime(nextRuntime);
    void pollTerminalOutputForRuntime(nextRuntime);
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

<div class="terminal-panel" role="dialog" aria-label="Persistent terminal">
  <header class="terminal-panel-header">
    <div class="terminal-session-tabs" role="tablist" aria-label="Terminal sessions">
      {#each terminalSessions as terminalSession, index}
        <div class="terminal-tab-shell" class:active={terminalSession.sessionId === session?.sessionId} role="presentation">
          <button
            type="button"
            class="terminal-tab-button"
            role="tab"
            aria-selected={terminalSession.sessionId === session?.sessionId}
            aria-label={`Switch to terminal session ${terminalSession.title || index + 1}`}
            on:click={() => activateTerminalSession(terminalSession)}
            title={terminalSession.cwd}
          >
            <span>{terminalSession.title || `Session ${index + 1}`}</span>
          </button>
          <small class="terminal-tab-status" aria-hidden="true">{terminalSession.running ? '●' : '○'}</small>
          <button
            type="button"
            class="terminal-tab-close"
            aria-label={`Close terminal session ${terminalSession.title || index + 1}`}
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
      <button type="button" title="Split terminal pane" aria-label="Split terminal pane" disabled={!actionEnabled('splitVertical')} on:click={() => void runTerminalAction('splitVertical')}>Split</button>
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
    <div class="terminal-pane-grid" data-split-orientation={splitOrientation}>
      {#each terminalPanes as pane (pane.sessionId)}
        {@const paneRuntime = paneRuntimes.get(pane.paneId)}
        <section class="terminal-pane" class:focused={pane.focused} data-pane-id={pane.paneId} aria-label={`Terminal pane ${pane.title}`}>
          <div class="terminal-pane-chrome">
            <span>{pane.title}</span>
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
      {/each}
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
