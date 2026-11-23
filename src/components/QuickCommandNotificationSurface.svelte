<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { Channel } from '@tauri-apps/api/core';
  import {
    actOnQuickCommandNotification,
    getQuickCommandNotification,
    type QuickCommandBanner
  } from '../lib/quickCommandNotification';
  import MeltActionButton from './melt/MeltActionButton.svelte';

  let banner = $state.raw<QuickCommandBanner | null>(null);
  let actingId = $state<string | null>(null);
  let actionFailed = $state(false);
  let readRevision = 0;
  let acknowledgedId = $state<string | null>(null);
  let disposed = false;
  let card = $state.raw<HTMLElement | null>(null);
  let hovered = false;
  let focused = false;
  // Native accepts pause only for a banner whose render it has acknowledged.
  let pausableId: string | null = null;
  let pausedId = $state<string | null>(null);
  // Native honors at most 30 s of pause per banner; the ring must drain with it.
  const PAUSE_BUDGET_MS = 30_000;
  let ringPaused = $state(false);
  let ringBudgetId: string | null = null;
  let ringBudgetMs = PAUSE_BUDGET_MS;
  let ringPausedAt = 0;
  let ringTimer: ReturnType<typeof setTimeout> | undefined;

  function pauseRing(id: string) {
    resumeRing();
    if (ringBudgetId !== id) {
      ringBudgetId = id;
      ringBudgetMs = PAUSE_BUDGET_MS;
    }
    if (ringBudgetMs <= 0) return;
    ringPaused = true;
    ringPausedAt = performance.now();
    ringTimer = setTimeout(resumeRing, ringBudgetMs);
  }

  function resumeRing() {
    clearTimeout(ringTimer);
    if (!ringPaused) return;
    ringPaused = false;
    ringBudgetMs -= performance.now() - ringPausedAt;
  }

  /** One pause per banner id; resume only when hover and focus have both ended. */
  function syncPause() {
    const id = banner?.id ?? null;
    if (!id) {
      hovered = focused = false;
      pausedId = pausableId = null;
      resumeRing();
      return;
    }
    // An action in flight may retire this banner; settle once it finishes.
    if (disposed || actingId === id) return;
    const wanted = hovered || focused;
    if (wanted && pausableId === id && pausedId !== id) {
      pausedId = id;
      pauseRing(id);
      sendPause(id, 'pause');
    } else if (!wanted && pausedId) {
      const resumed = pausedId;
      pausedId = null;
      resumeRing();
      if (resumed === id) sendPause(id, 'resume');
    }
  }

  function sendPause(id: string, action: 'pause' | 'resume') {
    // Best effort: a stale or unsupported pause must never surface or block.
    void actOnQuickCommandNotification(id, action).catch(() => {});
  }

  function readFocus() {
    focused = card?.contains(document.activeElement) ?? false;
  }

  function releaseHoverWhenHidden() {
    if (document.visibilityState === 'hidden') trackHover(false);
  }

  function trackHover(inside: boolean) {
    hovered = inside;
    syncPause();
  }

  function trackFocus(event: FocusEvent, inside: boolean) {
    focused = inside || (event.relatedTarget instanceof Node && (card?.contains(event.relatedTarget) ?? false));
    syncPause();
  }

  async function refresh(channel?: Channel<null>) {
    const revision = ++readRevision;
    try {
      let current = await getQuickCommandNotification(channel ? { channel } : undefined);
      while (!disposed && revision === readRevision) {
        if (current?.id !== banner?.id) actionFailed = false;
        const replaced = !!banner && !!current && banner.id !== current.id;
        banner = current;
        if (!current || current.id === acknowledgedId) {
          syncPause();
          return;
        }

        const id = current.id;
        await tick();
        if (disposed || revision !== readRevision || banner?.id !== id) return;
        // The card element is reused, so a replacement gets no pointer event of its own.
        if (replaced) hovered = card?.matches(':hover') ?? false;
        acknowledgedId = id;
        // Acknowledging an expired offer can pull its replacement without a channel wake.
        current = await getQuickCommandNotification({ renderedId: id });
        if (disposed || revision !== readRevision) return;
        if (current?.id === id) {
          pausableId = id;
          // Replacing a banner recreates its dismiss button and can drop focus.
          readFocus();
        }
      }
    } catch {
      if (!disposed && revision === readRevision) {
        banner = null;
        syncPause();
      }
    }
  }

  async function act(id: string, action: 'open' | 'dismiss') {
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
      if (!disposed && actingId === id) {
        actingId = null;
        await tick();
        readFocus();
        syncPause();
      }
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
      clearTimeout(ringTimer);
      channel.onmessage = () => {};
    };
  });
