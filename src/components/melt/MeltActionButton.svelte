<script lang="ts">
  import { Tooltip } from 'melt/builders';

  let className = '';

  export let ariaHaspopup: 'dialog' | 'menu' | 'listbox' | 'tree' | 'grid' | true | false | undefined = undefined;
  export let ariaLabel: string | undefined = undefined;
  export let ariaSort: 'none' | 'ascending' | 'descending' | 'other' | undefined = undefined;
  export let ariaColindex: number | undefined = undefined;
  export let ariaCurrent: 'page' | 'step' | 'location' | 'date' | 'time' | true | false | undefined = undefined;
  export let ariaSelected: boolean | 'true' | 'false' | undefined = undefined;
  export let ariaPressed: boolean | 'true' | 'false' | 'mixed' | undefined = undefined;
  export let ariaDisabled: boolean | 'true' | 'false' | undefined = undefined;
  export let ariaControls: string | undefined = undefined;
  export let ariaExpanded: boolean | 'true' | 'false' | undefined = undefined;
  export let dataPath: string | undefined = undefined;
  export let disabled = false;
  export let draggable: boolean | 'true' | 'false' = false;
  export let name: string | undefined = undefined;
  export let role: string | undefined = undefined;
  export let style: string | undefined = undefined;
  export let title: string | undefined = undefined;
  export let tooltip = '';
  export let type: 'button' | 'submit' | 'reset' = 'button';
  export let value: string | number | undefined = undefined;
  export let onClick: (event: MouseEvent) => void = () => undefined;
  export let onContextMenu: (event: MouseEvent) => void = () => undefined;
  export let onDblClick: (event: MouseEvent) => void = () => undefined;
  export let onDragEnd: (event: DragEvent) => void = () => undefined;
  export let onDragOver: (event: DragEvent) => void = () => undefined;
  export let onDragStart: (event: DragEvent) => void = () => undefined;
  export let onDrop: (event: DragEvent) => void = () => undefined;
  export let onKeyDown: (event: KeyboardEvent) => void = () => undefined;
  export let onMouseDown: (event: MouseEvent) => void = () => undefined;
  export let onMouseEnter: (event: MouseEvent) => void = () => undefined;
  export let onMouseLeave: (event: MouseEvent) => void = () => undefined;
  export let onPointerCancel: (event: PointerEvent) => void = () => undefined;
  export let onPointerDown: (event: PointerEvent) => void = () => undefined;
  export let onPointerEnter: (event: PointerEvent) => void = () => undefined;
  export let onPointerLeave: (event: PointerEvent) => void = () => undefined;
  export let onPointerMove: (event: PointerEvent) => void = () => undefined;
  export let onPointerUp: (event: PointerEvent) => void = () => undefined;
  export let onLostPointerCapture: (event: PointerEvent) => void = () => undefined;
  export { className as class };

  const actionTooltip = new Tooltip({
    closeOnPointerDown: true,
    openDelay: 500,
    closeDelay: 40,
    floatingConfig: {
      computePosition: {
        placement: 'bottom'
      }
    }
  });

  $: tooltipText = tooltip || title || ariaLabel;
  $: tooltipTrigger = tooltipText ? actionTooltip.trigger : {};
</script>

<button
  {...tooltipTrigger}
  class={className}
  type={type}
  role={role}
  aria-haspopup={ariaHaspopup}
  aria-label={ariaLabel}
  aria-sort={ariaSort}
  aria-colindex={ariaColindex}
  aria-current={ariaCurrent}
  aria-selected={ariaSelected}
  aria-pressed={ariaPressed}
  aria-disabled={ariaDisabled}
  aria-controls={ariaControls}
  aria-expanded={ariaExpanded}
  title={title}
  data-path={dataPath}
  disabled={disabled}
  draggable={draggable}
  name={name}
  style={style}
  value={value}
  on:click={onClick}
  on:contextmenu={onContextMenu}
  on:dblclick={onDblClick}
  on:dragstart={onDragStart}
  on:dragend={onDragEnd}
  on:dragover={onDragOver}
  on:drop={onDrop}
  on:keydown={onKeyDown}
  on:mousedown={onMouseDown}
  on:mouseenter={onMouseEnter}
  on:mouseleave={onMouseLeave}
  on:pointercancel={onPointerCancel}
  on:pointerdown={onPointerDown}
  on:pointerenter={onPointerEnter}
  on:pointerleave={onPointerLeave}
  on:pointermove={onPointerMove}
  on:pointerup={onPointerUp}
  on:lostpointercapture={onLostPointerCapture}
>
  <slot />
</button>



<style>
  .melt-action-button-tooltip {
    background: var(--js-color-surface-raised);
    border: 1px solid var(--js-color-border);
    border-radius: var(--js-radius-xs);
    box-shadow: var(--js-shadow-raised);
    color: var(--js-color-text);
    font-size: 0.62rem;
    font-weight: 650;
    max-width: min(18rem, calc(100vw - 0.5rem));
    overflow: hidden;
    padding: 0.18rem 0.36rem;
    text-overflow: ellipsis;
    white-space: nowrap;
    z-index: 100;
  }
</style>
