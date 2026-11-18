<script lang="ts">
  import MaterialSymbolIcon from './icons/MaterialSymbolIcon.svelte';

  let { onClose }: { onClose: () => void } = $props();
</script>

<button class="terminal-search-close" type="button" aria-label="Close terminal search" onclick={onClose}>
  <MaterialSymbolIcon name="close" />
</button>

<style>
  .terminal-search-close {
    background: var(--js-color-surface-sunken);
    border: 1px solid var(--js-color-border);
    border-radius: var(--js-radius-sm);
    color: var(--js-color-text);
    display: grid;
    flex: 0 0 auto;
    height: 24px;
    min-height: 24px;
    padding: 0;
    place-items: center;
    width: 24px;
  }

  .terminal-search-close:hover {
    background: var(--js-color-control-hover);
  }

  .terminal-search-close:focus-visible {
    outline: 2px solid var(--js-color-accent);
    outline-offset: 1px;
    box-shadow: none;
  }

  @media (forced-colors: active) {
    .terminal-search-close:focus-visible {
      outline-color: Highlight;
    }
  }
</style>
