<script lang="ts">
  import './SpeechHistoryPanelSurface.css';
  import { onMount } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import type { SpeechHistoryEntry } from '../ipc/events';
  import {
    copySpeechHistoryTranscript,
    getSpeechHistory,
    hideSpeechHistoryPanel,
    SPEECH_HISTORY_PANEL_CLOSED_EVENT,
    SPEECH_HISTORY_PANEL_OPEN_EVENT
  } from '../lib/speech';

  const contentCopyIconUrl = new URL(
    '../assets/icons/content_copy_24dp_E3E3E3_FILL0_wght300_GRAD0_opsz24.svg',
    import.meta.url
  ).href;

  let entries: SpeechHistoryEntry[] = [];
  let loading = false;
  let loadFailed = false;
  let disposed = false;
  let openSequence = 0;
  let copyingNonce: number | null = null;
  let copyErrorNonce: number | null = null;
  let dialogElement: HTMLDivElement;

  function outcomeLabel(outcome: SpeechHistoryEntry['outcome']) {
    return outcome === 'copied'
      ? 'Copied to clipboard'
      : 'Not copied — clipboard unavailable';
  }

  async function loadHistory() {
    const sequence = ++openSequence;
    loading = true;
    loadFailed = false;
    copyingNonce = null;
    copyErrorNonce = null;
    try {
      const history = await getSpeechHistory();
      if (disposed || sequence !== openSequence) return;
      entries = history.slice(0, 5);
    } catch {
      if (disposed || sequence !== openSequence) return;
      entries = [];
      loadFailed = true;
    } finally {
      if (!disposed && sequence === openSequence) loading = false;
    }
  }

  async function copyTranscript(entry: SpeechHistoryEntry) {
    const sequence = openSequence;
    copyingNonce = entry.nonce;
    copyErrorNonce = null;
    try {
      await copySpeechHistoryTranscript({ nonce: entry.nonce });
      if (disposed || sequence !== openSequence) return;
      entries = entries.map((candidate) => candidate.nonce === entry.nonce
        ? { ...candidate, outcome: 'copied' }
        : candidate);
    } catch {
      if (disposed || sequence !== openSequence) return;
      copyErrorNonce = entry.nonce;
    } finally {
      if (!disposed && sequence === openSequence && copyingNonce === entry.nonce) {
        copyingNonce = null;
      }
    }
  }

  function closePanel() {
    openSequence += 1;
    void hideSpeechHistoryPanel();
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      event.preventDefault();
      event.stopPropagation();
      closePanel();
    }
  }

  function handleFocusLoss() {
    closePanel();
  }

  onMount(() => {
    const unlisteners: Array<() => void> = [];
    disposed = false;

    function registerAsyncUnlistener(registration: Promise<() => void>) {
      void registration.then((unlisten) => {
        if (disposed) unlisten();
        else unlisteners.push(unlisten);
      });
    }

    registerAsyncUnlistener(listen(SPEECH_HISTORY_PANEL_OPEN_EVENT, () => {
      if (!disposed) {
        dialogElement?.focus();
        void loadHistory();
      }
    }));
    registerAsyncUnlistener(listen(SPEECH_HISTORY_PANEL_CLOSED_EVENT, () => {
      openSequence += 1;
      entries = [];
      loading = false;
      loadFailed = false;
      copyingNonce = null;
      copyErrorNonce = null;
    }));
    window.addEventListener('blur', handleFocusLoss);

    return () => {
      disposed = true;
      openSequence += 1;
      window.removeEventListener('blur', handleFocusLoss);
      for (const unlisten of unlisteners) unlisten();
    };
  });
</script>

<svelte:window on:keydown={handleKeydown} />

<div bind:this={dialogElement} class="speech-history-panel" role="dialog" tabindex="-1" aria-labelledby="speech-history-heading">
  <header class="speech-history-header">
    <div>
      <span class="speech-history-kicker">Local session</span>
      <h1 id="speech-history-heading">Speech history</h1>
    </div>
    <button type="button" class="speech-history-close" aria-label="Close speech history" on:click={closePanel}>×</button>
  </header>

  <div class="speech-history-content">
    {#if loading}
      <p class="speech-history-state" role="status">Loading speech history…</p>
    {:else if loadFailed}
      <p class="speech-history-state">Speech history is unavailable.</p>
    {:else if entries.length === 0}
      <p class="speech-history-state">No speech transcriptions this session.</p>
    {:else}
      <ol aria-label="Recent speech transcriptions">
        {#each entries as entry (entry.nonce)}
          <li>
            <div class="speech-history-row-main">
              <p class="speech-history-transcript">{entry.transcript}</p>
              <button
                type="button"
                class="speech-history-copy"
                aria-label="Copy speech transcription"
                title="Copy speech transcription"
                disabled={copyingNonce === entry.nonce}
                on:click={() => copyTranscript(entry)}
              >
                <img src={contentCopyIconUrl} alt="" aria-hidden="true" draggable="false" />
              </button>
            </div>
            <span class:copied={entry.outcome === 'copied'}>{outcomeLabel(entry.outcome)}</span>
            {#if copyErrorNonce === entry.nonce}
              <p class="speech-history-copy-error">Copy failed. Try again.</p>
            {/if}
          </li>
        {/each}
      </ol>
    {/if}
  </div>
</div>
