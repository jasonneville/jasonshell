export const TOP_BAR_CONTROL_IDS = ['terminal', 'command', 'tray', 'mic', 'sound', 'snip'] as const;
export type TopBarControlId = (typeof TOP_BAR_CONTROL_IDS)[number];

export const DEFAULT_TOP_BAR_CONTROL_ORDER: TopBarControlId[] = [...TOP_BAR_CONTROL_IDS];
export const TOP_BAR_CONTROL_ORDER_STORAGE_KEY = 'jasonshell:top-bar:control-order:v1';
export const TOP_BAR_CONTROL_DRAG_THRESHOLD_PX = 4;

export type TopBarControlRect = {
  key: string;
  left: number;
  width: number;
};

type ClosestButtonTarget = {
  closest(selector: string): unknown;
};

type PointerCaptureContainer = {
  contains(candidate: unknown): boolean;
};

export function resolveTopBarControlPointerCaptureTarget(
  target: unknown,
  container: PointerCaptureContainer
) {
  const button = target && typeof (target as ClosestButtonTarget).closest === 'function'
    ? (target as ClosestButtonTarget).closest('button')
    : null;
  return button && container.contains(button) ? button : container;
}

export function normalizeTopBarControlOrder(serialized: string | null): TopBarControlId[] {
  if (serialized === null) return [...DEFAULT_TOP_BAR_CONTROL_ORDER];

  let value: unknown;
  try {
    value = JSON.parse(serialized);
  } catch {
    return [...DEFAULT_TOP_BAR_CONTROL_ORDER];
  }
  if (!Array.isArray(value)) return [...DEFAULT_TOP_BAR_CONTROL_ORDER];

  const known = new Set<string>(TOP_BAR_CONTROL_IDS);
  const seen = new Set<string>();
  const normalized: TopBarControlId[] = [];
  for (const candidate of value) {
    if (typeof candidate === 'string' && known.has(candidate) && !seen.has(candidate)) {
      seen.add(candidate);
      normalized.push(candidate as TopBarControlId);
    }
  }
  for (const id of TOP_BAR_CONTROL_IDS) {
    if (!seen.has(id)) normalized.push(id);
  }
  return normalized;
}

export function topBarControlOrderFromDisplacement<T extends string>(
  sourceId: T,
  initialOrder: readonly T[],
  rects: readonly TopBarControlRect[],
  dragDeltaX: number
): readonly T[] {
  const sourceIndex = initialOrder.indexOf(sourceId);
  const rectsByKey = new Map(rects.map((rect) => [rect.key, rect]));
  const sourceRect = rectsByKey.get(sourceId);
  if (sourceIndex < 0 || !sourceRect || sourceRect.width <= 0 || dragDeltaX === 0) return initialOrder;

  const movedCenter = sourceRect.left + sourceRect.width / 2 + dragDeltaX;
  let destinationIndex = sourceIndex;
  if (dragDeltaX > 0) {
    for (let index = sourceIndex + 1; index < initialOrder.length; index += 1) {
      const rect = rectsByKey.get(initialOrder[index]);
      if (rect && rect.width > 0 && movedCenter > rect.left + rect.width / 2) destinationIndex = index;
    }
  } else {
    for (let index = sourceIndex - 1; index >= 0; index -= 1) {
      const rect = rectsByKey.get(initialOrder[index]);
      if (rect && rect.width > 0 && movedCenter < rect.left + rect.width / 2) destinationIndex = index;
    }
  }
  if (destinationIndex === sourceIndex) return initialOrder;

  const next = initialOrder.filter((id) => id !== sourceId);
  next.splice(destinationIndex, 0, sourceId);
  return next;
}

export function resolveTopBarControlPointerRelease(
  pendingControlId: TopBarControlId | null,
  dragStarted: boolean
) {
  return { suppressClickId: dragStarted ? pendingControlId : null };
}

export function shouldSuppressTopBarControlClick(
  pendingControlId: TopBarControlId | null,
  clickedControlId: TopBarControlId,
  event: { detail: number }
) {
  return pendingControlId === clickedControlId && event.detail !== 0;
}
