<script lang="ts">
  import MaterialSymbolIcon from './icons/MaterialSymbolIcon.svelte';
  import type { MaterialSymbolIconName } from './icons/materialSymbolIcons';

  export let icon: MaterialSymbolIconName | null = null;
  export let disabled = false;
  export let destructive = false;
  export let ariaLabel: string | undefined = undefined;
  export let onClick: ((event: MouseEvent) => void) | undefined = undefined;
  export let element: HTMLButtonElement | null = null;
</script>

<button
  class:context-menu-item--destructive={destructive}
  class="context-menu-item"
  type="button"
  role="menuitem"
  {disabled}
  aria-label={ariaLabel}
  bind:this={element}
  on:click={onClick}
>
  {#if icon}
    <span class="context-menu-icon"><MaterialSymbolIcon name={icon} /></span>
  {:else}
    <span class="context-menu-icon-placeholder" aria-hidden="true"></span>
  {/if}
  <slot />
</button>
