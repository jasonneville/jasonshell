<script lang="ts">
  import { Toggle } from 'melt/builders';

  let className = '';

  export let checked = false;
  export let disabled = false;
  export let label = '';
  export let ariaLabel = '';
  export let onChange: (checked: boolean) => void = () => undefined;
  export { className as class };

  const toggle = new Toggle({
    value: () => checked,
    disabled: () => disabled,
    onValueChange: onChange
  });
</script>

<button
  {...toggle.trigger}
  class={`melt-toggle ${className}`.trim()}
  type="button"
  aria-label={ariaLabel || label}
>
  <span class="melt-toggle-track" aria-hidden="true">
    <span class="melt-toggle-thumb"></span>
  </span>
  <span class="melt-toggle-label">
    <slot>{label}</slot>
  </span>
</button>

<style>
  .melt-toggle {
    align-items: center;
    background: var(--js-color-surface-sunken);
    border: 1px solid var(--js-color-border-soft);
    border-radius: var(--js-radius-sm);
    color: var(--js-color-text);
    display: flex;
    gap: 0.42rem;
    min-height: 2.15rem;
    min-width: 0;
    padding: 0.42rem;
    text-align: left;
    width: 100%;
  }

  .melt-toggle:hover:not(:disabled) {
    background: var(--js-color-control-hover);
    border-color: var(--js-color-accent-border);
  }

  .melt-toggle[data-checked] {
    background: var(--js-color-selected);
    border-color: var(--js-color-accent-border);
  }

  .melt-toggle:disabled {
    cursor: default;
    opacity: 0.52;
  }

  .melt-toggle-track {
    align-items: center;
    background: var(--js-color-control);
    border: 1px solid var(--js-color-border);
    border-radius: 999px;
    display: inline-flex;
    flex: 0 0 auto;
    height: 1rem;
    padding: 0.1rem;
    width: 1.85rem;
  }

  .melt-toggle-thumb {
    background: var(--js-color-text-muted);
    border-radius: 999px;
    height: 0.68rem;
    transform: translateX(0);
    transition:
      background var(--js-motion-fast) ease,
      transform var(--js-motion-fast) ease;
    width: 0.68rem;
  }

  .melt-toggle[data-checked] .melt-toggle-track {
    background: var(--js-color-accent-soft);
    border-color: var(--js-color-accent-border);
  }

  .melt-toggle[data-checked] .melt-toggle-thumb {
    background: var(--js-color-accent);
    transform: translateX(0.82rem);
  }

  .melt-toggle-label {
    color: var(--js-color-text);
    font-size: 0.62rem;
    font-weight: 750;
    min-width: 0;
  }
</style>
