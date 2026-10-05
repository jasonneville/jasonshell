<script lang="ts">
  import { onMount, tick } from 'svelte';
  import MeltActionButton from './melt/MeltActionButton.svelte';
  import MaterialSymbolIcon from './icons/MaterialSymbolIcon.svelte';
  import { observeSnipContext, snipImageUrl, decodeSnipImage, sameSnip, copySnip, saveSnip, dismissSnip, type SnipContext } from '../lib/snipping';

  let context = $state.raw<SnipContext | null>(null);
  let url = $state('');
  let image = $state<HTMLImageElement>();
  let decoded = $state(false);
  let pending = $state(false);
  let status = $state('');
  let error = $state(false);
  let epoch = 0;
  let disposed = false;
  function revoke() { if (url) URL.revokeObjectURL(url); url = ''; }
  function current(request: number, captured: SnipContext) { return !disposed && epoch === request && sameSnip(context, captured); }
  async function loadContext(next: SnipContext) {
    if (sameSnip(context, next) && context?.monitorId === next.monitorId) return;
    const request = ++epoch;
    context = next; pending = false; decoded = false; status = ''; error = false; revoke();
    try {
      const ownedUrl = await snipImageUrl(next, () => current(request, next));
      if (!current(request, next)) { URL.revokeObjectURL(ownedUrl); return; }
      url = ownedUrl; await tick();
      if (!current(request, next)) return;
      if (!image) throw new Error('image-failed');
      await decodeSnipImage(image);
      if (current(request, next)) decoded = true;
    } catch { if (current(request, next)) { status = 'Could not load screenshot.'; error = true; } }
  }
  async function action(kind: 'copy' | 'save' | 'dismiss') {
    if (pending || !context || (kind !== 'dismiss' && !decoded)) return;
    const captured = context; const request = epoch;
    pending = true; status = ''; error = false;
    try {
      if (kind === 'copy') {
        const result = await copySnip(captured);
        if (!current(request, captured) || !sameSnip(captured, result)) return;
        switch (result.status) {
          case 'committed': status = result.durable === true ? 'Copied' : 'Copied, but clipboard durability is not guaranteed.'; break;
          case 'committed-warning': status = 'Copied, but clipboard durability is not guaranteed.'; break;
          case 'publication-unknown': status = 'Could not confirm clipboard publication.'; error = true; break;
          case 'rejected': status = 'Clipboard busy; try again.'; error = true; break;
        }
      } else if (kind === 'save') {
        const result = await saveSnip(captured);
        if (!current(request, captured) || !sameSnip(captured, result)) return;
        status = result.status === 'saved' ? 'Saved' : result.status === 'error' ? 'Could not save screenshot.' : '';
        error = result.status === 'error';
      } else {
        const result = await dismissSnip(captured);
        if (!current(request, captured) || !sameSnip(captured, result)) return;
        ++epoch; context = null; decoded = false; revoke(); pending = false;
      }
    } catch {
      if (current(request, captured)) {
        status = kind === 'save' ? 'Could not save screenshot.' : kind === 'copy'
          ? 'Could not confirm clipboard publication.' : 'Could not dismiss screenshot.';
        error = true;
      }
    } finally { if (current(request, captured)) pending = false; }
  }
  onMount(() => {
    const stop = observeSnipContext('preview', (next) => void loadContext(next), undefined,
      () => { status = 'Could not load screenshot.'; error = true; });
    return () => { disposed = true; ++epoch; stop(); revoke(); };
  });
</script>

<section class="surface preview-surface" aria-label="Screen snip preview" aria-busy={pending}>
  <header class="preview-header">
    <div class="preview-copy">
      <div class="preview-title">Screen snip</div>
      {#if context}<div class="preview-process">{context.width} × {context.height} pixels</div>{/if}
    </div>
    <MeltActionButton class="preview-close-button" ariaLabel="Close screen snip" disabled={pending || !context} onClick={() => void action('dismiss')}><MaterialSymbolIcon name="close" /></MeltActionButton>
  </header>
  <div class="preview-frame">
    {#if url}<img bind:this={image} src={url} alt="Snip preview" draggable="false" style:visibility={decoded ? 'visible' : 'hidden'} />{/if}
  </div>
  <div class="preview-status" role={error ? 'alert' : 'status'}>{status}</div>
  <div class="preview-actions">
    <MeltActionButton class="preview-action-button" ariaLabel="Copy" disabled={pending || !decoded} onClick={() => void action('copy')}><MaterialSymbolIcon name="file_copy" /></MeltActionButton>
    <MeltActionButton class="preview-action-button" ariaLabel="Save" disabled={pending || !decoded} onClick={() => void action('save')}><MaterialSymbolIcon name="save" /></MeltActionButton>
    <MeltActionButton class="preview-action-button" ariaLabel="Dismiss" disabled={pending || !context} onClick={() => void action('dismiss')}><MaterialSymbolIcon name="close" /></MeltActionButton>
  </div>
</section>

<style>
  .preview-surface { background: var(--js-bg-surface); border: 1px solid var(--js-color-border-soft); border-radius: var(--js-radius-sm); box-shadow: var(--js-shadow-raised),var(--js-inset-highlight); color: var(--js-color-text); padding: .35rem; display: flex; flex-direction: column; gap: .25rem; box-sizing: border-box; width: 100%; height: 100%; min-height: 0; min-width: 0; }
  .preview-header { display: flex; align-items: center; justify-content: space-between; gap: var(--js-space-2); flex: 0 0 auto; min-width: 0; }
  .preview-copy { min-width: 0; }
  .preview-title { font-size: .72rem; font-weight: 750; }
  .preview-process { font-size: .58rem; font-weight: 600; color: var(--js-color-text-muted); }
  .preview-frame { background: var(--js-color-surface-sunken); border: 1px solid var(--js-color-border-soft); border-radius: calc(var(--js-radius-sm) - 1px); flex: 1 1 0; min-height: 0; min-width: 0; overflow: hidden; }
  img { display: block; width: 100%; height: 100%; object-fit: contain; }
  .preview-status { flex: 0 0 auto; min-height: 1rem; font-size: .64rem; color: var(--js-color-text-muted); overflow-wrap: anywhere; }
  .preview-status[role='alert'] { color: var(--js-color-error-text); }
  .preview-actions { display: flex; justify-content: flex-end; flex: 0 0 auto; gap: var(--js-space-2); }
  .preview-surface :global(.preview-action-button),
  .preview-surface :global(.preview-close-button) { display: inline-flex; align-items: center; justify-content: center; flex: 0 0 auto; width: 2rem; height: 2rem; border-radius: var(--js-radius-xs); color: var(--js-color-text); }
  .preview-surface :global(.preview-action-button:not(:disabled):hover),
  .preview-surface :global(.preview-close-button:not(:disabled):hover) { background: var(--js-color-accent-soft); }
  .preview-surface :global(button:disabled) { opacity: .45; }
</style>
