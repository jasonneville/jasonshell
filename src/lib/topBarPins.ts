import type { EventTarget } from '@tauri-apps/api/event';

export type TopBarPinLike = {
  path: string;
};

export function topBarWebviewWindowEventTarget(label = 'top-bar'): EventTarget {
  return { kind: 'WebviewWindow', label };
}

export function findAddedPinPath(currentPins: TopBarPinLike[], nextPins: TopBarPinLike[]) {
  if (nextPins.length <= currentPins.length) {
    return null;
  }

  const currentPaths = new Set(currentPins.map((pin) => pin.path.toLowerCase()));
  for (let index = nextPins.length - 1; index >= 0; index -= 1) {
    const candidate = nextPins[index];
    if (!currentPaths.has(candidate.path.toLowerCase())) {
      return candidate.path;
    }
  }

  return null;
}

export function stackPinRevealPath(
  currentPins: TopBarPinLike[],
  nextPins: TopBarPinLike[],
  pendingVisiblePinPath: string | null,
  allowDetectedAdd: boolean
) {
  return pendingVisiblePinPath ?? (allowDetectedAdd ? findAddedPinPath(currentPins, nextPins) : null);
}

export function reorderPinnedFolders<T extends TopBarPinLike>(pins: T[], sourcePath: string, toIndex: number): T[] {
  const sourceIndex = pins.findIndex((pin) => pin.path === sourcePath);
  const clampedIndex = Math.max(0, Math.min(pins.length - 1, toIndex));
  if (sourceIndex < 0 || sourceIndex === clampedIndex) {
    return pins;
  }

  const nextPins = [...pins];
  const [pin] = nextPins.splice(sourceIndex, 1);
  nextPins.splice(clampedIndex, 0, pin);
  return nextPins;
}
