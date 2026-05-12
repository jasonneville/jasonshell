<script lang="ts">
  import { onMount } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import {
    AUDIO_PANEL_CLOSED_EVENT,
    AUDIO_PANEL_OPEN_EVENT,
    getAudioState,
    hideAudioPanel,
    normalizeVolumePercent,
    setAppVolume,
    setDefaultAudioInputDevice,
    setDefaultAudioOutputDevice,
    setMasterVolume,
    type AudioRefreshReason,
    type AudioState
  } from '../lib/audio';
  import MeltActionButton from './melt/MeltActionButton.svelte';

  const AUDIO_REFRESH_DEBOUNCE_MS = 150;
  const AUDIO_REFRESH_POLL_MS = 2000;

  let audioLoading = false;
  let audioError = '';
  let audioState: AudioState | null = null;
  let audioCommandSequence = 0;
  let audioRefreshTimer: ReturnType<typeof setTimeout> | null = null;
  let audioRefreshPollTimer: ReturnType<typeof setInterval> | null = null;
  let audioPanelVisible = false;

  function applyAudioState(nextAudioState: AudioState, options: { clearError?: boolean } = {}) {
    audioState = {
      ...nextAudioState,
      masterVolumePercent: normalizeVolumePercent(nextAudioState.masterVolumePercent),
      sessions: nextAudioState.sessions.map((session) => ({
        ...session,
        volumePercent: normalizeVolumePercent(session.volumePercent)
      }))
    };
    if (options.clearError ?? true) {
      audioError = '';
    }
  }

  async function refreshAudioState(_options: { reason?: AudioRefreshReason } = {}) {
    audioLoading = true;
    try {
      applyAudioState(await getAudioState());
    } catch (error) {
      console.error('Failed to load audio state', error);
      audioError = 'Audio controls unavailable';
    } finally {
      audioLoading = false;
    }
  }

  function scheduleAudioRefresh(reason: AudioRefreshReason) {
    if (!audioPanelVisible) {
      return;
    }
    if (audioRefreshTimer) {
      clearTimeout(audioRefreshTimer);
    }
    audioRefreshTimer = setTimeout(() => {
      audioRefreshTimer = null;
      void refreshAudioState({ reason });
    }, AUDIO_REFRESH_DEBOUNCE_MS);
  }

  function startAudioRefreshPolling() {
    if (audioRefreshPollTimer) {
      return;
    }
    audioRefreshPollTimer = setInterval(() => {
      if (audioPanelVisible) {
        void refreshAudioState({ reason: 'session-changed' });
      }
    }, AUDIO_REFRESH_POLL_MS);
  }

  function stopAudioRefreshPolling() {
    if (audioRefreshPollTimer) {
      clearInterval(audioRefreshPollTimer);
      audioRefreshPollTimer = null;
    }
  }

  function stopPendingAudioRefresh() {
    if (audioRefreshTimer) {
      clearTimeout(audioRefreshTimer);
      audioRefreshTimer = null;
    }
  }

  function closeAudioPanel() {
    audioPanelVisible = false;
    stopPendingAudioRefresh();
    stopAudioRefreshPolling();
    void hideAudioPanel();
  }

  function audioVolumeLabel(value: number | null | undefined) {
    return `${normalizeVolumePercent(value ?? 0)}%`;
  }

  function updateMasterVolumeLocal(volumePercent: number) {
    audioState = {
      ...(audioState ?? {
        masterVolumePercent: volumePercent,
        inputDevices: [],
        outputDevices: [],
        sessions: []
      }),
      masterVolumePercent: volumePercent
    };
  }

  function updateSessionVolumeLocal(sessionId: string, volumePercent: number) {
    if (!audioState) {
      return;
    }
    audioState = {
      ...audioState,
      sessions: audioState.sessions.map((session) => (
        session.id === sessionId ? { ...session, volumePercent } : session
      ))
    };
  }

  function updateDefaultDeviceLocal(kind: 'input' | 'output', deviceId: string) {
    if (!audioState) {
      return;
    }
    audioState = kind === 'input'
      ? { ...audioState, defaultInputDeviceId: deviceId }
      : { ...audioState, defaultOutputDeviceId: deviceId };
  }

  function commitAudioCommand(command: Promise<AudioState>) {
    const sequence = ++audioCommandSequence;
    command.then((nextAudioState) => {
      if (sequence === audioCommandSequence) {
        applyAudioState(nextAudioState);
      }
    }).catch((error) => {
      if (sequence !== audioCommandSequence) {
        return;
      }
      console.error('Failed to update audio state', error);
      audioError = 'Audio update failed';
      getAudioState().then((nextAudioState) => {
        if (sequence === audioCommandSequence) {
          applyAudioState(nextAudioState, { clearError: false });
        }
      }).catch((refreshError) => {
        console.error('Failed to refresh audio state after update failure', refreshError);
      });
    });
  }

  function handleMasterVolumeInput(event: Event) {
    const volumePercent = normalizeVolumePercent(Number((event.currentTarget as HTMLInputElement).value));
    updateMasterVolumeLocal(volumePercent);
    commitAudioCommand(setMasterVolume(volumePercent));
  }

  function handleSessionVolumeInput(sessionId: string, event: Event) {
    const volumePercent = normalizeVolumePercent(Number((event.currentTarget as HTMLInputElement).value));
    updateSessionVolumeLocal(sessionId, volumePercent);
    commitAudioCommand(setAppVolume(sessionId, volumePercent));
  }

  function handleOutputDeviceChange(event: Event) {
    const deviceId = (event.currentTarget as HTMLSelectElement).value;
    updateDefaultDeviceLocal('output', deviceId);
    commitAudioCommand(setDefaultAudioOutputDevice(deviceId));
  }

  function handleInputDeviceChange(event: Event) {
    const deviceId = (event.currentTarget as HTMLSelectElement).value;
    updateDefaultDeviceLocal('input', deviceId);
    commitAudioCommand(setDefaultAudioInputDevice(deviceId));
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      closeAudioPanel();
    }
  }

  onMount(() => {
    const unlisteners: Array<() => void> = [];
    let disposed = false;
    const registerAsyncUnlistener = (registration: Promise<() => void>) => {
      void registration.then((unlisten) => {
        if (disposed) {
          unlisten();
          return;
        }
        unlisteners.push(unlisten);
      });
    };

    registerAsyncUnlistener(listen(AUDIO_PANEL_OPEN_EVENT, () => {
      audioPanelVisible = true;
      void refreshAudioState();
      startAudioRefreshPolling();
    }));
    registerAsyncUnlistener(listen(AUDIO_PANEL_CLOSED_EVENT, () => {
      audioPanelVisible = false;
      stopPendingAudioRefresh();
      stopAudioRefreshPolling();
    }));
    return () => {
      disposed = true;
      stopPendingAudioRefresh();
      audioPanelVisible = false;
      stopAudioRefreshPolling();
      for (const unlisten of unlisteners) {
        unlisten();
      }
    };
  });
