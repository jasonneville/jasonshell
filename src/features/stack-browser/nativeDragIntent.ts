export type StackRowDragIntent = Readonly<{
  pointerId: number;
  x: number;
  y: number;
}> | null;

type PointerStart = { pointerId: number; button: number; buttons: number; x: number; y: number };
type PointerMove = { pointerId: number; buttons: number; x: number; y: number };

export function beginStackRowDrag(event: PointerStart): StackRowDragIntent {
  return event.button === 0 && (event.buttons & 1) !== 0
    ? { pointerId: event.pointerId, x: event.x, y: event.y }
    : null;
}

export function moveStackRowDrag(intent: StackRowDragIntent, event: PointerMove) {
  return {
    startNativeDrag: intent !== null && intent.pointerId === event.pointerId
      && (event.buttons & 1) !== 0
      && Math.hypot(event.x - intent.x, event.y - intent.y) >= 6
  };
}

export function releaseStackRowDrag(intent: StackRowDragIntent, pointerId: number): StackRowDragIntent {
  return intent?.pointerId === pointerId ? null : intent;
}
