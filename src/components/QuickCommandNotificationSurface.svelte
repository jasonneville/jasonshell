<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { Channel } from '@tauri-apps/api/core';
  import {
    actOnQuickCommandNotification,
    getQuickCommandNotification,
    type QuickCommandBanner,
    type QuickCommandBannerAction
  } from '../lib/quickCommandNotification';
  import MeltActionButton from './melt/MeltActionButton.svelte';

  let banner = $state.raw<QuickCommandBanner | null>(null);
  let actingId = $state<string | null>(null);
  let actionFailed = $state(false);
  let readRevision = 0;
  let acknowledgedId = $state<string | null>(null);
  let disposed = false;

  async function refresh(channel?: Channel<null>) {
    const revision = ++readRevision;
    try {
      let current = await getQuickCommandNotification(channel ? { channel } : undefined);
      while (!disposed && revision === readRevision) {
        if (current?.id !== banner?.id) actionFailed = false;
        banner = current;
        if (!current || current.id === acknowledgedId) return;

        const id = current.id;
        await tick();
        if (disposed || revision !== readRevision || banner?.id !== id) return;
        acknowledgedId = id;
        // Acknowledging an expired offer can pull its replacement without a channel wake.
        current = await getQuickCommandNotification({ renderedId: id });
      }
    } catch {
      if (!disposed && revision === readRevision) banner = null;
    }
  }

  async function act(id: string, action: QuickCommandBannerAction) {
    if (disposed || banner?.id !== id || actingId === id) return;
    actingId = id;
    actionFailed = false;
    // A read submitted before the action must not repaint its old banner.
    ++readRevision;
    try {
      await actOnQuickCommandNotification(id, action);
      if (!disposed && banner?.id === id) await refresh();
    } catch {
      if (!disposed && banner?.id === id) actionFailed = true;
    } finally {
      if (!disposed && actingId === id) actingId = null;
    }
  }

  onMount(() => {
    const channel = new Channel<null>();
    channel.onmessage = () => {
      if (!disposed) void refresh();
    };
    void refresh(channel);
    return () => {
      disposed = true;
      channel.onmessage = () => {};
    };
  });
</script>

{#if banner}
  {@const { id, label, status } = banner}
  {@const verdict = status === 'success' ? 'Succeeded' : 'Failed'}
  <div class="notification-viewport">
  <section class="notification" aria-label="Quick Commands notification">
    {#key id}
      <MeltActionButton class="notification-dismiss" ariaLabel="Dismiss notification" disabled={actingId === id} onClick={() => void act(id, 'dismiss')}>
        <svg viewBox="0 0 28 28" width="14" height="14" fill="none" stroke="currentColor" aria-hidden="true">
          <circle class="countdown-track" cx="14" cy="14" r="12" />
          <circle class={['countdown-ring', { draining: acknowledgedId === id }]} cx="14" cy="14" r="12" pathLength="100" transform="rotate(-90 14 14)" />
          <path d="m9 9 10 10m0-10L9 19" stroke-width="2.5" stroke-linecap="round" />
        </svg>
      </MeltActionButton>
    {/key}
    <div class="notification-body">
      <MeltActionButton class="notification-open" ariaLabel={`Open ${label} output — ${verdict}`} disabled={actingId === id} onClick={() => void act(id, 'open')}>
        <strong title={label}>{label}</strong>
        <span class="verdict">{verdict}</span>
        <svg class={['status-icon', { failed: status === 'failure' }]} viewBox="0 0 20 20" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
          {#if status === 'success'}<path d="m4.5 10 3.5 3.5 7.5-7.5" />{:else}<path d="M10 4.5v7m0 3.5h.01" />{/if}
        </svg>
      </MeltActionButton>
      {#if actionFailed}<span class="notification-error" role="alert">Action unavailable. Try again.</span>{/if}
    </div>
  </section>
  </div>
{/if}

<style>
  .notification-viewport {
    --dismiss-diameter: 14px;
    position: relative;
    box-sizing: border-box;
    width: 100%;
    height: 100vh;
    padding: calc(var(--dismiss-diameter) / 2) 0 0 calc(var(--dismiss-diameter) / 2);
  }
  .notification {
    display: flex;
    align-items: center;
    gap: var(--js-space-2);
    box-sizing: border-box;
    width: 100%;
    height: 100%;
    padding: 4px;
    border: 1px solid var(--js-color-border-soft);
    border-radius: var(--js-radius-sm);
    background: var(--js-bg-surface);
    box-shadow: var(--js-shadow-raised), var(--js-inset-highlight);
    color: var(--js-color-text);
  }
  .notification :global(.notification-open), .notification :global(.notification-dismiss) {
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: var(--js-radius-xs);
    color: inherit;
  }
  .notification-body { display: flex; flex-direction: column; flex: 1; align-self: stretch; min-width: 0; }
  .notification-error {
    font-size: .72rem;
    line-height: 14px;
    padding-inline: var(--js-space-2);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .notification :global(.notification-open) {
    justify-content: flex-start;
    flex: 1;
    align-self: stretch;
    min-width: 0;
    gap: var(--js-space-2);
    padding-inline: var(--js-space-2);
    text-align: left;
  }
  .notification :global(.notification-dismiss) {
    position: absolute;
    top: 0;
    left: 0;
    z-index: 1;
    width: var(--dismiss-diameter);
    height: var(--dismiss-diameter);
    min-width: 0;
    min-height: 0;
    padding: 0;
    border-radius: 50%;
    background: var(--js-color-surface-raised);
  }
  .notification :global(.notification-dismiss:focus-visible) { outline-offset: -1px; }
  .countdown-track { opacity: .25; }
  .countdown-ring { stroke-width: 2; stroke-dasharray: 100; stroke-dashoffset: 0; }
  .countdown-ring.draining { animation: dismiss-countdown 6s linear forwards; }
  @keyframes dismiss-countdown { to { stroke-dashoffset: 100; } }
  @media (prefers-reduced-motion: reduce) {
    .countdown-ring.draining { animation: none; }
  }
  .notification :global(button:not(:disabled):hover) { background: var(--js-color-accent-soft); }
  .verdict { flex: 0 0 auto; font-size: .72rem; color: var(--js-color-text-muted); }
  strong { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: .86rem; color: var(--js-color-text); }
  .status-icon { flex: 0 0 auto; color: var(--js-color-success-border); }
  .failed, [role='alert'] { color: var(--js-color-error-text); }
</style>
