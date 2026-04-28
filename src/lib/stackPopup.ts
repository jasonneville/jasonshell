import { invoke } from '@tauri-apps/api/core';
import { emit, emitTo } from '@tauri-apps/api/event';
import { IPC_COMMANDS } from '../ipc/commands.js';
import { topBarWebviewWindowEventTarget } from './topBarPins';

export const STACK_POPUP_LABEL = 'stack-popup';
export const TOP_BAR_LABEL = 'top-bar';
export const STACK_POPUP_OPEN_EVENT = 'stack-popup:open';
export const STACK_PINS_UPDATED_EVENT = 'stack-pins:updated';

export type StackPin = {
  id: string;
  name: string;
  path: string;
};

type StackItem = {
  path: string;
  name: string;
  kind: 'folder' | 'file';
  typeLabel: string;
  iconDataUrl?: string | null;
  sizeBytes: number | null;
  modifiedAt: number | null;
  isHidden: boolean;
  isReadonly: boolean;
  isSystem: boolean;
  isSymlink: boolean;
  isReparsePoint: boolean;
};

export type StackFolderWarning = {
  path?: string | null;
  message: string;
};

type StackFolderPage = {
  path: string;
  items: StackItem[];
  offset: number;
  limit: number;
  total: number;
  hasMore: boolean;
  warnings?: StackFolderWarning[];
};

type StackPasteResult = {
  pasted: StackItem[];
  failures?: StackPasteFailure[];
};

export type StackPasteFailure = {
  path: string;
  message: string;
};

type RawShowStackPopupRequest = {
  anchorLeft: number;
  anchorWidth: number;
  path: string;
  requestId?: string | null;
};

export type StackPopupLogicalSize = {
  width: number;
  height: number;
};

export type StackEntry = {
  id: string;
  name: string;
  path: string;
  entryType: 'Folder' | 'File';
  typeLabel: string;
  iconDataUrl?: string | null;
  size?: number | null;
  modifiedMs?: number | null;
  isHidden: boolean;
  isReadonly: boolean;
  isSystem: boolean;
  isSymlink: boolean;
  isReparsePoint: boolean;
};

export type StackFolderListing = {
  path: string;
  entries: StackEntry[];
  total: number;
  warnings: StackFolderWarning[];
};

export type StackFolderListingPage = StackFolderListing & {
  offset: number;
  limit: number;
  hasMore: boolean;
};

export type StackPasteListing = StackFolderListing & {
  pasteFailures: StackPasteFailure[];
};

export type ShowStackPopupRequest = {
  anchorLeft: number;
  anchorWidth: number;
  folderPath: string;
};

let stackPopupRequestSequence = 0;

async function emitStackPinsUpdated(pins: StackPin[]) {
  await emit(STACK_PINS_UPDATED_EVENT, pins).catch(() => undefined);
  await emitTo(topBarWebviewWindowEventTarget(TOP_BAR_LABEL), STACK_PINS_UPDATED_EVENT, pins).catch(() => undefined);
}

export async function showStackPopup(request: ShowStackPopupRequest): Promise<void> {
  const payload = {
    anchorLeft: request.anchorLeft,
    anchorWidth: request.anchorWidth,
    path: request.folderPath,
    requestId: nextStackPopupRequestId()
  };
  await invoke(IPC_COMMANDS.showStackPopup, { request: payload });
  await emitTo(STACK_POPUP_LABEL, STACK_POPUP_OPEN_EVENT, payload).catch(() => undefined);
}

function nextStackPopupRequestId() {
  stackPopupRequestSequence += 1;
  return `${Date.now().toString(36)}-${stackPopupRequestSequence.toString(36)}`;
}

export function hideStackPopup(): Promise<void> {
  return invoke(IPC_COMMANDS.hideStackPopup);
}

export function getStackPopupRequest(): Promise<RawShowStackPopupRequest | null> {
  return invoke(IPC_COMMANDS.getStackPopupRequest);
}

export function beginStackPopupFocusLossHold(): Promise<void> {
  return invoke(IPC_COMMANDS.beginStackPopupFocusLossHold);
}

export function endStackPopupFocusLossHold(): Promise<void> {
  return invoke(IPC_COMMANDS.endStackPopupFocusLossHold);
}

export function resizeStackPopup(width: number, height: number, persist = false): Promise<StackPopupLogicalSize> {
  return invoke(IPC_COMMANDS.resizeStackPopup, { width, height, persist });
}

const STACK_FOLDER_INITIAL_PAGE_LIMIT = 80;
const STACK_FOLDER_SUBSEQUENT_PAGE_LIMIT = 500;

