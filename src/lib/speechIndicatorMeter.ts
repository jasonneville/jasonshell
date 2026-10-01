const THRESHOLD = 0.035;
const HOLD_MS = 300;

export type SpeechIndicatorMeterSnapshot = {
  showBars: boolean;
  level: number;
};

export function createSpeechIndicatorMeter() {
  let level = 0;
  let audibleUntil = 0;

  function snapshot(nowMs: number): SpeechIndicatorMeterSnapshot {
    if (nowMs >= audibleUntil) return { showBars: false, level: 0 };
    return { showBars: true, level };
  }

  return {
    update(inputLevel: number, nowMs: number): boolean {
      const normalized = Number.isFinite(inputLevel) ? Math.min(1, Math.max(0, inputLevel)) : 0;
      if (normalized < THRESHOLD) return false;

      level = normalized;
      audibleUntil = nowMs + HOLD_MS;
      return true;
    },
    snapshot,
    reset(): void {
      level = 0;
      audibleUntil = 0;
    }
  };
}
