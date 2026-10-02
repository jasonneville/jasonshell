<script lang="ts">
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { onMount } from 'svelte';
  import { getSpeechStatus, listenSpeechStatus, listenSpeechVoiceLevel } from '../lib/speech';
  import { createSpeechIndicatorMeter, type SpeechIndicatorMeterSnapshot } from '../lib/speechIndicatorMeter';

  const micIconUrl = new URL(
    '../assets/icons/mic_24dp_E3E3E3_FILL1_wght300_GRAD0_opsz24.svg',
    import.meta.url
  ).href;

  // The pure meter owns the 0.035 speech threshold and 300ms word-gap hold.
  const holdMs = 300;
  let activeNonce: number | null = null;
  const meter = createSpeechIndicatorMeter();
  let meterSnapshot: SpeechIndicatorMeterSnapshot = { showBars: false, level: 0 };
  let releaseTimer: ReturnType<typeof setTimeout> | undefined;
  let disposed = false;
  let statusRevision = 0;

  function hideSpeechIndicator() {
    activeNonce = null;
    resetMeter();
    // This transparent, pointer-ignoring surface never manages popup, overlay, or panel focus.
    void getCurrentWindow().hide();
  }

  function showSpeechIndicator(nonce: number) {
    if (activeNonce !== nonce) resetMeter();
    activeNonce = nonce;
    void getCurrentWindow().show().then(() => {
      if (disposed || activeNonce !== nonce) void getCurrentWindow().hide();
    });
  }

  function applyVoiceLevel(level: number) {
    const wasAudible = meter.update(level, performance.now());
    syncMeter();
    if (!meterSnapshot.showBars) {
      if (releaseTimer) clearTimeout(releaseTimer);
      releaseTimer = undefined;
      return;
    }
    if (!wasAudible) return;
    if (releaseTimer) clearTimeout(releaseTimer);
    releaseTimer = setTimeout(() => {
      syncMeter();
      releaseTimer = undefined;
    }, holdMs);
  }

  function syncMeter() {
    meterSnapshot = meter.snapshot(performance.now());
  }

  function resetMeter() {
    meter.reset();
    meterSnapshot = { showBars: false, level: 0 };
    if (releaseTimer) clearTimeout(releaseTimer);
    releaseTimer = undefined;
  }

  function applySpeechStatus(event: { status: string; nonce: number | null }) {
    if (event.status === 'recording' && event.nonce !== null) {
      showSpeechIndicator(event.nonce);
      return;
    }
    hideSpeechIndicator();
  }

  onMount(() => {
    let unlistenStatus: (() => void) | undefined;
    let unlistenLevel: (() => void) | undefined;
    disposed = false;
    statusRevision = 0;

    void listenSpeechStatus((event) => {
      statusRevision += 1;
      if (event.status === 'recording' && event.nonce !== null) {
        showSpeechIndicator(event.nonce);
        return;
      }
      // idle, transcribing, copied, and error always end visible capture feedback.
      // Recording and meter updates remain nonce-scoped; terminal state must fail closed.
      hideSpeechIndicator();
    }).then((unlisten) => {
      if (disposed) unlisten();
      else {
        unlistenStatus = unlisten;
        // Any status event received after this point supersedes this snapshot.
        const hydrationRevision = statusRevision;
        void getSpeechStatus().then((status) => {
          if (!disposed && statusRevision === hydrationRevision) applySpeechStatus(status);
        }).catch(() => {
          if (!disposed && statusRevision === hydrationRevision) hideSpeechIndicator();
        });
      }
    });

    void listenSpeechVoiceLevel((event) => {
      if (!disposed && event.nonce === activeNonce) applyVoiceLevel(event.level);
    }).then((unlisten) => {
      if (disposed) unlisten();
      else unlistenLevel = unlisten;
    });

    return () => {
      disposed = true;
      resetMeter();
      unlistenStatus?.();
      unlistenLevel?.();
    };
  });

  $: isSpeaking = meterSnapshot.showBars;
</script>

<main class="speech-indicator" aria-hidden="true">
  <div class={isSpeaking ? 'mic-shell mic-shell--speaking' : 'mic-shell'}>
    <img class="mic-glyph" src={micIconUrl} alt="" />
    <span class="mic-glyph-contrast"></span>
  </div>
</main>

<style>
  :global(html), :global(body) { background: transparent; margin: 0; overflow: hidden; }
  .speech-indicator { align-items: center; display: flex; height: 100%; justify-content: center; pointer-events: none; width: 100%; }
  .mic-shell {
    align-items: center;
    background: color-mix(in srgb, var(--js-color-surface-raised) 55%, transparent);
    border: 1px solid color-mix(in srgb, #8fe3ff 24%, transparent);
    border-radius: 50%;
    box-sizing: border-box;
    box-shadow: inset 0 0 3px 1px transparent;
    display: flex;
    height: 40px;
    justify-content: center;
    padding: 7px;
    width: 40px;
    transition: border-color 160ms ease, box-shadow 160ms ease;
  }
  .mic-shell--speaking {
    border-color: #8fe3ff;
    box-shadow: inset 0 0 3px 1px color-mix(in srgb, #8fe3ff 35%, transparent);
  }
  .mic-glyph { flex-shrink: 0; height: 24px; width: 24px; }
  .mic-glyph-contrast { display: none; flex-shrink: 0; height: 24px; width: 24px; }
  @media (prefers-reduced-motion: reduce) { .mic-shell { transition: none; } }
  @media (forced-colors: active) {
    .mic-shell { background: Canvas; border: 1px solid CanvasText; box-shadow: none; forced-color-adjust: none; }
    .mic-shell--speaking { border-color: Highlight; }
    .mic-glyph { display: none; }
    .mic-glyph-contrast { background: CanvasText; display: block; mask: url('../assets/icons/mic_24dp_E3E3E3_FILL1_wght300_GRAD0_opsz24.svg') center / contain no-repeat; }
  }
</style>