export async function listStackFolder(
  folderPath: string,
  onPage?: (page: StackFolderListingPage) => void | Promise<void>
): Promise<StackFolderListing> {
  const entries: StackEntry[] = [];
  const warnings: StackFolderWarning[] = [];
  let offset = 0;
  let total = 0;
  let responsePath = folderPath;

  while (true) {
    const limit = offset === 0 ? STACK_FOLDER_INITIAL_PAGE_LIMIT : STACK_FOLDER_SUBSEQUENT_PAGE_LIMIT;
    const page = await invoke<StackFolderPage>(IPC_COMMANDS.readStackFolder, {
      path: folderPath,
      offset,
      limit
    });
    const listingPage = stackFolderListingPageFromPage(page);
    responsePath = page.path;
    total = page.total;
    entries.push(...listingPage.entries);
    warnings.push(...listingPage.warnings);
    await onPage?.(listingPage);

    const nextOffset = page.offset + page.limit;
    if (!page.hasMore) {
      break;
    }
    if (nextOffset <= offset) {
      warnings.push({ message: 'Folder listing stopped because the backend returned no progress.' });
      break;
    }
    offset = nextOffset;
  }

  return { path: responsePath, entries, total, warnings };
}

export function listStackPins(): Promise<StackPin[]> {
  return invoke(IPC_COMMANDS.listPinnedStackFolders);
}

export async function pinStackFolder(folderPath: string): Promise<StackPin[]> {
  const pins = await invoke<StackPin[]>(IPC_COMMANDS.pinStackFolder, { path: folderPath });
  await emitStackPinsUpdated(pins);
  return pins;
}

export async function unpinStackFolder(folderPath: string): Promise<StackPin[]> {
  const pins = await invoke<StackPin[]>(IPC_COMMANDS.unpinStackFolder, { path: folderPath });
  await emitStackPinsUpdated(pins);
  return pins;
}

export async function reorderStackPins(paths: string[]): Promise<StackPin[]> {
  const pins = await invoke<StackPin[]>(IPC_COMMANDS.reorderPinnedStackFolders, { orderedPaths: paths });
  await emitStackPinsUpdated(pins);
  return pins;
}

export function copyStackItems(paths: string[], cut: boolean): Promise<void> {
  return invoke(cut ? IPC_COMMANDS.cutStackItems : IPC_COMMANDS.copyStackItems, { paths });
}

export async function pasteStackItems(destinationPath: string): Promise<StackPasteListing> {
  const result = await invoke<StackPasteResult>(IPC_COMMANDS.pasteStackItems, { destination: destinationPath });
  const listing = await listStackFolder(destinationPath);
  return { ...listing, pasteFailures: result.failures ?? [] };
}

export function renameStackItem(path: string, newName: string): Promise<StackEntry> {
  return invoke<StackItem>(IPC_COMMANDS.renameStackItem, { path, newName }).then(stackEntryFromItem);
}

export function deleteStackItem(path: string): Promise<void> {
  return invoke(IPC_COMMANDS.deleteStackItem, { path });
}

export function newStackFolder(parent: string, name: string): Promise<StackEntry> {
  return invoke<StackItem>(IPC_COMMANDS.newStackFolder, { parent, name }).then(stackEntryFromItem);
}

export function revealStackItem(path: string): Promise<void> {
  return invoke(IPC_COMMANDS.revealStackItem, { path });
}

export function openStackItem(path: string): Promise<void> {
  return invoke(IPC_COMMANDS.openStackItem, { path });
}

export function openStackItemWithPicker(path: string): Promise<void> {
  return invoke(IPC_COMMANDS.openStackItemWithPicker, { path });
}

function stackEntryFromItem(item: StackItem): StackEntry {
  return {
    id: item.path,
    name: item.name,
    path: item.path,
    entryType: item.kind === 'folder' ? 'Folder' : 'File',
    typeLabel: item.typeLabel,
    iconDataUrl: item.iconDataUrl ?? null,
    size: item.sizeBytes,
    modifiedMs: item.modifiedAt,
    isHidden: item.isHidden,
    isReadonly: item.isReadonly,
    isSystem: item.isSystem,
    isSymlink: item.isSymlink,
    isReparsePoint: item.isReparsePoint
  };
}

function stackFolderListingPageFromPage(page: StackFolderPage): StackFolderListingPage {
  return {
    path: page.path,
    entries: page.items.map(stackEntryFromItem),
    total: page.total,
    warnings: page.warnings ?? [],
    offset: page.offset,
    limit: page.limit,
    hasMore: page.hasMore
  };
}
