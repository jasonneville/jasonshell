/** Presentation only: the speech meter still owns threshold, hold and nonce. */
export function createSpeechIndicatorFill(paint: (level: number) => void) {
  const reduced = window.matchMedia('(prefers-reduced-motion: reduce)');
  let displayed = 0;
  let target = 0;
  let origin = 0;
  let started = 0;
  let duration = 100;
  let frame = 0;
  let disposed = false;

  function stop() {
    if (frame) cancelAnimationFrame(frame);
    frame = 0;
  }
  function show(level: number) {
    displayed = level;
    paint(level);
  }
  function draw(now: number) {
    frame = 0;
    if (disposed || document.hidden) return;
    const progress = Math.min(1, Math.max(0, (now - started) / duration));
    // Ease out without overshoot; assign the exact target on the final frame.
    show(progress === 1 ? target : origin + (target - origin) * (1 - (1 - progress) ** 2));
    if (progress < 1) frame = requestAnimationFrame(draw);
  }
  function resume() {
    stop();
    if (disposed) return;
    if (reduced.matches) { show(target); return; }
    if (document.hidden || displayed === target) return;
    origin = displayed;
    started = performance.now();
    duration = target > displayed ? 100 : 180;
    frame = requestAnimationFrame(draw);
  }
  reduced.addEventListener('change', resume);
  document.addEventListener('visibilitychange', resume);
  return {
    setTarget(level: number) {
      if (disposed) return;
      const next = Number.isFinite(level) ? Math.min(1, Math.max(0, level)) : 0;
      // Repeated held samples must not restart easing or defer convergence.
      if (next === target) return;
      target = next;
      resume();
    },
    reset() {
      if (disposed) return;
      stop();
      target = 0;
      show(0);
    },
    dispose() {
      disposed = true;
      stop();
      reduced.removeEventListener('change', resume);
      document.removeEventListener('visibilitychange', resume);
    }
  };
}
