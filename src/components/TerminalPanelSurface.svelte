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

  type TerminalLifecycleState = 'starting' | 'waiting' | 'running' | 'failed' | 'exited';
  type TerminalOutputPayload = StackTerminalOutputChunk;
  type TerminalClosedPayload = {
    sessionId: string;
    running?: boolean;
  };

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
  let renderedSequences = new Set<string>();
  let lastResizeKey = '';
  let unlisteners: Array<() => void> = [];
  let listenersDisposed = false;

  onMount(() => {
    listenersDisposed = false;
    void initializeTerminalListeners();
    void startTerminal();
    return () => {
      listenersDisposed = true;
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
      listen<TerminalOutputPayload>('stack-terminal:output', (event) => {
        if (event.payload.sessionId !== session?.sessionId) {
          return;
        }
        writeTerminalChunk(event.payload);
        scrollToBottom();
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
      convertEol: true,
      cursorBlink: true,
      fontFamily: 'var(--js-font-sans)',
      fontSize: 12,
      lineHeight: 1.35,
      scrollback: 8000,
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
      void writeTerminalData(data);
    });
    terminal.open(host);
    resizeObserver = new ResizeObserver(() => scheduleFit());
    resizeObserver.observe(host);
    void tick().then(() => {
      fitTerminal();
      scheduleFit();
    });
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
    if (!terminal || !fitAddon || !session) {
      return;
    }
    try {
      fitAddon.fit();
      const width = host?.clientWidth ?? 0;
      const height = host?.clientHeight ?? 0;
      const resizeKey = `${session.sessionId}:${terminal.cols}:${terminal.rows}:${width}:${height}`;
      if (resizeKey === lastResizeKey || terminal.cols <= 0 || terminal.rows <= 0) {
        return;
      }
      lastResizeKey = resizeKey;
      void enqueueTerminalOperation(() =>
        resizeStackTerminal(session!.sessionId, terminal!.cols, terminal!.rows, width, height)
      ).catch((error) => {
        lastResizeKey = '';
        console.error('Failed to resize persistent terminal', error);
      });
    } catch (error) {
      console.debug('Persistent terminal fit unavailable', error);
    }
  }

  async function writeTerminalData(data: string) {
    if (!session || !data) {
      return;
    }
    const sessionId = session.sessionId;
    await enqueueTerminalOperation(() => writeStackTerminal(sessionId, data));
    await pollTerminalOutput();
  }

  function enqueueTerminalOperation<T>(operation: () => Promise<T>): Promise<T> {
    const queued = operationQueue.then(operation, operation);
    operationQueue = queued.then(
      () => undefined,
      () => undefined
    );
    return queued;
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
        scrollToBottom();
      } else if (result.output) {
        writeTerminalOutput(result.output);
        scrollToBottom();
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
      const sequenceKey = `${chunk.sessionId}:${chunk.sequence}`;
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

  function scrollToBottom() {
    void tick().then(() => terminal?.scrollToBottom());
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

<div class="terminal-panel" role="dialog" aria-label="Persistent terminal">
  <header class="terminal-panel-header">
    <div>
      <strong>Terminal</strong>
      <span>{session?.cwd || 'Starting shell'}</span>
    </div>
    <button type="button" class="terminal-restart" on:click={() => void restartTerminal()}>Restart</button>
  </header>
  <section class="terminal-panel-body">
    <div bind:this={host} class="terminal-panel-output" aria-label="Terminal output"></div>
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
