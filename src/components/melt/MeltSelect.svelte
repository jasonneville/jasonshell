<script lang="ts">
  type MeltSelectOption = {
    value: string;
    label: string;
  };

  import { Select } from 'melt/builders';

  let className = '';

  export let label = '';
  export let value = '';
  export let options: MeltSelectOption[] = [];
  export let placeholder = 'Select';
  export let onChange: (value: string) => void = () => undefined;
  export { className as class };

  const select = new Select<string>({
    value: () => value,
    onValueChange: (nextValue) => {
      if (typeof nextValue === 'string') {
        onChange(nextValue);
      }
    },
    sameWidth: true,
    scrollAlignment: 'nearest'
  });

  $: selectedLabel = options.find((option) => option.value === value)?.label ?? placeholder;
</script>

<div class={`melt-select-field ${className}`.trim()}>
  {#if label}
    <label {...select.label} class="melt-select-label">{label}</label>
  {/if}
  <button {...select.trigger} class="melt-select-trigger" type="button" aria-label={label || placeholder}>
    <span>{selectedLabel}</span>
    <span class="melt-select-caret" aria-hidden="true">⌄</span>
  </button>
  <div {...select.content} class="melt-select-content">
    {#each options as option (option.value)}
      <div
        {...select.getOption(option.value, option.label)}
        id={select.getOptionId(option.value)}
        class="melt-select-option"
      >
        <span>{option.label}</span>
        {#if option.value === value}
          <span aria-hidden="true">✓</span>
        {/if}
      </div>
    {/each}
  </div>
</div>

<style>
  .melt-select-field {
    display: grid;
    gap: 0.26rem;
    min-width: 0;
  }

  .melt-select-label {
    color: var(--js-color-text-muted);
    font-size: 0.62rem;
    font-weight: 750;
  }

  .melt-select-trigger {
    align-items: center;
    background: var(--js-color-surface-sunken);
    border: 1px solid var(--js-color-border);
    border-radius: var(--js-radius-sm);
    color: var(--js-color-text);
    display: inline-flex;
    font: inherit;
    font-size: 0.7rem;
    justify-content: space-between;
    min-height: 1.65rem;
    min-width: 0;
    padding: 0 0.48rem;
    text-align: left;
    width: 100%;
  }

  .melt-select-trigger:hover,
  .melt-select-trigger[data-open] {
    background: var(--js-color-control-hover);
    border-color: var(--js-color-accent-border);
  }

  .melt-select-trigger span:first-child {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .melt-select-caret {
    color: var(--js-color-text-muted);
    flex: 0 0 auto;
    font-size: 0.82rem;
    line-height: 1;
    margin-left: 0.45rem;
  }

  .melt-select-content {
    background: var(--js-color-surface-raised);
    border: 1px solid var(--js-color-border);
    border-radius: var(--js-radius-md);
    box-shadow: var(--js-shadow-raised);
    color: var(--js-color-text);
    display: grid;
    gap: 0.12rem;
    margin: 0;
    max-height: min(16rem, var(--melt-popover-available-height, 16rem));
    min-width: var(--melt-invoker-width, 12rem);
    overflow: auto;
    padding: 0.24rem;
    z-index: 50;
  }

  .melt-select-content:not([data-open]) {
    display: none;
  }

  .melt-select-option {
    align-items: center;
    border-radius: var(--js-radius-sm);
    color: var(--js-color-text);
    display: flex;
    font-size: 0.7rem;
    gap: 0.6rem;
    justify-content: space-between;
    min-height: 1.65rem;
    padding: 0 0.44rem;
  }

  .melt-select-option[data-highlighted],
  .melt-select-option:hover {
    background: var(--js-color-control-hover);
  }

  .melt-select-option[aria-selected='true'] {
    background: var(--js-color-selected);
    color: var(--js-color-text-strong);
  }
</style>
