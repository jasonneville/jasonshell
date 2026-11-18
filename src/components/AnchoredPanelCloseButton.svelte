<script lang="ts">
  import MaterialSymbolIcon from './icons/MaterialSymbolIcon.svelte';
  import MeltActionButton from './melt/MeltActionButton.svelte';

  let { label, onClick }: { label: string; onClick: () => void } = $props();
</script>

<MeltActionButton class="anchored-panel-close" ariaLabel={label} tooltip={label} {onClick}>
  <MaterialSymbolIcon name="close" />
</MeltActionButton>

<style>
  :global(.anchored-panel-close) {
    align-items: center;
    background: var(--js-color-surface-sunken);
    border: 1px solid var(--js-color-border-soft);
    border-radius: var(--js-radius-sm);
    color: var(--js-color-text);
    display: inline-flex;
    flex: 0 0 auto;
    height: max(24px, 1.7rem);
    justify-content: center;
    padding: 0;
    width: max(24px, 1.7rem);
  }

  :global(.anchored-panel-close:hover) {
    background: var(--js-color-control-hover);
  }

  @media (forced-colors: active) {
    :global(.anchored-panel-close) {
      border-color: CanvasText;
    }

    :global(.anchored-panel-close:focus-visible) {
      outline: 2px solid Highlight;
      outline-offset: 1px;
    }
  }
</style>
