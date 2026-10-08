export type TaskGalleryMode = 'ribbon' | 'matrix';

/** All geometry is logical CSS pixels; native performs the single DPI conversion. */
export type TaskGalleryLayout = {
  mode: TaskGalleryMode;
  columns: number;
  cardWidth: number;
  cardHeight: number;
  thumbnailWidth: number;
  thumbnailHeight: number;
  footerHeight: number;
  viewportWidth: number;
  viewportHeight: number;
  panelWidth: number;
  panelHeight: number;
};

export type TaskGalleryBounds = { left: number; top: number; width: number; height: number };

export const TASK_GALLERY_GEOMETRY = Object.freeze({
  inset: 8,
  headerHeight: 24,
  headerGap: 8,
  cardGap: 8,
  cardInset: 4,
  cardWidth: 180,
  cardHeight: 136,
  thumbnailWidth: 172,
  thumbnailHeight: 96,
  footerHeight: 32,
  scrollbarWidth: 12,
  maxColumns: 3,
  maxVisibleRows: 2,
  maxFrames: 12
});

/** Fixture/legacy fallback only. Production open events carry native-authoritative layout. */
export function taskGalleryLayout(
  count: number,
  availableWidth = 584,
  availableHeight = 328
): TaskGalleryLayout {
  const g = TASK_GALLERY_GEOMETRY;
  const items = Number.isFinite(count) ? Math.max(1, Math.floor(count)) : 1;
  const width = Number.isFinite(availableWidth) ? Math.max(29, Math.floor(availableWidth)) : 584;
  const height = Number.isFinite(availableHeight) ? Math.max(49, Math.floor(availableHeight)) : 328;
  const ribbonWidth = 2 * g.inset + items * g.cardWidth + (items - 1) * g.cardGap;
  const mode: TaskGalleryMode = items >= 2 && items <= 3 && ribbonWidth <= width ? 'ribbon' : 'matrix';
  const gutter = mode === 'matrix' ? g.scrollbarWidth : 0;
  const columns = mode === 'ribbon' ? items : Math.min(items, g.maxColumns,
    Math.max(1, Math.floor((width - 2 * g.inset - gutter + g.cardGap) / (g.cardWidth + g.cardGap))));
  const cardWidth = Math.min(g.cardWidth,
    Math.max(1, Math.floor((width - 2 * g.inset - gutter - (columns - 1) * g.cardGap) / columns)));
  const rows = Math.min(g.maxVisibleRows, Math.ceil(items / columns));
  const viewportWidth = columns * cardWidth + (columns - 1) * g.cardGap + gutter;
  const viewportHeight = Math.min(rows * g.cardHeight + (rows - 1) * g.cardGap,
    height - 2 * g.inset - g.headerHeight - g.headerGap);
  return {
    mode, columns, cardWidth, cardHeight: g.cardHeight,
    thumbnailWidth: Math.max(1, cardWidth - 2 * g.cardInset),
    thumbnailHeight: g.thumbnailHeight, footerHeight: g.footerHeight,
    viewportWidth, viewportHeight,
    panelWidth: viewportWidth + 2 * g.inset,
    panelHeight: viewportHeight + 2 * g.inset + g.headerHeight + g.headerGap
  };
}
