<script lang="ts">
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { onMount } from 'svelte';
  import SpeechIndicatorOrb from './SpeechIndicatorOrb.svelte';
  import { getSpeechStatus, listenSpeechStatus, listenSpeechVoiceLevel } from '../lib/speech';
  import { createSpeechIndicatorMeter, type SpeechIndicatorMeterSnapshot } from '../lib/speechIndicatorMeter';
  import { createSpeechIndicatorFill } from '../lib/speechIndicatorFill';

  const micIconUrl = new URL(
    '../assets/icons/mic_24dp_E3E3E3_FILL1_wght300_GRAD0_opsz24.svg',
    import.meta.url
  ).href;
  // Exact path from the packaged artwork above, used only for its internal
  // keyline. The original asset remains the actual alpha mask for all paint.
  const micOutlinePath = 'M409.04-449.04Q380-478.08 380-520v-240q0-41.92 29.04-70.96Q438.08-860 480-860q41.92 0 70.96 29.04Q580-801.92 580-760v240q0 41.92-29.04 70.96Q521.92-420 480-420q-41.92 0-70.96-29.04ZM450-130v-131.85q-99-11.31-164.5-84.92Q220-420.39 220-520h60q0 83 58.5 141.5T480-320q83 0 141.5-58.5T680-520h60q0 99.61-65.5 173.23Q609-273.16 510-261.85V-130h-60Z';

  // The pure meter owns the 0.035 speech threshold and 300ms word-gap hold.
  const holdMs = 300;
  // Visual full scale only: raw 0.5 fills the mic; speech detection is unchanged.
  const visualFullScale = 0.5;
  let activeNonce: number | null = null;
  const meter = createSpeechIndicatorMeter();
  let meterSnapshot: SpeechIndicatorMeterSnapshot = { showBars: false, level: 0 };
  let releaseTimer: ReturnType<typeof setTimeout> | undefined;
  let disposed = false;
  let statusRevision = 0;
  let micLevel = 0;
  let fill: ReturnType<typeof createSpeechIndicatorFill> | undefined;

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
    fill?.setTarget(Math.min(1, meterSnapshot.level / visualFullScale));
  }

  function resetMeter() {
    meter.reset();
    meterSnapshot = { showBars: false, level: 0 };
    fill?.reset();
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
    fill = createSpeechIndicatorFill((level) => { micLevel = level; });

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
      fill?.dispose();
      unlistenStatus?.();
      unlistenLevel?.();
    };
  });

  // --mic-level is displayed paint, not the meter's unchanged raw snapshot.
</script>

<main class="speech-indicator" aria-hidden="true">
  <div class="mic-shell">
    <SpeechIndicatorOrb active={activeNonce !== null} />
    <span class="mic-glyph" style:--mic-level={micLevel} style:mask-image={`url("${micIconUrl}")`}>
      <svg class="mic-keyline" viewBox="0 -960 960 960" aria-hidden="true"><path d={micOutlinePath} /></svg>
    </span>
  </div>
</main>

<style>
  :global(html), :global(body) { background: transparent; margin: 0; overflow: hidden; }
  .speech-indicator { align-items: center; display: flex; height: 100%; justify-content: center; pointer-events: none; width: 100%; }
  .mic-shell {
    align-items: center;
    background: var(--js-color-accent, #c0d8e7);
    border-radius: 50%;
    box-sizing: border-box;
    display: flex;
    height: 35px;
    justify-content: center;
    padding: 8px;
    position: relative;
    overflow: hidden;
    isolation: isolate;
    width: 35px;
  }
  .mic-keyline { display: block; height: 100%; width: 100%; fill: none; stroke: #162d49; stroke-width: 40; stroke-linejoin: round; }
  .mic-glyph {
    flex-shrink: 0;
    height: 24px;
    width: 24px;
    position: relative;
    z-index: 1;
    /* Paint within the asset's original y=100..830 path bounds. At .5,
       the stop positions match Cool alloy's selected 0 / .45 / .52 / 1. */
    background: linear-gradient(to bottom, #162d49 0%, #335371 calc((1 - var(--mic-level)) * (1 - 0.2 * var(--mic-level)) * 100%), #a3c2e0 calc((1 - var(--mic-level)) * (1 + 0.08 * var(--mic-level)) * 100%), #d7ebfa 100%);
    background-size: 100% 18.25px;
    background-position: center 2.5px;
    background-repeat: no-repeat;
    mask-size: contain;
    mask-position: center;
    mask-repeat: no-repeat;
  }
  /* The same navy ink/internal keyline remains legible on theme accents when
     WebGL is unavailable; no lighter fallback-only ink or backing disk. */
  @media (forced-colors: active) {
    .mic-shell { background: Canvas; outline: 1px solid CanvasText; outline-offset: -1px; forced-color-adjust: none; }
    .mic-glyph { background: CanvasText; }
    .mic-keyline { display: none; }
    .mic-shell:has(:global(canvas[style*="visibility: hidden"])) .mic-glyph { background: CanvasText; }
  }
</style>
