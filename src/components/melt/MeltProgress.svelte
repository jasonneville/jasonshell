<script lang="ts">
  import { Progress } from 'melt/builders';

  let className = '';

  export let value = 0;
  export let max = 100;
  export let label = '';
  export let tone: 'cpu' | 'memory' | 'gpu' = 'cpu';
  export { className as class };

  const progress = new Progress({
    value: () => value,
    max: () => max
  });
</script>

<span
  {...progress.root}
  class={`melt-progress ${tone} ${className}`.trim()}
  aria-label={label}
>
  <span {...progress.progress} class="melt-progress-fill"></span>
</span>

<style>
  .melt-progress {
    background: var(--js-color-surface-overlay);
    border-radius: 999px;
    display: block;
    height: 0.28rem;
    overflow: hidden;
    position: relative;
    width: 100%;
  }

  .melt-progress-fill {
    background: var(--js-color-accent);
    border-radius: inherit;
    display: block;
    height: 100%;
    transform: translateX(var(--neg-progress));
  }

  .melt-progress.memory .melt-progress-fill {
    background: var(--js-color-warning-border);
  }

  .melt-progress.gpu .melt-progress-fill {
    background: var(--js-color-success-border);
  }
</style>
