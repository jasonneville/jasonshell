<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { beginSelection, moveSelection, finishSelection, normalizeSelection, type SelectionState, type Pointer } from '../lib/snipSelection';
  import { observeSnipContext, snipImageUrl, decodeSnipImage, sameSnip, readySnip, beginSnip, completeSnip, cancelSnip, type SnipContext, type SnipToken } from '../lib/snipping';

  let root: HTMLDivElement;
  let image = $state<HTMLImageElement>();
  let context = $state.raw<SnipContext | null>(null);
  let url = $state('');
  let decoded = $state(false);
  let ready = $state(false);
  let selection = $state.raw<SelectionState | null>(null);
  let bounds = $state({ width: 0, height: 0 });
  let epoch = 0;
  let disposed = false;
  let terminal = false;
  let armedToken: SnipToken | null = null;
  let beginPending: Promise<boolean> | null = null;
  let acknowledged = false;
  const rect = $derived(selection ? normalizeSelection(selection.start, selection.end, bounds) : null);

  function revoke() { if (url) URL.revokeObjectURL(url); url = ''; }
  function release(pointerId: number) {
    try { if (root.hasPointerCapture(pointerId)) root.releasePointerCapture(pointerId); } catch { /* Browser already released it. */ }
  }
  function current(request: number, captured: SnipContext) {
    return !disposed && epoch === request && sameSnip(context, captured);
  }
  async function loadContext(next: SnipContext) {
    if (sameSnip(context, next) && context?.monitorId === next.monitorId) return;
    const request = ++epoch;
    const oldPointer = selection?.pointerId;
    selection = null;
    if (oldPointer !== undefined) release(oldPointer);
    context = next; ready = false; decoded = false; terminal = false; acknowledged = false; beginPending = null;
    revoke();
    try {
      const ownedUrl = await snipImageUrl(next, () => current(request, next));
      if (!current(request, next)) { URL.revokeObjectURL(ownedUrl); return; }
      url = ownedUrl;
      await tick();
      if (!current(request, next)) return;
      if (!image) throw new Error('image-failed');
      await decodeSnipImage(image);
      if (!current(request, next)) return;
      decoded = true;
      const result = await readySnip(next);
      if (!current(request, next) || !sameSnip(next, result)) return;
      acknowledged = true;
      ready = result.ready === true || sameSnip(next, armedToken);
    } catch { if (current(request, next)) void cancel(); }
  }
  function pointer(event: PointerEvent): Pointer {
    const measured = root.getBoundingClientRect();
    return { x: event.clientX - measured.left, y: event.clientY - measured.top,
      pointerId: event.pointerId, button: event.button, isPrimary: event.isPrimary };
  }
  function down(event: PointerEvent) {
    if (!ready || terminal || selection || !context) return;
    const measured = root.getBoundingClientRect();
    bounds = { width: measured.width, height: measured.height };
    const started = beginSelection(pointer(event), bounds);
    if (!started) return;
    event.preventDefault(); selection = started;
    try { root.setPointerCapture(event.pointerId); } catch { void cancel(); return; }
    const captured = context; const request = epoch;
    beginPending = beginSnip(captured).then((result) => current(request, captured)
      && sameSnip(captured, result) && result.accepted === true).catch(() => false);
  }
  function move(event: PointerEvent) { if (!terminal) selection = moveSelection(selection, pointer(event), bounds); }
  async function up(event: PointerEvent) {
    if (terminal || selection?.pointerId !== event.pointerId || !context) return;
    const finished = finishSelection(selection, pointer(event), bounds);
    const captured = context; const request = epoch; const pending = beginPending;
    terminal = true; selection = null; release(event.pointerId);
    if (!finished.selection) { void cancelSnip(captured).catch(() => undefined); return; }
    if (!(await pending) || !current(request, captured)) return;
    try { await completeSnip(captured, finished.selection); }
    catch { if (current(request, captured)) void cancelSnip(captured).catch(() => undefined); }
  }
  async function cancel() {
    if (terminal || !context) return;
    terminal = true; ready = false;
    revoke(); decoded = false;
    const pointerId = selection?.pointerId; selection = null;
    if (pointerId !== undefined) release(pointerId);
    await cancelSnip(context).catch(() => undefined);
  }
  function pointerCancel(event: PointerEvent) {
    if (selection && (event.pointerId === undefined || event.pointerId === selection.pointerId)) void cancel();
  }
  onMount(() => {
    const stop = observeSnipContext('selecting', (next) => void loadContext(next), (token) => {
      if (!sameSnip(context, token)) return;
      armedToken = token;
      if (decoded && acknowledged && sameSnip(context, token) && !terminal) ready = true;
    }, () => void cancel());
    return () => { disposed = true; ++epoch; stop(); revoke(); };
  });
</script>

<svelte:window onkeydown={(event) => { if (event.key === 'Escape') { event.preventDefault(); void cancel(); } }} />
<!-- The native overlay owns keyboard cancellation; this region never takes focus. -->
<div bind:this={root} class="snip-overlay" role="region" aria-label="Screen snip selection"
  onpointerdown={down} onpointermove={move} onpointerup={(event) => void up(event)}
  onpointercancel={pointerCancel} onlostpointercapture={pointerCancel}>
  {#if url}
    <img bind:this={image} src={url} alt="Frozen desktop" draggable="false" style:visibility={decoded ? 'visible' : 'hidden'} />
  {/if}
  {#if ready}
    <div class="dim" style={`left:0;top:0;width:100%;height:${rect ? rect.y + 'px' : '100%'}`}></div>
    {#if rect}
      <div class="dim" style={`left:0;top:${rect.y}px;width:${rect.x}px;height:${rect.height}px`}></div>
      <div class="dim" style={`left:${rect.x + rect.width}px;top:${rect.y}px;right:0;height:${rect.height}px`}></div>
      <div class="dim" style={`left:0;top:${rect.y + rect.height}px;width:100%;bottom:0`}></div>
      <div class="selection" style={`left:${rect.x}px;top:${rect.y}px;width:${rect.width}px;height:${rect.height}px`}></div>
    {/if}
    <span class="hint">Esc to cancel</span>
  {/if}
</div>

<style>
  .snip-overlay { position: fixed; inset: 0; width: 100vw; height: 100vh; overflow: hidden; touch-action: none; cursor: crosshair; }
  img { position: absolute; inset: 0; width: 100%; height: 100%; object-fit: fill; pointer-events: none; }
  .dim { position: absolute; background: rgba(0,0,0,.48); pointer-events: none; }
  .selection { position: absolute; border: 2px solid var(--js-color-accent); box-shadow: var(--js-focus-ring); box-sizing: border-box; pointer-events: none; }
  .hint { position: absolute; top: .5rem; left: .5rem; color: var(--js-color-text); background: var(--js-bg-surface); pointer-events: none; }
</style>
