<script lang="ts">
  let { text }: { text: string } = $props();
  let viewport: HTMLSpanElement;
  let content: HTMLSpanElement;

  const ENGAGEMENT_DELAY_MS = 600;
  const TRAVEL_PIXELS_PER_SECOND = 40;
  const END_HOLD_MS = 2000;
  const ORIGIN_HOLD_MS = 3000;

  function attachTicker(clip: HTMLSpanElement, label: HTMLSpanElement) {
    const button = clip.closest('button');
    if (!button) return;

    const motionPreference = window.matchMedia('(prefers-reduced-motion: reduce)');
    let hovered = button.matches(':hover');
    let focused = document.activeElement === button;
    let animation: Animation | null = null;
    let revealTimer: ReturnType<typeof setTimeout> | undefined;
    let animatedOverflow = 0;

    function reset() {
      clearTimeout(revealTimer);
      revealTimer = undefined;
      animation?.cancel();
      animation = null;
      animatedOverflow = 0;
      clip.classList.remove('moving');
    }

    function update() {
      if ((!hovered && !focused) || motionPreference.matches) {
        reset();
        return;
      }

      // Keep fractional intrinsic width stable across inline/inline-block.
      // scrollWidth rounds up and would restart motion after the display change.
      const titleWidth = label.getBoundingClientRect().width;
      const overflow = Math.max(0, titleWidth - clip.clientWidth);
      if (overflow <= 0) {
        reset();
        return;
      }
      if (animation && animatedOverflow === overflow) return;
      reset();

      const travelMs = overflow / TRAVEL_PIXELS_PER_SECOND * 1000;
      const duration = travelMs * 2 + END_HOLD_MS + ORIGIN_HOLD_MS;
      const origin = 'translateX(0px)';
      const end = `translateX(-${overflow}px)`;
      animatedOverflow = overflow;
      animation = label.animate([
        { transform: origin, offset: 0 },
        { transform: end, offset: travelMs / duration },
        { transform: end, offset: (travelMs + END_HOLD_MS) / duration },
        { transform: origin, offset: (travelMs * 2 + END_HOLD_MS) / duration },
        { transform: origin, offset: 1 }
      ], { delay: ENGAGEMENT_DELAY_MS, duration, iterations: Infinity, easing: 'linear' });
      // Preserve genuine ellipsis during the initial delay, then expose only
      // the moving text inside the same fixed-width clipping viewport.
      revealTimer = setTimeout(() => {
        clip.classList.add('moving');
        revealTimer = undefined;
      }, ENGAGEMENT_DELAY_MS);
    }

    function enter() { hovered = true; update(); }
    function leave() { hovered = false; update(); }
    function focus() { focused = true; update(); }
    function blur() { focused = false; update(); }

    button.addEventListener('mouseenter', enter);
    button.addEventListener('mouseleave', leave);
    button.addEventListener('focus', focus);
    button.addEventListener('blur', blur);
    motionPreference.addEventListener('change', update);
    const observer = new ResizeObserver(update);
    observer.observe(clip);
    observer.observe(label);
    update();

    return () => {
      reset();
      observer.disconnect();
      button.removeEventListener('mouseenter', enter);
      button.removeEventListener('mouseleave', leave);
      button.removeEventListener('focus', focus);
      button.removeEventListener('blur', blur);
      motionPreference.removeEventListener('change', update);
    };
  }

  $effect(() => {
    text;
    return attachTicker(viewport, content);
  });
</script>

<span class="overflow-ticker" bind:this={viewport}><span class="overflow-ticker-text" bind:this={content}>{text}</span></span>

<style>
  .overflow-ticker {
    display: block;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .overflow-ticker-text { display: inline; }
  .overflow-ticker:global(.moving) { text-overflow: clip; }
  :global(.moving) .overflow-ticker-text {
    display: inline-block;
    width: max-content;
  }
</style>
