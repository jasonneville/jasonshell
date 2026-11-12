<script lang="ts">
  import MeltActionButton from './melt/MeltActionButton.svelte';

  let { label, value, resolved, onChange, onReset, onValidityChange = () => {} }: {
    label: string;
    value: string;
    resolved: string;
    onChange: (value: string) => void;
    onReset: () => void;
    onValidityChange?: (valid: boolean) => void;
  } = $props();
  const fieldId = $props.id();

  let intermediate = $state<string | null>(null);
  let invalid = $state(false);
  let alphaIntermediate = $state<string | null>(null);
  let alphaInvalid = $state(false);
  const parts = $derived(colorParts(resolved));
  const displayedHex = $derived(intermediate ?? parts.hex);

  function colorParts(color: string): { hex: string; alpha: number } {
    const hex = color.match(/^#([\da-f]{6})([\da-f]{2})?$/i);
    if (hex) return { hex: `#${hex[1]}`, alpha: hex[2] ? parseInt(hex[2], 16) / 255 : 1 };
    const rgb = color.match(/^rgba?\(\s*([\d.]+)[,\s]+([\d.]+)[,\s]+([\d.]+)(?:\s*[,/]\s*([\d.]+))?\s*\)$/);
    if (rgb) return {
      hex: '#' + rgb.slice(1, 4).map((part) => Math.round(Number(part)).toString(16).padStart(2, '0')).join(''),
      alpha: rgb[4] === undefined ? 1 : Number(rgb[4])
    };
    return { hex: '#000000', alpha: 1 };
  }

  function emit(hex: string, alpha: number) {
    intermediate = null;
    invalid = false;
    alphaIntermediate = null;
    alphaInvalid = false;
    onValidityChange(true);
    onChange(hex + Math.round(alpha * 255).toString(16).padStart(2, '0'));
  }

  function editHex(input: string) {
    intermediate = input;
    const normalized = input.trim();
    const valid = /^#(?:[\da-f]{3}|[\da-f]{6}|[\da-f]{8})$/i.test(normalized);
    invalid = !valid;
    onValidityChange(valid && !alphaInvalid);
    if (!valid) return;
    const expanded = normalized.length === 4
      ? '#' + [...normalized.slice(1)].map((digit) => digit + digit).join('')
      : normalized;
    onChange(expanded.length === 9 ? expanded : expanded + Math.round(parts.alpha * 255).toString(16).padStart(2, '0'));
  }

  function reset() {
    intermediate = null;
    invalid = false;
    alphaIntermediate = null;
    alphaInvalid = false;
    onValidityChange(true);
    onReset();
  }
</script>

<div class="theme-color-field">
  <div class="theme-color-caption">
    <strong>{label}</strong>
    {#if value.includes('var(')}<small>Linked · override to unlink</small>{/if}
  </div>
  <div class="theme-color-controls">
    <input type="color" aria-label={`${label} picker`} value={parts.hex} oninput={(event) => emit(event.currentTarget.value, parts.alpha)} />
    <input class="theme-hex" type="text" aria-label={`${label} hex`} aria-invalid={invalid} aria-describedby={invalid ? `${fieldId}-hex-error` : undefined} value={displayedHex} spellcheck="false" maxlength="9" oninput={(event) => editHex(event.currentTarget.value)} />
    <label class="theme-alpha"><span>Alpha</span><input type="number" aria-label={`${label} alpha`} aria-invalid={alphaInvalid} aria-describedby={alphaInvalid ? `${fieldId}-alpha-error` : undefined} min="0" max="100" step="1" value={alphaIntermediate ?? Math.round(parts.alpha * 100)} oninput={(event) => {
      const alpha = event.currentTarget.valueAsNumber;
      alphaIntermediate = event.currentTarget.value;
      alphaInvalid = !Number.isFinite(alpha) || alpha < 0 || alpha > 100;
      onValidityChange(!alphaInvalid && !invalid);
      if (!alphaInvalid) {
        alphaIntermediate = null;
        onChange(parts.hex + Math.round(alpha / 100 * 255).toString(16).padStart(2, '0'));
      }
    }} /><span>%</span></label>
    <MeltActionButton ariaLabel={`Reset ${label}`} onClick={reset}>Reset</MeltActionButton>
  </div>
  {#if invalid}<small id={`${fieldId}-hex-error`} class="theme-field-error">Use #RGB, #RRGGBB, or #RRGGBBAA. Preview keeps the last valid color.</small>{/if}
  {#if alphaInvalid}<small id={`${fieldId}-alpha-error`} class="theme-field-error">Alpha must be between 0 and 100. Preview keeps the last valid color.</small>{/if}
</div>

<style>
  .theme-color-field { display: grid; gap: .28rem; min-width: 0; }
  .theme-color-caption { display: flex; flex-wrap: wrap; align-items: baseline; gap: .4rem; }
  strong { font-size: .7rem; font-weight: 700; }
  small { color: #abb9ce; font-size: .6rem; }
  .theme-color-controls { display: grid; grid-template-columns: 2rem minmax(4.5rem, 1fr) minmax(6.3rem, .7fr) auto; gap: .35rem; align-items: center; }
  input { background: #101722; border: 1px solid #58677e; border-radius: var(--js-radius-sm); color: #f1f5ff; min-width: 0; min-height: 1.9rem; width: 100%; font: inherit; font-size: .7rem; padding: .2rem .35rem; }
  input[type='color'] { padding: .15rem; cursor: pointer; }
  .theme-hex { font-family: var(--js-font-mono, monospace); }
  .theme-alpha { display: flex; align-items: center; gap: .18rem; font-size: .6rem; }
  .theme-alpha input { flex-shrink: 0; width: 3.5rem; }
  .theme-field-error { color: #ffc8cf; }
  input:focus-visible { outline: 2px solid #b5d0ff; outline-offset: 2px; }
  input[aria-invalid='true'] { border-color: #ffadb9; }
  @media (max-width: 340px) {
    .theme-color-controls { grid-template-columns: 2rem minmax(0, 1fr) auto; }
    .theme-hex { grid-column: 2 / -1; }
    .theme-alpha { grid-column: 1 / 3; }
    .theme-color-controls :global(button) { grid-column: 3; }
  }
</style>