</script>

<svelte:window on:keydown={handleKeydown} />

<div class="audio-panel surface" id="audio-panel" role="dialog" aria-label="Sound controls">
  <div class="sound-panel-header">
    <strong>Sound</strong>
    <div class="sound-panel-actions">
      <MeltActionButton class="sound-refresh" ariaLabel="Refresh sound devices" onClick={() => void refreshAudioState()}>Refresh</MeltActionButton>
      <MeltActionButton class="sound-refresh" ariaLabel="Close sound controls" onClick={closeAudioPanel}>Close</MeltActionButton>
    </div>
  </div>
  {#if audioError}
    <p class="sound-status" role="status">{audioError}</p>
  {:else if audioLoading && !audioState}
    <p class="sound-status" role="status">Loading audio controls...</p>
  {:else if audioState}
    <label class="sound-slider">
      <span>Master <strong>{audioVolumeLabel(audioState.masterVolumePercent)}</strong></span>
      <input
        type="range"
        min="0"
        max="100"
        value={audioState.masterVolumePercent}
        aria-label="Master volume"
        on:input={handleMasterVolumeInput}
      />
    </label>
    <label class="sound-select">
      <span>Output</span>
      <select
        value={audioState.defaultOutputDeviceId ?? ''}
        aria-label="Output audio device"
        on:change={handleOutputDeviceChange}
      >
        {#each audioState.outputDevices as device (device.id)}
          <option value={device.id}>{device.name}</option>
        {/each}
      </select>
    </label>
    <label class="sound-select">
      <span>Input</span>
      <select
        value={audioState.defaultInputDeviceId ?? ''}
        aria-label="Input audio device"
        on:change={handleInputDeviceChange}
      >
        {#each audioState.inputDevices as device (device.id)}
          <option value={device.id}>{device.name}</option>
        {/each}
      </select>
    </label>
    <div class="sound-apps" role="group" aria-label="Application volumes">
      {#if audioState.sessions.length}
        {#each audioState.sessions as session (session.id)}
          <label class="sound-slider app-volume">
            <span>{session.name} <strong>{audioVolumeLabel(session.volumePercent)}</strong></span>
            <input
              type="range"
              min="0"
              max="100"
              value={session.volumePercent}
              aria-label={`${session.name} volume`}
              on:input={(event) => handleSessionVolumeInput(session.id, event)}
            />
          </label>
        {/each}
      {:else}
        <p class="sound-status">No active app audio</p>
      {/if}
    </div>
  {/if}
</div>

<style>
  .audio-panel.surface {
    background: var(--js-bg-surface);
    border: 1px solid var(--js-color-border);
    box-shadow: var(--js-shadow-raised), var(--js-inset-highlight);
    box-sizing: border-box;
    color: var(--js-color-text);
    display: grid;
    gap: 0.55rem;
    height: 100%;
    overflow: auto;
    padding: 0.62rem;
    width: 100%;
  }

  .sound-panel-header {
    align-items: center;
    display: flex;
    gap: 0.5rem;
    justify-content: space-between;
  }

  .sound-panel-header strong {
    font-size: 0.68rem;
    letter-spacing: 0;
    text-transform: uppercase;
  }

  .sound-panel-actions {
    display: inline-flex;
    gap: 0.35rem;
  }

  :global(.sound-refresh) {
    background: var(--js-bg-control);
    border: 1px solid var(--js-color-border);
    border-radius: var(--js-radius-sm);
    color: var(--js-color-text);
    cursor: pointer;
    font-size: 0.58rem;
    font-weight: 750;
    min-height: 1.35rem;
    padding: 0.08rem 0.4rem;
  }

  :global(.sound-refresh:hover) {
    background: var(--js-color-control-hover);
    border-color: var(--js-color-accent-border);
  }

  .sound-slider,
  .sound-select {
    display: grid;
    gap: 0.22rem;
  }

  .sound-slider span,
  .sound-select span {
    align-items: center;
    color: var(--js-color-text-muted);
    display: flex;
    font-size: 0.58rem;
    font-weight: 800;
    justify-content: space-between;
    letter-spacing: 0;
    min-width: 0;
    text-transform: uppercase;
  }

  .sound-slider span strong {
    color: var(--js-color-text);
    font-variant-numeric: tabular-nums;
  }

  .sound-slider input[type='range'] {
    accent-color: var(--js-color-accent);
    width: 100%;
  }

  .sound-select select {
    background: var(--js-color-surface-sunken);
    border: 1px solid var(--js-color-border);
    border-radius: var(--js-radius-sm);
    color: var(--js-color-text);
    font: inherit;
    font-size: 0.62rem;
    min-height: 1.5rem;
    min-width: 0;
    padding: 0 0.35rem;
  }

  .sound-apps {
    border-top: 1px solid var(--js-color-border-soft);
    display: grid;
    gap: 0.45rem;
    max-height: 14rem;
    overflow: auto;
    padding-top: 0.55rem;
  }

  .app-volume span {
    text-transform: none;
  }

  .sound-status {
    color: var(--js-color-text-muted);
    font-size: 0.62rem;
    margin: 0;
  }
</style>