</script>

<svelte:window onblur={() => trackHover(false)} />
<svelte:document onvisibilitychange={releaseHoverWhenHidden} />

<!-- Live before any banner: a region inserted together with its text is often not announced. -->
<div class="announcement" role={banner ? (banner.status === 'failure' ? 'alert' : 'status') : undefined} aria-live={banner?.status === 'failure' ? 'assertive' : 'polite'}>
  {#key banner?.id}{#if banner}<span>{banner.label}: {banner.status === 'success' ? 'Succeeded' : 'Failed'}</span>{/if}{/key}
</div>

{#if banner}
  {@const { id, label, status } = banner}
  {@const verdict = status === 'success' ? 'Succeeded' : 'Failed'}
  <div class="notification-viewport">
  <section
    bind:this={card}
    class="notification"
    aria-label="Quick Commands notification"
    onpointerenter={() => trackHover(true)}
    onpointerleave={() => trackHover(false)}
    onfocusin={(event) => trackFocus(event, true)}
    onfocusout={(event) => trackFocus(event, false)}
  >
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
    {#key id}
      <MeltActionButton class="notification-dismiss" ariaLabel="Dismiss notification" disabled={actingId === id} onClick={() => void act(id, 'dismiss')}>
        <svg viewBox="0 0 28 28" width="22" height="22" fill="none" stroke="currentColor" aria-hidden="true">
          <circle class="countdown-track" cx="14" cy="14" r="12" />
          <circle class={['countdown-ring', { draining: acknowledgedId === id, paused: ringPaused && pausedId === id }]} cx="14" cy="14" r="12" pathLength="100" transform="rotate(-90 14 14)" />
          <path d="m10 10 8 8m0-8-8 8" stroke-width="2" stroke-linecap="round" />
        </svg>
      </MeltActionButton>
    {/key}
  </section>
  </div>
{/if}

<style>
  .notification-viewport {
    /* Native client is 327x57; the 320x50 card keeps its on-screen position. */
    box-sizing: border-box;
    width: 100%;
    height: 100vh;
    padding: 7px 0 0 7px;
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
    flex: 0 0 auto;
    width: 32px;
    height: 32px;
    min-width: 0;
    min-height: 0;
    padding: 0;
    color: var(--js-color-text-muted);
  }
  .notification :global(.notification-dismiss:hover) { color: var(--js-color-text-strong); }
  .notification :global(button:focus-visible) { outline-offset: -2px; }
  .countdown-track { opacity: .25; }
  .countdown-ring { stroke-width: 2; stroke-dasharray: 100; stroke-dashoffset: 0; }
  .countdown-ring.draining { animation: dismiss-countdown 6s linear forwards; }
  .countdown-ring.paused { animation-play-state: paused; }
  @keyframes dismiss-countdown { to { stroke-dashoffset: 100; } }
  @media (prefers-reduced-motion: reduce) {
    .countdown-ring.draining { animation: none; }
  }
  .notification :global(button:not(:disabled):hover) { background: var(--js-color-accent-soft); }
  .verdict { flex: 0 0 auto; font-size: .72rem; color: var(--js-color-text-muted); }
  strong { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: .86rem; color: var(--js-color-text); }
  .status-icon { flex: 0 0 auto; color: var(--js-status-success); }
  .status-icon.failed { color: var(--js-status-error); }
  .notification-error { color: var(--js-color-error-text); }
  .announcement {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
  }
</style>
