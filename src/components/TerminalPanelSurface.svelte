<script lang="ts">
  import './TerminalPanelSurface.css';
  import '@xterm/xterm/css/xterm.css';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';
  import { FitAddon } from '@xterm/addon-fit';
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

  type TerminalLifecycleState = 'starting' | 'waiting' | 'running' | 'failed' | 'exited';
  type TerminalOutputPayload = StackTerminalOutputChunk;
  type TerminalClosedPayload = {
    sessionId: string;
    running?: boolean;
  };

  const TERMINAL_PANEL_OPEN_EVENT = 'terminal-panel:open';

  const TERMINAL_PANEL_FONT_FAMILY = '"Cascadia Mono", "Cascadia Code", Consolas, ui-monospace, "SFMono-Regular", monospace';

  let host: HTMLDivElement | null = null;
  let terminal: Terminal | null = null;
  let fitAddon: FitAddon | null = null;
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
  let currentInputText = '';
  let currentInputSelectionActive = false;
  let visibleResizeSettled = false;
  let visibleResizePromise: Promise<void> | null = null;

  onMount(() => {
    listenersDisposed = false;
    document.addEventListener('click', closeTerminalContextMenu);
    window.addEventListener('focus', handlePanelOpen);
    void initializeTerminalListeners();
    void startTerminal();
    return () => {
      listenersDisposed = true;
      document.removeEventListener('click', closeTerminalContextMenu);
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
    terminal.loadAddon(fitAddon);
    terminal.onData((data) => {
      trackTerminalInput(data);
      void writeTerminalData(data);
    });
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
      if (event.type === 'keydown' && currentInputSelectionActive && (event.key === 'Backspace' || event.key === 'Delete')) {
        event.preventDefault();
        event.stopPropagation();
        void deleteSelectedCurrentInput();
        return false;
      }
      if (event.type === 'keydown' && event.ctrlKey && event.key.toLowerCase() === 'c' && terminal?.hasSelection()) {
        void copySelection();
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

  function trackTerminalInput(data: string) {
    currentInputSelectionActive = false;
    for (const ch of data) {
      if (ch === '\r' || ch === '\n' || ch === '\u0003') {
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
    fitAddon?.dispose();
    fitAddon = null;
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
    currentInputText = '';
    currentInputSelectionActive = false;
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
    <button type="button" class="terminal-restart" on:click={() => void restartTerminal()}>Restart</button>
  </header>
  <section class="terminal-panel-body">
    <!-- svelte-ignore a11y_no_noninteractive_element_interactions: xterm owns terminal interaction semantics; this handler only narrows triple-click selection. -->
    <div bind:this={host} class="terminal-panel-output" role="log" aria-label="Terminal output" on:mousedown|capture={handleTerminalMouseDown} on:contextmenu={openTerminalContextMenu}></div>
    {#if contextMenu}
      <div
        class="terminal-panel-context-menu"
        role="menu"
        tabindex="-1"
        style={`left: ${contextMenu.x}px; top: ${contextMenu.y}px;`}
      >
        <button type="button" role="menuitem" on:click={() => void copySelectionFromContextMenu()}>Copy</button>
        <button type="button" role="menuitem" on:click={() => void pasteClipboardFromContextMenu()}>Paste</button>
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
