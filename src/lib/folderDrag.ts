export const JASONSHELL_FOLDER_DRAG_TYPE = 'application/x-jasonshell-folder';
const URI_LIST_DRAG_TYPE = 'text/uri-list';
const TEXT_DRAG_TYPE = 'text/plain';

export function hasFolderDragPayload(transfer: DataTransfer | null): boolean {
  if (!transfer) {
    return false;
  }

  const types = Array.from(transfer.types ?? []);
  return types.includes(JASONSHELL_FOLDER_DRAG_TYPE)
    || types.includes(URI_LIST_DRAG_TYPE)
    || types.includes(TEXT_DRAG_TYPE);
}

export function folderPathFromTransfer(transfer: DataTransfer | null): string | null {
  if (!transfer) {
    return null;
  }

  // Prefer explicit custom payload
  const custom = transfer.getData(JASONSHELL_FOLDER_DRAG_TYPE);
  if (custom) {
    return normalizeDroppedPath(custom);
  }

  const uri = transfer.getData(URI_LIST_DRAG_TYPE);
  if (uri) {
    return normalizeDroppedPath(pathFromUriList(uri));
  }

  // If files are present (Tauri/Electron may expose .path on File objects), prefer those
  if (transfer.files && transfer.files.length > 0) {
    for (const f of Array.from(transfer.files as any)) {
      try {
        if (f && typeof f.path === 'string' && f.path) {
          return normalizeDroppedPath(f.path);
        }
      } catch {
        // ignore
      }
    }
  }

  const text = transfer.getData(TEXT_DRAG_TYPE);
  if (text) {
    return normalizeDroppedPath(text);
  }

  return null;
}

export function folderPathToUri(path: string): string {
  const normalized = path.trim().replace(/\\/g, '/');
  if (!normalized) {
    return '';
  }

  const prefixed = /^[a-z]:/i.test(normalized)
    ? `/${normalized}`
    : normalized.startsWith('/')
      ? normalized
      : `/${normalized}`;
  return encodeURI(`file://${prefixed}`);
}

function normalizeDroppedPath(raw: string): string | null {
  const trimmed = raw.trim();
  if (!trimmed) {
    return null;
  }
  return trimmed.replace(/^"(.*)"$/, '$1');
}

function pathFromUriList(raw: string): string {
  const firstEntry = raw
    .split(/\r?\n/u)
    .map((line) => line.trim())
    .find((line) => line.length > 0 && !line.startsWith('#'));
  if (!firstEntry) {
    return '';
  }
  if (!firstEntry.toLowerCase().startsWith('file://')) {
    return firstEntry;
  }

  try {
    const url = new URL(firstEntry);
    if (url.protocol !== 'file:') {
      return '';
    }
    let path = decodeURIComponent(url.pathname);
    if (/^\/[a-z]:/i.test(path)) {
      path = path.slice(1);
    }
    return path.replace(/\//g, '\\');
  } catch {
    return firstEntry;
  }
}
