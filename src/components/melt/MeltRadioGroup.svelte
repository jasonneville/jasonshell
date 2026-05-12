<script lang="ts">
  type MeltRadioOption = {
    value: string;
    label: string;
  };

  import { RadioGroup } from 'melt/builders';

  let className = '';

  export let label = '';
  export let value = '';
  export let options: MeltRadioOption[] = [];
  export let onChange: (value: string) => void = () => undefined;
  export { className as class };

  $: selectedValue = options.some((option) => option.value === value) ? value : '';

  const radioGroup = new RadioGroup({
    value: () => selectedValue,
    orientation: 'horizontal',
    loop: true,
    selectWhenFocused: true,
    onValueChange: (nextValue) => {
      if (nextValue) {
        onChange(nextValue);
      }
    }
  });
</script>

<div class={`melt-radio-field ${className}`.trim()}>
  {#if label}
    <label {...radioGroup.label} class="melt-radio-label">{label}</label>
  {/if}
  <input {...radioGroup.hiddenInput} />
  <div {...radioGroup.root} class="melt-radio-group" aria-label={label}>
    {#each options as option (option.value)}
      {@const radioItem = radioGroup.getItem(option.value)}
      <button
        {...radioItem.attrs}
        class="melt-radio-item"
        type="button"
        aria-label={option.label}
      >
        <span>{option.label}</span>
      </button>
    {/each}
  </div>
</div>

<style>
  .melt-radio-field {
    display: grid;
    gap: 0.3rem;
    min-width: 0;
  }

  .melt-radio-label {
    color: var(--js-color-text-muted);
    font-size: 0.62rem;
    font-weight: 750;
  }

  .melt-radio-group {
    display: flex;
    flex-wrap: wrap;
    gap: 0.35rem;
    min-width: 0;
  }

  .melt-radio-item {
    background: var(--js-bg-control);
    border: 1px solid var(--js-color-border);
    border-radius: var(--js-radius-sm);
    color: var(--js-color-text);
    font: inherit;
    font-size: 0.68rem;
    font-weight: 800;
    min-height: 1.55rem;
    min-width: 0;
    padding: 0 0.55rem;
  }

  .melt-radio-item:hover,
  .melt-radio-item:focus-visible {
    background: var(--js-color-control-hover);
    border-color: var(--js-color-accent-border);
  }

  .melt-radio-item[data-state='checked'] {
    background: var(--js-color-selected);
    border-color: var(--js-color-accent-border);
    color: var(--js-color-text-strong);
  }

  .melt-radio-item span {
    display: block;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
