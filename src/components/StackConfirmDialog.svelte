<script lang="ts">
  import { onMount, tick } from 'svelte';

  export let title: string;
  export let message = '';
  export let confirmLabel = 'Confirm';
  export let cancelLabel = 'Cancel';
  export let tone: 'default' | 'danger' = 'default';
  export let busy = false;
  export let initialFocus: 'cancel' | 'confirm' = 'cancel';
  export let dismissOnBackdrop = true;
  export let dismissOnEscape = true;
  export let onConfirm: () => void;
  export let onCancel: () => void;
  export let returnFocus: HTMLElement | null = null;

  const id = `stack-confirm-${Math.random().toString(36).slice(2)}`;
  let dialog: HTMLDivElement;
  let cancelButton: HTMLButtonElement;
  let confirmButton: HTMLButtonElement;

  onMount(() => {
    const origin = returnFocus ?? (document.activeElement instanceof HTMLElement ? document.activeElement : null);
    void tick().then(() => (initialFocus === 'confirm' ? confirmButton : cancelButton)?.focus());
    return () => void tick().then(() => origin?.isConnected && origin.focus());
  });

  function cancel() {
    if (!busy) onCancel();
  }

  function focusableDescendants() {
    const candidates = dialog.querySelectorAll<HTMLElement>([
      'a[href]',
      'area[href]',
      'button',
      'input:not([type="hidden"])',
      'select',
      'textarea',
      'iframe',
      'object',
      'embed',
      'summary',
      'audio[controls]',
      'video[controls]',
      '[contenteditable]:not([contenteditable="false"])',
      '[tabindex]'
    ].join(','));

    return Array.from(candidates).filter((element) => {
      const control = element as HTMLElement & { disabled?: boolean };
      if (control.disabled || element.matches(':disabled') || element.tabIndex < 0) return false;
      const style = getComputedStyle(element);
      return style.display !== 'none' && style.visibility !== 'hidden' && element.getClientRects().length > 0;
    });
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape' && dismissOnEscape && !busy) {
      event.preventDefault();
      event.stopPropagation();
      onCancel();
      return;
    }
    if (event.key !== 'Tab') return;
    const focusables = focusableDescendants();
    if (!focusables.length) {
      event.preventDefault();
      dialog.focus();
      return;
    }
    const index = focusables.indexOf(document.activeElement as HTMLElement);
    const next = event.shiftKey
      ? (index <= 0 ? focusables.length - 1 : index - 1)
      : (index < 0 || index === focusables.length - 1 ? 0 : index + 1);
    event.preventDefault();
    focusables[next]?.focus();
  }
</script>

<div class="stack-confirm-backdrop" role="presentation">
  <button class="stack-confirm-hitbox" type="button" aria-label="Dismiss confirmation dialog" disabled={busy || !dismissOnBackdrop} on:click={cancel}></button>
  <div bind:this={dialog} class="stack-confirm-dialog" role="alertdialog" aria-modal="true" tabindex="-1" aria-labelledby={`${id}-title`} aria-describedby={`${id}-message`} aria-busy={busy} on:keydown={handleKeydown}>
    <header><span aria-hidden="true"></span><h2 id={`${id}-title`}>{title}</h2></header>
    {#if message}<p id={`${id}-message`}>{message}</p>{:else}<div id={`${id}-message`}><slot /></div>{/if}
    {#if message}<slot />{/if}
    <div class="stack-confirm-actions">
      <button bind:this={cancelButton} type="button" disabled={busy} on:click={cancel}>{cancelLabel}</button>
      <button bind:this={confirmButton} type="button" class:danger={tone === 'danger'} disabled={busy} on:click={onConfirm}>{confirmLabel}</button>
    </div>
  </div>
</div>

<style>
  .stack-confirm-backdrop { background: color-mix(in srgb, #05080e 66%, transparent); display: grid; inset: 0; padding: 12px; place-items: center; position: fixed; z-index: 100; }
  .stack-confirm-hitbox { background: transparent; border: 0; inset: 0; position: absolute; }
  .stack-confirm-dialog { background: color-mix(in srgb, var(--js-color-surface-raised) 95%, #101827); border: 1px solid color-mix(in srgb, var(--js-color-border) 72%, var(--js-color-accent-border)); border-radius: var(--js-radius-md); box-shadow: var(--js-shadow-raised); color: var(--js-color-text); display: grid; gap: 12px; max-height: calc(100vh - 24px); max-width: min(30rem, calc(100vw - 24px)); overflow: auto; padding: 14px; position: relative; width: 100%; z-index: 1; }
  header { align-items: center; display: grid; gap: 9px; grid-template-columns: 4px 1fr; }
  header span { align-self: stretch; background: var(--js-color-accent); border-radius: 999px; box-shadow: 0 0 16px color-mix(in srgb, var(--js-color-accent) 45%, transparent); }
  h2, p { margin: 0; } h2 { color: var(--js-color-text-strong); font-size: .88rem; } p { color: var(--js-color-text-muted); font-size: .72rem; line-height: 1.45; }
  .stack-confirm-actions { display: flex; gap: 8px; justify-content: flex-end; }
  .stack-confirm-actions button { background: var(--js-bg-control); border: 1px solid var(--js-color-border); border-radius: var(--js-radius-sm); color: var(--js-color-text); font: inherit; font-size: .7rem; min-height: 30px; padding: 0 12px; }
  .stack-confirm-actions button:not(:disabled):hover, .stack-confirm-actions button:focus-visible { border-color: var(--js-color-accent-border); box-shadow: var(--js-focus-ring); outline: 0; }
  .stack-confirm-actions .danger { background: var(--js-color-error); border-color: var(--js-color-error-border); color: var(--js-color-text-strong); }
</style>
