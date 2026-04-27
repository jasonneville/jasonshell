export type ContextMenuPoint = {
  x: number;
  y: number;
};

export type ContextMenuSize = {
  width: number;
  height: number;
};

export type ContextMenuViewport = {
  width: number;
  height: number;
};

const DEFAULT_VIEWPORT_PADDING = 8;

export function positionContextMenuInViewport(
  anchor: ContextMenuPoint,
  menu: ContextMenuSize,
  viewport: ContextMenuViewport,
  padding = DEFAULT_VIEWPORT_PADDING
): ContextMenuPoint {
  const safePadding = Math.max(0, padding);
  const safeWidth = Math.max(0, menu.width);
  const safeHeight = Math.max(0, menu.height);
  const maxX = Math.max(safePadding, viewport.width - safeWidth - safePadding);
  const maxY = Math.max(safePadding, viewport.height - safeHeight - safePadding);
  const preferredX = anchor.x + safeWidth + safePadding > viewport.width ? anchor.x - safeWidth : anchor.x;
  const preferredY = anchor.y + safeHeight + safePadding > viewport.height ? anchor.y - safeHeight : anchor.y;

  return {
    x: clamp(preferredX, safePadding, maxX),
    y: clamp(preferredY, safePadding, maxY)
  };
}

function clamp(value: number, min: number, max: number) {
  return Math.min(Math.max(value, min), Math.max(min, max));
}
