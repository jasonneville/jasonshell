<script lang="ts">
  import { MATERIAL_SYMBOL_ICON_PATHS, type MaterialSymbolIconName } from './materialSymbolIcons';

  export let name: MaterialSymbolIconName;
  export let decorative = true;
</script>

<svg
  class="material-symbol-icon"
  viewBox="0 -960 960 960"
  fill="currentColor"
  focusable="false"
  role={decorative ? undefined : 'img'}
  aria-hidden={decorative ? 'true' : undefined}
  aria-label={decorative ? undefined : name}
  xmlns="http://www.w3.org/2000/svg"
>
  <path d={MATERIAL_SYMBOL_ICON_PATHS[name] as unknown as string} />
</svg>

<style>
  .material-symbol-icon {
    display: inline-block;
    flex: 0 0 auto;
    width: 1rem;
    height: 1rem;
    vertical-align: middle;
    line-height: 0;
    pointer-events: none;
  }
</style>
