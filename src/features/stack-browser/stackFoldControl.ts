export function createStackFoldMarker(
  open: boolean,
  isDestroyed: () => boolean,
  restoreAfterToggle: (marker: HTMLButtonElement) => void
): HTMLElement {
  const marker = document.createElement('button');
  marker.type = 'button';
  marker.className = 'stack-editor-fold-control';
  marker.setAttribute('aria-label', open ? 'Collapse code block' : 'Expand code block');
  marker.setAttribute('aria-expanded', String(open));
  marker.textContent = open ? '⌄' : '›';
  marker.addEventListener('click', () => {
    if (isDestroyed()) return;
    restoreAfterToggle(marker);
  });
  marker.addEventListener('keydown', (event) => {
    if (event.key !== 'Enter' && event.key !== ' ') return;
    event.preventDefault();
    if (isDestroyed()) return;
    marker.click();
  });
  return marker;
}
