<script lang="ts">
  import { onMount } from 'svelte';
  import {
    INITIAL_MIC_CONTROL_MODEL,
    MIC_CONTROL_PRESENTATION,
    acceptSpeechStart,
    acceptSpeechStop,
    beginSpeechStart,
    beginSpeechStop,
    failSpeechCommand,
    normalizeSpeechCommandError,
    reduceSpeechEvent,
    settleSpeechCommand,
    speechControlLabel,
    type MicControlModel
  } from '../features/top-bar/micControlState';
  import {
    listenSpeechStatus,
    showSpeechHistoryPanel,
    startSpeechCapture,
    stopSpeechCapture
  } from '../lib/speech';
  import MeltActionButton from './melt/MeltActionButton.svelte';

  const micIconUrl = new URL(
    '../assets/icons/mic_24dp_E3E3E3_FILL1_wght300_GRAD0_opsz24.svg',
    import.meta.url
  ).href;

  let micModel: MicControlModel = INITIAL_MIC_CONTROL_MODEL;
  let disposed = false;
  $: micPresentation = MIC_CONTROL_PRESENTATION[micModel.state];
  $: micLabel = speechControlLabel(micModel);

  onMount(() => {
    disposed = false;
    let unlisten: (() => void) | null = null;
    listenSpeechStatus((event) => {
      if (!disposed) micModel = reduceSpeechEvent(micModel, event);
    }).then((resolvedUnlisten) => {
      if (disposed) resolvedUnlisten();
      else unlisten = resolvedUnlisten;
    }).catch(() => {
      if (disposed) return;
      micModel = failSpeechCommand(micModel);
    });
    return () => {
      disposed = true;
      unlisten?.();
    };
  });

  async function handleMicControl() {
    if (micModel.state === 'idle' || (micModel.state === 'error' && micModel.nonce === null)) {
      micModel = beginSpeechStart(micModel);
      await settleSpeechCommand(
        startSpeechCapture(),
        () => disposed,
        (response) => { micModel = acceptSpeechStart(micModel, response); },
        (error) => {
          micModel = failSpeechCommand(micModel, normalizeSpeechCommandError(error));
        }
      );
      return;
    }
    if (micModel.state === 'recording' && micModel.nonce !== null) {
      const nonce = micModel.nonce;
      micModel = beginSpeechStop(micModel);
      await settleSpeechCommand(
        stopSpeechCapture({ nonce }),
        () => disposed,
        (response) => { micModel = acceptSpeechStop(micModel, response); },
        (error) => {
          micModel = failSpeechCommand(micModel, normalizeSpeechCommandError(error));
        }
      );
    }
  }

  function openSpeechHistory(event: MouseEvent) {
    event.preventDefault();
    event.stopPropagation();
    const rect = (event.currentTarget as HTMLElement).getBoundingClientRect();
    void showSpeechHistoryPanel({
      anchorLeft: rect.left,
      anchorWidth: rect.width
    });
  }

  function handleSpeechHistoryKeydown(event: KeyboardEvent) {
    if (event.key === 'ContextMenu' || (event.shiftKey && event.key === 'F10')) {
      event.preventDefault();
      event.stopPropagation();
      const rect = (event.currentTarget as HTMLElement).getBoundingClientRect();
      void showSpeechHistoryPanel({
        anchorLeft: rect.left,
        anchorWidth: rect.width
      });
    }
  }
</script>

  <div
    class="mic-control"
    style:--mic-icon-url={`url("${micIconUrl}")`}
  >
  <MeltActionButton
    class={`mic-button mic-button--${micModel.state}`}
    ariaLabel={micLabel}
    ariaHaspopup="dialog"
    ariaPressed={micPresentation.pressed}
    disabled={micPresentation.disabled || (micModel.state === 'error' && micModel.nonce !== null)}
    tooltip={micLabel}
    onClick={handleMicControl}
    onContextMenu={openSpeechHistory}
    onKeyDown={handleSpeechHistoryKeydown}
  >
    <span class="mic-glyph" aria-hidden="true"></span>
    {#if micModel.state === 'stopping' || micModel.state === 'transcribing'}
      <span class="mic-progress-spinner" aria-hidden="true"></span>
    {/if}
    {#if micModel.state === 'copied'}
      <span class="mic-copy-check" aria-hidden="true">✓</span>
    {/if}
  </MeltActionButton>
  <span class="mic-status" role="status" aria-live="polite">{micModel.announcement}</span>
</div>
