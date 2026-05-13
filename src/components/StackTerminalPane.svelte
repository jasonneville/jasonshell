<script lang="ts">
  import './StackTerminalPane.css';
  import '@xterm/xterm/css/xterm.css';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';
  import { FitAddon } from '@xterm/addon-fit';
  import { SearchAddon } from '@xterm/addon-search';
  import { SerializeAddon } from '@xterm/addon-serialize';
  import { Unicode11Addon } from '@xterm/addon-unicode11';
  import { WebLinksAddon } from '@xterm/addon-web-links';
  import { WebglAddon } from '@xterm/addon-webgl';
  import { Terminal } from '@xterm/xterm';
  import { onMount, tick } from 'svelte';
  import {
    readStackTerminal,
    resizeStackTerminal,
    startStackTerminal,
    stopStackTerminal,
    writeStackTerminal,
    type StackTerminalOutputChunk,
    type StackTerminalProfile,
    type StackTerminalSession
  } from '../lib/stackPopup';
  import { detectStackTerminalLinks, isSafeStackTerminalOpenTarget } from '../features/stack-browser/terminalViewModel';

  type TerminalLifecycleState = 'idle' | 'starting' | 'runningWaitingForFirstByte' | 'running' | 'exited' | 'failed';
  type StackTerminalClosedPayload = { sessionId: string; running?: boolean };
  type StackTerminalOutputPayload = { sessionId: string; text: string; sequence: number; stream?: 'stdout' | 'stderr' | 'system' };
  type StackTerminalRenderer = 'webgl' | 'default' | 'fallback';

  export let currentPath = '';
  export let profile: StackTerminalProfile = 'windowsTerminal';
  export let profileLabel = 'PowerShell';
  export let onCwdChange: (cwd: string) => void | Promise<void> = () => undefined;
  export let onCloseRequest: () => void | Promise<void> = () => undefined;
  export let onError: (message: string) => void = () => undefined;

  const STACK_TERMINAL_REPLAY_LIMIT = 256 * 1024;
  const STACK_TERMINAL_FONT_FAMILY = '"Cascadia Mono", "Cascadia Code", Consolas, ui-monospace, "SFMono-Regular", monospace';

  let host: HTMLDivElement | null = null;
  let searchInput: HTMLInputElement | null = null;
  let terminal: Terminal | null = null;
  let fitAddon: FitAddon | null = null;
  let searchAddon: SearchAddon | null = null;
  let serializeAddon: SerializeAddon | null = null;
  let webglAddon: WebglAddon | null = null;
  let resizeObserver: ResizeObserver | null = null;
  let resizeFrame: number | null = null;
  let flushFrame: number | null = null;
  let startupTimeout: number | null = null;
  let pollTimer: number | null = null;
  let operationQueue: Promise<void> = Promise.resolve();
  let writeQueue: Promise<void> = Promise.resolve();
  let pollInFlight = false;
  let pollQueued = false;
  let listenersDisposed = false;
  let unlisteners: Array<() => void> = [];
  let session: StackTerminalSession | null = null;
  let output = '';
  let cwd = '';
  let busy = false;
  let lifecycleState: TerminalLifecycleState = 'idle';
  let firstOutputReceived = false;
  let firstByteAt = 0;
  let startupStatus = '';
  let lastFitError = '';
  let lastFitRetry = 0;
  let lastResizeKey = '';
  let pendingChunks: Array<StackTerminalOutputPayload | StackTerminalOutputChunk> = [];
  let renderedSequences = new Set<string>();
  let searchOpen = false;
  let searchDraft = '';
  let renderer: StackTerminalRenderer = 'default';
  let contextMenu: { x: number; y: number } | null = null;

  $: detectedLinks = detectStackTerminalLinks(output.slice(-12000), cwd);

  onMount(() => {
    listenersDisposed = false;
    initializeEventListeners();
    document.addEventListener('click', closeTerminalContextMenu);
    return () => {
      listenersDisposed = true;
      document.removeEventListener('click', closeTerminalContextMenu);
      stopPolling();
      clearStartupTimeout();
      cancelOutputFlush();
      disposeTerminalView();
      if (session) {
        void stopStackTerminal(session.sessionId).catch(() => undefined);
      }
      for (const unlisten of unlisteners) {
        unlisten();
      }
      unlisteners = [];
    };
  });

  export async function startTerminal(focusAfterStart = false) {
    if (!currentPath || busy) {
      return;
    }
    if (session && cwd === currentPath) {
      startPolling();
      if (focusAfterStart) {
        focusTerminal();
      }
      return;
    }

    await stopTerminal();
    cwd = currentPath;
    output = '';
    firstOutputReceived = false;
    firstByteAt = 0;
    pendingChunks = [];
    renderedSequences = new Set<string>();
    lifecycleState = 'starting';
    startupStatus = `Starting ${profileLabel}...`;
    busy = true;
    resetTerminalView();
    startStartupTimeout();
    try {
      session = await startStackTerminal(currentPath, profile);
      cwd = session.cwd || currentPath;
      lifecycleState = 'runningWaitingForFirstByte';
      startupStatus = `Starting ${profileLabel}...`;
      resetTerminalView();
      startPolling();
      void scheduleFitAfterOpen();
    } catch (error) {
      const message = operationErrorMessage(error, 'Embedded terminal unavailable');
      lifecycleState = 'failed';
      startupStatus = message;
      onError(message);
      clearStartupTimeout();
      resetTerminalView();
    } finally {
      busy = false;
      if (focusAfterStart) {
        focusTerminal();
      }
    }
  }

  export async function stopTerminal() {
    stopPolling();
    clearStartupTimeout();
    const sessionId = session?.sessionId;
    session = null;
    operationQueue = Promise.resolve();
    lifecycleState = 'idle';
    firstOutputReceived = false;
    firstByteAt = 0;
    startupStatus = '';
    pollQueued = false;
    pendingChunks = [];
    cancelOutputFlush();
    if (sessionId) {
      await stopStackTerminal(sessionId).catch(() => undefined);
    }
  }

  export async function syncFolderToTerminalCwd() {
    await pollOutput();
    if (cwd && cwd !== currentPath) {
      await onCwdChange(cwd);
    }
  }

  export function focusTerminal() {
    void tick().then(() => {
      ensureTerminalView();
      terminal?.focus();
    });
  }

  function initializeEventListeners() {
    const register = (promise: Promise<UnlistenFn>) => {
      void promise.then((unlisten) => {
        if (listenersDisposed) {
          unlisten();
          return;
        }
        unlisteners.push(unlisten);
      }).catch((error) => {
        console.error('Failed to initialize Stack terminal listener', error);
      });
    };

    register(listen<StackTerminalOutputPayload>('stack-terminal:output', (event) => {
      if (event.payload.sessionId !== session?.sessionId) {
        return;
      }
      enqueueOutputChunk(event.payload);
    }));

    register(listen<StackTerminalClosedPayload>('stack-terminal:closed', (event) => {
      if (event.payload.sessionId !== session?.sessionId) {
        return;
      }
      stopPolling();
      clearStartupTimeout();
      lifecycleState = firstOutputReceived ? 'exited' : 'failed';
      startupStatus = firstOutputReceived ? 'Terminal exited' : 'Terminal exited before output';
    }));
  }

  function ensureTerminalView() {
    if (!host) {
      return;
    }
    if (terminal) {
      if (terminal.element && !host.contains(terminal.element)) {
        disposeTerminalView();
      } else {
        terminal.options.disableStdin = !session || busy;
      }
    }
    if (terminal) {
      fitTerminal();
      return;
    }

    const nextTerminal = new Terminal({
      allowProposedApi: false,
      convertEol: false,
      cursorBlink: true,
      cursorStyle: 'block',
      disableStdin: !session || busy,
      fontFamily: STACK_TERMINAL_FONT_FAMILY,
      fontSize: 13,
      letterSpacing: 0,
      lineHeight: 1.25,
      screenReaderMode: false,
      scrollback: 5000,
      windowsPty: { backend: 'conpty' },
      theme: {
        background: '#06080b',
        foreground: '#d7e2f5',
        cursor: '#d7e2f5',
        cursorAccent: '#06080b',
        selectionBackground: '#315f8c'
      }
    });
    const nextFitAddon = new FitAddon();
    const nextSearchAddon = new SearchAddon();
    const nextSerializeAddon = new SerializeAddon();
    const unicodeAddon = new Unicode11Addon();
    nextTerminal.loadAddon(nextFitAddon);
    nextTerminal.loadAddon(nextSearchAddon);
    nextTerminal.loadAddon(nextSerializeAddon);
    nextTerminal.loadAddon(unicodeAddon);
    nextTerminal.unicode.activeVersion = '11';
    nextTerminal.loadAddon(new WebLinksAddon());
    nextTerminal.registerLinkProvider({
      provideLinks: (bufferLineNumber, callback) => {
        const bufferLine = nextTerminal.buffer.active.getLine(bufferLineNumber);
        const lineText = bufferLine?.translateToString(true) ?? '';
        callback(detectStackTerminalLinks(lineText, cwd).filter(isSafeStackTerminalOpenTarget).map((link) => ({
          text: link.text,
          range: {
            start: { x: link.index + 1, y: bufferLineNumber },
            end: { x: link.index + link.text.length + 1, y: bufferLineNumber }
          },
          activate: () => console.debug('Stack terminal link selected', link.kind, link.target)
        })));
      }
    });
    nextTerminal.onData((data) => {
      void writeTerminalData(data);
    });
    nextTerminal.attachCustomKeyEventHandler((event) => {
      if (event.type === 'keydown' && event.ctrlKey && event.key.toLowerCase() === 'f') {
        openSearch();
        return false;
      }
      if (event.type === 'keydown' && event.ctrlKey && event.key.toLowerCase() === 'v') {
        event.preventDefault();
        event.stopPropagation();
        void pasteClipboard();
        return false;
      }
      if (event.type === 'keydown' && event.key === 'Escape' && searchOpen) {
        closeSearch();
        return false;
      }
      if (event.type === 'keydown' && event.ctrlKey && event.shiftKey && event.key.toLowerCase() === 'w') {
        void onCloseRequest();
        return false;
      }
      if (event.type === 'keydown' && event.ctrlKey && event.key.toLowerCase() === 'c' && nextTerminal.hasSelection()) {
        void copySelection();
        return false;
      }
      return true;
    });
    nextTerminal.open(host);
    terminal = nextTerminal;
    fitAddon = nextFitAddon;
    searchAddon = nextSearchAddon;
    serializeAddon = nextSerializeAddon;
    loadWebglAddon();
    resizeObserver = new ResizeObserver(() => scheduleFit());
    resizeObserver.observe(host);
    if (output) {
      nextTerminal.write(output);
    }
    void scheduleFitAfterOpen();
  }

  function loadWebglAddon() {
    if (!terminal) {
      return;
    }
    try {
      const addon = new WebglAddon();
      addon.onContextLoss(() => {
        addon.dispose();
        if (webglAddon === addon) {
          webglAddon = null;
        }
        renderer = 'fallback';
      });
      terminal.loadAddon(addon);
      webglAddon = addon;
      renderer = 'webgl';
    } catch (error) {
      console.debug('Stack terminal WebGL renderer unavailable', error);
      renderer = 'fallback';
    }
  }

  async function scheduleFitAfterOpen() {
    await tick();
    fitTerminal();
    scheduleFit();
  }

  function scheduleFit() {
    if (resizeFrame !== null) {
      window.cancelAnimationFrame(resizeFrame);
    }
    resizeFrame = window.requestAnimationFrame(() => {
      resizeFrame = null;
      fitTerminal();
    });
  }

  function fitTerminal() {
    try {
      if (!fitAddon) {
        return;
      }
      fitAddon.fit();
      lastFitError = '';
      if (session && terminal && terminal.cols > 0 && terminal.rows > 0) {
        const sessionId = session.sessionId;
        const cols = terminal.cols;
        const rows = terminal.rows;
        const pixelWidth = host?.clientWidth ?? 0;
        const pixelHeight = host?.clientHeight ?? 0;
        const resizeKey = `${sessionId}:${cols}:${rows}:${pixelWidth}:${pixelHeight}`;
        if (resizeKey === lastResizeKey) {
          return;
        }
        lastResizeKey = resizeKey;
        void enqueueOperation(() => resizeStackTerminal(sessionId, cols, rows, pixelWidth, pixelHeight)).catch((error) => {
          lastFitError = operationErrorMessage(error, 'Terminal resize unavailable');
          lastResizeKey = '';
        });
      } else if (terminal && lastFitRetry < 2) {
        lastFitRetry += 1;
        window.requestAnimationFrame(() => fitTerminal());
      }
    } catch (error) {
      lastFitError = operationErrorMessage(error, 'Terminal fit unavailable');
    }
  }

  function resetTerminalView(outputSnapshot = '') {
    renderedSequences = new Set<string>();
    pendingChunks = [];
    cancelOutputFlush();
    void tick().then(() => {
      ensureTerminalView();
      terminal?.reset();
      if (outputSnapshot) {
        terminal?.write(outputSnapshot);
      }
      terminal?.scrollToBottom();
    });
  }

  function enqueueOutputChunk(chunk: StackTerminalOutputPayload | StackTerminalOutputChunk) {
    if (chunk.sessionId !== session?.sessionId) {
      return;
    }
    pendingChunks.push(chunk);
    if (flushFrame !== null) {
      return;
    }
    flushFrame = window.requestAnimationFrame(() => {
      flushFrame = null;
      flushOutputChunks();
    });
  }

  function flushOutputChunks() {
    const activeSessionId = session?.sessionId;
    if (!activeSessionId || pendingChunks.length === 0) {
      pendingChunks = [];
      return;
    }
    const chunks = pendingChunks
      .filter((chunk) => chunk.sessionId === activeSessionId)
      .sort((left, right) => (left.sequence ?? 0) - (right.sequence ?? 0));
    pendingChunks = [];
    for (const chunk of chunks) {
      writeChunk(chunk);
    }
  }

  function writeChunk(chunk: StackTerminalOutputPayload | StackTerminalOutputChunk) {
    if (typeof chunk.sequence === 'number') {
      const sequenceKey = `${chunk.sessionId}:${chunk.stream ?? 'stdout'}:${chunk.sequence}`;
      if (renderedSequences.has(sequenceKey)) {
        return;
      }
      renderedSequences.add(sequenceKey);
    }
    writeOutput(chunk.text);
  }

  function cancelOutputFlush() {
    if (flushFrame !== null) {
      window.cancelAnimationFrame(flushFrame);
      flushFrame = null;
    }
  }

  function disposeTerminalView() {
    if (resizeFrame !== null) {
      window.cancelAnimationFrame(resizeFrame);
      resizeFrame = null;
    }
    lastResizeKey = '';
    resizeObserver?.disconnect();
    resizeObserver = null;
    webglAddon?.dispose();
    webglAddon = null;
    fitAddon?.dispose();
    fitAddon = null;
    searchAddon = null;
    serializeAddon = null;
    terminal?.dispose();
    terminal = null;
    cancelOutputFlush();
  }

  async function writeTerminalData(data: string) {
    if (!session || !data) {
      return;
    }
    const sessionId = session.sessionId;
    await enqueueTerminalWrite(() => writeStackTerminal(sessionId, data));
  }

  function enqueueTerminalWrite<T>(operation: () => Promise<T>): Promise<T> {
    const queued = writeQueue.then(operation, operation);
    writeQueue = queued.then(() => undefined, () => undefined);
    return queued;
  }

  function enqueueOperation<T>(operation: () => Promise<T>): Promise<T> {
    const queued = operationQueue.then(operation, operation);
    operationQueue = queued.then(() => undefined, () => undefined);
    return queued;
  }

  async function copySelection() {
    const selection = terminal?.getSelection() ?? '';
    if (!selection) {
      return;
    }
    await navigator.clipboard?.writeText(selection).catch((error) => {
      console.debug('Stack terminal selection copy unavailable', error);
    });
  }

  async function pasteClipboard() {
    const text = await navigator.clipboard?.readText().catch((error) => {
      console.debug('Stack terminal clipboard paste unavailable', error);
      return '';
    });
    if (text) {
      await writeTerminalData(text);
    }
  }

  function openTerminalContextMenu(event: MouseEvent) {
    event.preventDefault();
    contextMenu = { x: event.clientX, y: event.clientY };
    terminal?.focus();
  }

  function closeTerminalContextMenu() {
    contextMenu = null;
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
    // Watchdog only; push events/prompt output should arrive before this fallback.
    pollTimer = window.setInterval(() => {
      void pollOutput();
    }, 1500);
  }

  function stopPolling() {
    if (pollTimer !== null) {
      window.clearInterval(pollTimer);
      pollTimer = null;
    }
  }

  async function pollOutput() {
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
      const result = await enqueueOperation(() => readStackTerminal(sessionId));
      if (result.sessionId !== session?.sessionId) {
        return;
      }
      if (result.chunks?.length) {
        for (const chunk of result.chunks) {
          enqueueOutputChunk(chunk);
        }
      }
      await applyCwd(result.cwd || cwd);
      if (result.exited) {
        stopPolling();
        clearStartupTimeout();
        lifecycleState = firstOutputReceived ? 'exited' : 'failed';
        startupStatus = result.exitCode === null || result.exitCode === undefined
          ? 'Terminal exited before output'
          : `Terminal exited before output (${result.exitCode})`;
      }
    } catch (error) {
      stopPolling();
      const message = operationErrorMessage(error, 'Embedded terminal unavailable');
      lifecycleState = firstOutputReceived ? 'exited' : 'failed';
      startupStatus = message;
      onError(message);
    } finally {
      pollInFlight = false;
      if (pollQueued) {
        pollQueued = false;
        void pollOutput();
      }
    }
  }

  function writeOutput(nextOutputChunk: string) {
    if (nextOutputChunk && terminalOutputHasVisibleText(nextOutputChunk)) {
      firstOutputReceived = true;
      firstByteAt ||= Date.now();
      lifecycleState = 'running';
      startupStatus = '';
      clearStartupTimeout();
    }
    const nextOutput = output + nextOutputChunk;
    output = compactTerminalOutput(nextOutput);
    terminal?.write(nextOutputChunk);
  }

  async function applyCwd(nextCwd: string) {
    const normalized = nextCwd || cwd;
    if (!normalized) {
      return;
    }
    cwd = normalized;
    if (normalized !== currentPath) {
      await onCwdChange(normalized);
    }
  }

  function openSearch() {
    searchOpen = true;
    void tick().then(() => searchInput?.focus());
  }

  function closeSearch() {
    searchOpen = false;
    searchDraft = '';
    terminal?.focus();
  }

  function runSearch(reverse = false) {
    if (!searchDraft.trim()) {
      return;
    }
    if (reverse) {
      searchAddon?.findPrevious(searchDraft);
    } else {
      searchAddon?.findNext(searchDraft);
    }
  }

  function startStartupTimeout() {
    clearStartupTimeout();
    startupTimeout = window.setTimeout(() => {
      if (!firstOutputReceived && lifecycleState === 'runningWaitingForFirstByte') {
        startupStatus = `Still starting ${profileLabel}...`;
      }
    }, 1200);
  }

  function clearStartupTimeout() {
    if (startupTimeout !== null) {
      window.clearTimeout(startupTimeout);
      startupTimeout = null;
    }
  }

  function terminalOutputHasVisibleText(value: string) {
    return stripTerminalAnsiControls(value).trim().length > 0;
  }

  function stripTerminalAnsiControls(value: string) {
    return value
      .replace(/\x1b\][^\x07]*(?:\x07|\x1b\\)/g, '')
      .replace(/\x1b(?:[@-Z\\-_]|\[[0-?]*[ -/]*[@-~])/g, '')
      .replace(/[\x00-\x08\x0b\x0c\x0e-\x1f\x7f]/g, '');
  }

  function compactTerminalOutput(value: string) {
    const clearPattern = /\x1b(?:c|\[[0-?]*[ -/]*[HJ])/g;
    let lastClearEnd = 0;
    for (const match of value.matchAll(clearPattern)) {
      lastClearEnd = (match.index ?? 0) + match[0].length;
    }
    const compacted = lastClearEnd > 0 ? value.slice(lastClearEnd) : value;
    return compacted.length > STACK_TERMINAL_REPLAY_LIMIT
      ? compacted.slice(compacted.length - STACK_TERMINAL_REPLAY_LIMIT)
      : compacted;
  }

  function copyVisibleTerminal() {
    const snapshot = serializeAddon?.serialize() ?? output;
    void navigator.clipboard?.writeText(snapshot).catch(() => undefined);
  }

  function operationErrorMessage(error: unknown, fallback: string) {
    return error instanceof Error && error.message ? error.message : fallback;
  }
</script>

<section class="stack-terminal" data-renderer={renderer} aria-label="Embedded terminal">
  {#if searchOpen}
    <form class="stack-terminal-search" on:submit|preventDefault={() => runSearch(false)}>
      <input bind:this={searchInput} bind:value={searchDraft} aria-label="Search terminal scrollback" />
      <button type="submit">Next</button>
      <button type="button" on:click={() => runSearch(true)}>Prev</button>
      <button type="button" on:click={closeSearch}>Close</button>
    </form>
  {/if}
  <div class="stack-terminal-output" role="log" bind:this={host} on:contextmenu={openTerminalContextMenu}></div>
  {#if contextMenu}
    <div
      class="stack-terminal-context-menu"
      role="menu"
      tabindex="-1"
      style={`left: ${contextMenu.x}px; top: ${contextMenu.y}px;`}
    >
      <button type="button" role="menuitem" on:click={() => void copySelectionFromContextMenu()}>Copy</button>
      <button type="button" role="menuitem" on:click={() => void pasteClipboardFromContextMenu()}>Paste</button>
    </div>
  {/if}
  {#if !firstOutputReceived && lifecycleState !== 'idle'}
    <div class="stack-terminal-startup" role="status">
      <strong>{startupStatus || `Starting ${profileLabel}...`}</strong>
      <span>{profileLabel}</span>
      <code>{cwd || currentPath}</code>
      {#if lastFitError}
        <small>{lastFitError}</small>
      {/if}
    </div>
  {/if}
  <div class="stack-terminal-diagnostics" aria-hidden="true">
    <span>{renderer}</span>
    <span>{detectedLinks.length}</span>
    <button type="button" tabindex="-1" on:click={copyVisibleTerminal}>Copy visible</button>
  </div>
</section>
