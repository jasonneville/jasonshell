export const JASONSHELL_FOLDER_DRAG_TYPE = 'application/x-jasonshell-folder';
const URI_LIST_DRAG_TYPE = 'text/uri-list';
const TEXT_DRAG_TYPE = 'text/plain';
const FILES_DRAG_TYPE = 'Files';

export function setFolderDragPayload(
  transfer: DataTransfer,
  paths: string[],
  effectAllowed: DataTransfer['effectAllowed'] = 'copy'
) {
  const normalizedPaths = Array.from(new Set(
    paths
      .map((path) => normalizeDroppedPath(path))
      .filter((path): path is string => Boolean(path))
  ));
  if (!normalizedPaths.length) {
    return;
  }

  transfer.effectAllowed = effectAllowed;
  transfer.setData(JASONSHELL_FOLDER_DRAG_TYPE, normalizedPaths[0]);
  transfer.setData(TEXT_DRAG_TYPE, normalizedPaths.join('\n'));
  transfer.setData(URI_LIST_DRAG_TYPE, normalizedPaths.map((path) => folderPathToUri(path)).join('\r\n'));
}

export function hasFolderDragPayload(transfer: DataTransfer | null): boolean {
  if (!transfer) {
    return false;
  }

  const types = Array.from(transfer.types ?? []);
  return types.includes(JASONSHELL_FOLDER_DRAG_TYPE)
    || types.includes(URI_LIST_DRAG_TYPE)
    || types.includes(TEXT_DRAG_TYPE)
    || types.includes(FILES_DRAG_TYPE)
    || (transfer.files?.length ?? 0) > 0;
}

export function folderPathFromTransfer(transfer: DataTransfer | null): string | null {
  return folderPathsFromTransfer(transfer)[0] ?? null;
}

export function folderPathsFromTransfer(transfer: DataTransfer | null): string[] {
  if (!transfer) {
    return [];
  }

  const paths: string[] = [];
  const pushPath = (path: string | null) => {
    if (path && !paths.some((existing) => existing.toLowerCase() === path.toLowerCase())) {
      paths.push(path);
    }
  };

  // Prefer explicit custom payload
  const custom = transfer.getData(JASONSHELL_FOLDER_DRAG_TYPE);
  if (custom) {
    pushPath(normalizeDroppedPath(custom));
  }

  const uri = transfer.getData(URI_LIST_DRAG_TYPE);
  if (uri) {
    for (const path of pathsFromUriList(uri)) {
      pushPath(normalizeDroppedPath(path));
    }
  }

  // If files are present (Tauri/Electron may expose .path on File objects), prefer those
  if (transfer.files && transfer.files.length > 0) {
    for (const f of Array.from(transfer.files as any)) {
      try {
        const file = f as { path?: unknown };
        if (typeof file.path === 'string' && file.path) {
          pushPath(normalizeDroppedPath(file.path));
        }
      } catch {
        // ignore
      }
    }
  }

  const text = transfer.getData(TEXT_DRAG_TYPE);
  if (text) {
    for (const path of pathsFromPlainText(text)) {
      pushPath(normalizeDroppedPath(path));
    }
  }

  return paths;
}

export function folderPathToUri(path: string): string {
  const normalized = path.trim().replace(/\\/g, '/');
  if (!normalized) {
    return '';
  }
  if (normalized.startsWith('//')) {
    return encodeURI(`file:${normalized}`);
  }

  const prefixed = /^[a-z]:/i.test(normalized)
    ? `/${normalized}`
    : normalized.startsWith('/')
      ? normalized
      : `/${normalized}`;
  return encodeURI(`file://${prefixed}`);
}

export function normalizeDroppedPath(raw: string): string | null {
  const trimmed = raw.trim();
  if (!trimmed) {
    return null;
  }
  const unquoted = trimmed.replace(/^"(.*)"$/, '$1');
  return normalizeFileUriPath(unquoted) ?? unquoted;
}

function pathsFromPlainText(raw: string): string[] {
  const trimmed = raw.trim();
  if (!trimmed) {
    return [];
  }
  if (trimmed.toLowerCase().startsWith('file://')) {
    return pathsFromUriList(raw);
  }
  const lines = raw
    .split(/\r?\n/u)
    .map((line) => line.trim())
    .filter((line) => line.length > 0);
  return lines.length > 1 ? lines : [trimmed];
}

function pathsFromUriList(raw: string): string[] {
  return raw
    .split(/\r?\n/u)
    .map((line) => line.trim())
    .filter((line) => line.length > 0 && !line.startsWith('#'))
    .map((entry) => normalizeFileUriPath(entry) ?? entry)
    .filter((entry) => entry.length > 0);
}

function normalizeFileUriPath(raw: string): string | null {
  if (!raw.toLowerCase().startsWith('file://')) {
    return null;
  }
  try {
    const url = new URL(raw);
    if (url.protocol !== 'file:') {
      return null;
    }
    let path = decodeURIComponent(url.pathname);
    const host = decodeURIComponent(url.hostname);
    if (host && host.toLowerCase() !== 'localhost') {
      return `\\\\${host}${path.replace(/\//g, '\\')}`;
    }
    if (/^\/[a-z]:/i.test(path)) {
      path = path.slice(1);
    }
    return path.replace(/\//g, '\\');
  } catch {
    return null;
  }
}
