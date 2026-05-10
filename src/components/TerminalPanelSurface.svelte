<script lang="ts">
  import './TerminalPanelSurface.css';
  import '@xterm/xterm/css/xterm.css';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';
  import { FitAddon } from '@xterm/addon-fit';
  import { SearchAddon } from '@xterm/addon-search';
  import { Terminal } from '@xterm/xterm';
  import { onMount, tick } from 'svelte';
  import {
    readStackTerminal,
    resizeStackTerminal,
    startPersistentTerminal,
    stopStackTerminal,
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

  type TerminalLifecycleState = 'starting' | 'waiting' | 'running' | 'failed' | 'exited';
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

  let host: HTMLDivElement | null = null;
  let terminal: Terminal | null = null;
  let fitAddon: FitAddon | null = null;
  let searchAddon: SearchAddon | null = null;
  let resizeObserver: ResizeObserver | null = null;
  let resizeFrame: number | null = null;
  let session: StackTerminalSession | null = null;
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

  let currentInputText = '';
  let currentInputSelectionActive = false;
  let visibleResizeSettled = false;
  let visibleResizePromise: Promise<void> | null = null;
  let commandState: TerminalCommandState | null = null;
  let selectedCommandIndex = -1;
  let shellCwdMarkerSeen = false;
  let shellParserDisposers: Array<() => void> = [];

  $: terminalActionState = buildTerminalActionState();
  $: visibleRecentCommands = recentTerminalCommands(commandState);
  $: visibleRecentDirectories = recentTerminalDirectories(commandState);
  $: toolbarActions = terminalActions.filter((action) => ['search', 'openCwdInFiles', 'openExternalTerminalHere', 'restartTerminal', 'stopTerminal'].includes(action.id));

  onMount(() => {
    listenersDisposed = false;
    document.addEventListener('pointerdown', closeTerminalMenusOnOutsidePointer, true);
    window.addEventListener('focus', handlePanelOpen);
    void initializeTerminalListeners();
    void startTerminal();
    return () => {
      listenersDisposed = true;
      document.removeEventListener('pointerdown', closeTerminalMenusOnOutsidePointer, true);
      window.removeEventListener('focus', handlePanelOpen);
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
        handlePanelOpen();
      })
    );

    register(
      listen<TerminalOutputPayload>('stack-terminal:output', (event) => {
        if (event.payload.sessionId !== session?.sessionId) {
          return;
        }
        writeTerminalChunk(event.payload);
      })
    );

    register(
      listen<TerminalClosedPayload>('stack-terminal:closed', (event) => {
        if (event.payload.sessionId !== session?.sessionId) {
          return;
        }
        stopPolling();
        clearStartupTimer();
        lifecycle = outputReceived ? 'exited' : 'failed';
        status = outputReceived ? 'Terminal exited' : 'Terminal exited before output';
      })
    );

    register(
      listen<TerminalCwdPayload>('stack-terminal:cwd', (event) => {
        if (event.payload.sessionId !== session?.sessionId || !event.payload.cwd) {
          return;
        }
        if (!shellCwdMarkerSeen) {
          applyAuthoritativeTerminalCwd(event.payload.cwd);
        }
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
      session = await startPersistentTerminal();
      commandState = createTerminalCommandState(session.sessionId, session.cwd);
      selectedCommandIndex = -1;
      shellCwdMarkerSeen = false;
      lifecycle = 'waiting';
      status = 'Waiting for terminal output...';
      handlePanelOpen();
      startPolling();
      void pollTerminalOutput();
      focusTerminal();
    } catch (error) {
      clearStartupTimer();
      lifecycle = 'failed';
      status = errorMessage(error, 'Terminal failed to start');
      console.error('Failed to start persistent terminal', error);
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

  function ensureTerminalView() {
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
      cursorStyle: 'block',
      fontFamily: TERMINAL_PANEL_FONT_FAMILY,
      fontSize: 13,
      letterSpacing: 0,
      lineHeight: 1.25,
      screenReaderMode: false,
      scrollback: 8000,
      windowsPty: { backend: 'conpty' },
      theme: {
        background: '#05070a',
        foreground: '#d7e2f5',
        cursor: '#ffffff',
        cursorAccent: '#05070a',
        selectionBackground: '#315f8c'
      }
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

  function applyAuthoritativeTerminalCwd(cwd: string) {
    if (!session || !cwd) {
      return;
    }
    session = { ...session, cwd };
    if (commandState) {
      commandState = { ...commandState, cwd };
    }
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
  }

  function handlePanelOpen() {
    visibleResizeSettled = false;
    void scheduleFitAfterPanelOpen();
  }

  async function scheduleFitAfterPanelOpen() {
    await tick();
    focusTerminal();
    visibleResizePromise = resizeTerminalToFit()
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
    void resizeTerminalToFit();
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

  async function writeTerminalData(data: string) {
    if (!session || !data) {
      return;
    }
    const sessionId = session.sessionId;
    await ensureVisibleResizeBeforeInput();
    await enqueueTerminalWrite(() => writeStackTerminal(sessionId, data));
  }

  function enqueueTerminalWrite<T>(operation: () => Promise<T>): Promise<T> {
    const queued = writeQueue.then(operation, operation);
    writeQueue = queued.then(
      () => undefined,
      () => undefined
    );
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
      hasRepo: Boolean(cwd)
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

  async function stopTerminal() {
    if (!session) return;
    const sessionId = session.sessionId;
    stopPolling();
    await stopStackTerminal(sessionId).catch((error) => console.debug('Persistent terminal stop unavailable', error));
    lifecycle = 'exited';
    status = 'Terminal stopped';
    session = null;
  }

  async function copySelectionFromContextMenu() {
    closeTerminalContextMenu();
    await copySelection();
  }

  async function pasteClipboardFromContextMenu() {
    closeTerminalContextMenu();
    await pasteClipboard();
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

  function focusTerminal() {
    void tick().then(() => {
      ensureTerminalView();
      terminal?.focus();
    });
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
    const oldSession = session?.sessionId;
    session = null;
    commandState = null;
    selectedCommandIndex = -1;
    shellCwdMarkerSeen = false;
    currentInputText = '';
    currentInputSelectionActive = false;
    recentOutputText = '';

    renderedSequences = new Set<string>();
    if (oldSession) {
      await stopStackTerminal(oldSession).catch(() => undefined);
    }
    disposeTerminalView();
    await startTerminal();
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
    <div>
      <strong>Terminal</strong>
      <span>{session?.cwd || 'Starting shell'}</span>
    </div>
    <nav class="terminal-toolbar" aria-label="Terminal actions">
      <button type="button" title="Search" disabled={!actionEnabled('search')} on:click={() => void runTerminalAction('search')}>⌕</button>
      <button type="button" title="Quick Select" on:click={toggleQuickSelect}>QS</button>
      <button type="button" title="Recent commands" on:click={() => { recentMenuOpen = !recentMenuOpen; actionMenuOpen = false; }}>⌘</button>
      <button type="button" title="Reveal cwd in Files" disabled={!actionEnabled('openCwdInFiles')} on:click={() => void runTerminalAction('openCwdInFiles')}>Files</button>
      <button type="button" title="Open external terminal here" disabled={!actionEnabled('openExternalTerminalHere')} on:click={() => void runTerminalAction('openExternalTerminalHere')}>Ext</button>
      <button type="button" title="Terminal menu" on:click={() => { actionMenuOpen = !actionMenuOpen; recentMenuOpen = false; }}>⋯</button>
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
  <section class="terminal-panel-body">
    <!-- svelte-ignore a11y_no_noninteractive_element_interactions: xterm owns terminal interaction semantics; this handler only narrows triple-click selection. -->
    <div bind:this={host} class="terminal-panel-output" role="log" aria-label="Terminal output" on:mousedown|capture={handleTerminalMouseDown} on:contextmenu={openTerminalContextMenu}></div>
    {#if searchOpen}
      <form class="terminal-search" on:pointerdown|stopPropagation on:submit|preventDefault={searchNext}>
        <input class="terminal-search-input" bind:value={searchQuery} placeholder="Search" aria-label="Search terminal" />
        <button type="submit">Next</button>
        <button type="button" on:click={searchPrevious}>Prev</button>
        <button type="button" on:click={closeSearch}>Esc</button>
      </form>
    {/if}
    {#if quickSelectOpen}
      <div class="terminal-quick-select" role="listbox" tabindex="-1" aria-label="Quick Select targets" on:pointerdown|stopPropagation>
        {#each quickSelectTargets as target}
          <button type="button" on:click={() => void openQuickSelectTarget(target)}><kbd>{target.label}</kbd>{target.text}</button>
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
    {#if !outputReceived && lifecycle !== 'running'}
      <div class="terminal-panel-status" class:error={lifecycle === 'failed'} role={lifecycle === 'failed' ? 'alert' : 'status'} aria-live="polite">
        <strong>{status}</strong>
        {#if lifecycle === 'failed'}
          <span>Restart terminal or check PowerShell startup.</span>
        {:else}
          <span>JasonShell terminal starts with the app and stays alive while hidden.</span>
        {/if}
      </div>
    {/if}
  </section>
</div>
