/** Monitor-local CSS geometry. Native code remains authoritative for display topology. */
export type Point = Readonly<{ x: number; y: number }>;
export type Bounds = Readonly<{ width: number; height: number }>;
export type Rect = Point & Bounds;
export type Pointer = Point & Readonly<{ pointerId: number; button: number; isPrimary: boolean }>;
export type SelectionState = Readonly<{ pointerId: number; start: Point; end: Point }>;

function validBounds(bounds: Bounds): boolean {
  return Number.isFinite(bounds.width) && Number.isFinite(bounds.height)
    && bounds.width > 0 && bounds.height > 0;
}

function clampPoint(point: Point, bounds: Bounds): Point | null {
  if (!validBounds(bounds) || !Number.isFinite(point.x) || !Number.isFinite(point.y)) return null;
  return { x: Math.min(bounds.width, Math.max(0, point.x)), y: Math.min(bounds.height, Math.max(0, point.y)) };
}

export function normalizeSelection(start: Point, end: Point, bounds: Bounds): Rect | null {
  const first = clampPoint(start, bounds);
  const last = clampPoint(end, bounds);
  if (!first || !last) return null;
  const width = Math.abs(last.x - first.x);
  const height = Math.abs(last.y - first.y);
  return width > 0 && height > 0
    ? { x: Math.min(first.x, last.x), y: Math.min(first.y, last.y), width, height } : null;
}

/** The renderer must ignore a second pointerdown while a state already exists. */
export function beginSelection(pointer: Pointer, bounds: Bounds): SelectionState | null {
  if (pointer.button !== 0 || !pointer.isPrimary || !Number.isSafeInteger(pointer.pointerId)) return null;
  const start = clampPoint(pointer, bounds);
  return start ? { pointerId: pointer.pointerId, start, end: start } : null;
}

export function moveSelection(state: SelectionState | null, pointer: Pointer, bounds: Bounds): SelectionState | null {
  if (!state || state.pointerId !== pointer.pointerId) return state;
  const end = clampPoint(pointer, bounds);
  return end ? { ...state, end } : state;
}

export function finishSelection(state: SelectionState | null, pointer: Pointer, bounds: Bounds): {
  state: SelectionState | null; selection: Rect | null;
} {
  if (!state || state.pointerId !== pointer.pointerId) return { state, selection: null };
  return { state: null, selection: normalizeSelection(state.start, pointer, bounds) };
}

export function cancelSelection(state: SelectionState | null, pointerId?: number): SelectionState | null {
  return pointerId === undefined || state?.pointerId === pointerId ? null : state;
}
