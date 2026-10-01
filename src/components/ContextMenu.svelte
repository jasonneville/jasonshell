<script lang="ts">
  import MaterialSymbolIcon from './icons/MaterialSymbolIcon.svelte';

  export let className = '';
  export let ariaLabel: string | undefined = undefined;
  export let tabindex: number | undefined = -1;
  export let style = '';
  export let element: HTMLDivElement | null = null;

  // Keep the icon primitive in the shared menu module so all menu consumers use
  // the same visual contract. Individual rows render their own leading icon.
  void MaterialSymbolIcon;
</script>

<div
  class={`js-context-menu ${className}`}
  role="menu"
  aria-label={ariaLabel}
  {tabindex}
  {style}
  bind:this={element}
  on:click
  on:contextmenu
  on:keydown
  on:pointerdown
  on:scroll
>
  <slot />
</div>

<style>
  :global(.js-context-menu) {
    background: color-mix(in srgb, var(--js-color-surface-overlay) 73%, transparent);
    backdrop-filter: blur(8px);
    border: 1px solid color-mix(in srgb, var(--js-color-border) 86%, transparent);
    border-radius: 12px;
    box-shadow: var(--js-shadow-raised);
    color: var(--js-color-text);
    min-width: 8rem;
    padding: 4px;
    position: fixed;
    transform-origin: var(--context-menu-origin, top left);
    animation: js-context-menu-in 150ms ease-out both;
    z-index: 100;
  }

  :global(.js-context-menu .context-menu-item) {
    align-items: center;
    background: transparent;
    border: 0;
    border-radius: 8px;
    color: inherit;
    cursor: pointer;
    display: grid;
    font: inherit;
    gap: 8px;
    grid-template-columns: 16px minmax(0, 1fr);
    justify-content: start;
    min-height: 30px;
    padding: 4px 8px;
    text-align: left;
    width: 100%;
  }

  :global(.js-context-menu .context-menu-item:hover),
  :global(.js-context-menu .context-menu-item:focus-visible) {
    background: var(--js-color-control-hover);
    outline: 0;
  }

  :global(.js-context-menu .context-menu-item:focus-visible) { box-shadow: var(--js-focus-ring); }
  :global(.js-context-menu .context-menu-item:disabled) { cursor: default; opacity: .5; pointer-events: none; }
  :global(.js-context-menu .context-menu-item--destructive) { color: var(--js-color-danger, #f08080); }
  :global(.js-context-menu .context-menu-icon),
  :global(.js-context-menu .context-menu-icon-placeholder) { color: var(--js-color-text-muted); height: 16px; pointer-events: none; width: 16px; }
  :global(.js-context-menu .context-menu-icon) { align-items: center; display: flex; justify-content: center; line-height: 0; }
  :global(.js-context-menu .context-menu-icon-placeholder) { display: block; }
  :global(.js-context-menu .context-menu-separator) { background: var(--js-color-border-soft, var(--js-color-border)); border: 0; height: 1px; margin: 4px -1px; }

  @keyframes js-context-menu-in {
    from { transform: scale(.95); }
    to { transform: scale(1); }
  }
</style>
