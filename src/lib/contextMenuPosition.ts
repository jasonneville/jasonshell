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

export type ScrollableContextMenuPlacement = ContextMenuPoint & {
  maxHeight: number;
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

export function positionScrollableContextMenuInViewport(
  anchor: ContextMenuPoint,
  menu: ContextMenuSize,
  viewport: ContextMenuViewport,
  padding = DEFAULT_VIEWPORT_PADDING
): ScrollableContextMenuPlacement {
  const maxHeight = contextMenuAvailableViewportHeight(viewport, padding);
  const positioned = positionContextMenuInViewport(
    anchor,
    { width: menu.width, height: Math.min(Math.max(0, menu.height), maxHeight) },
    viewport,
    padding
  );

  return {
    ...positioned,
    maxHeight
  };
}

export function contextMenuAvailableViewportHeight(
  viewport: ContextMenuViewport,
  padding = DEFAULT_VIEWPORT_PADDING
): number {
  const safePadding = Math.max(0, padding);
  return Math.max(0, viewport.height - safePadding * 2);
}

function clamp(value: number, min: number, max: number) {
  return Math.min(Math.max(value, min), Math.max(min, max));
}
