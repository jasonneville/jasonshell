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

export type StackGitFileStatusKind = 'modified' | 'added' | 'deleted' | 'untracked' | 'conflict';

export type StackGitFileStatus = {
  path: string;
  relativePath: string;
  status: StackGitFileStatusKind;
};

export type StackGitStatus = {
  repositoryRoot: string;
  branch: string;
  modified: number;
  added: number;
  deleted: number;
  untracked: number;
  conflicts: number;
  entries: StackGitFileStatus[];
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
  sessionId?: string;
  warnings?: StackFolderWarning[];
  diagnostics?: StackFolderPageDiagnostics;
};

type StackFolderPageDiagnostics = {
  folderOpenDurationMs: number;
  pageDurationMs: number;
  pageItemCount: number;
  iconResolutionCount: number;
  iconResolutionDurationMs: number;
  iconCacheHits: number;
  iconCacheMisses: number;
  iconFallbackCount: number;
  payloadItemCount: number;
};

type StackPasteResult = {
  pasted: StackItem[];
  failures?: StackPasteFailure[];
};

export type StackOpenWithCandidate = {
  id: string;
  label: string;
  executable: string;
  source: string;
};

export type StackNativeDragPreparation = {
  paths: string[];
  effect: 'copy';
  mechanism: string;
};

export type StackItemIconResolution = {
  path: string;
  iconDataUrl: string | null;
  cacheHit: boolean;
  resolutionDurationMs: number;
};

export type StackItemIconResolutionBatch = {
  items: StackItemIconResolution[];
  requestedCount: number;
  resolvedCount: number;
  cacheHits: number;
  cacheMisses: number;
  truncated: boolean;
  maxBatchSize: number;
  totalDurationMs: number;
};

export type StackPathSuggestionRequest = {
  parentPath: string;
  segment: string;
  limit?: number;
};

export type StackPathSuggestion = {
  name: string;
  path: string;
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

type StackFolderListingDiagnostics = {
  phase: 'page' | 'first-paint' | 'metadata-complete' | 'icon-queue-complete';
  path: string;
  pageOffset: number;
  requestedLimit: number;
  pageDurationMs: number;
  folderOpenDurationMs: number;
  firstPaintDurationMs: number;
  metadataListingCompleteDurationMs: number;
  iconQueueCompleteDurationMs: number;
  pageItemCount: number;
  iconResolutionCount: number;
  iconResolutionDurationMs: number;
  iconCacheHits: number;
  iconCacheMisses: number;
  iconFallbackCount: number;
  payloadItemCount: number;
  totalItems: number;
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

const STACK_FOLDER_INITIAL_PAGE_LIMIT = 60;
const STACK_FOLDER_SUBSEQUENT_PAGE_LIMIT = 120;

export async function listStackFolder(
  folderPath: string,
  onPage?: (page: StackFolderListingPage) => void | Promise<void>
): Promise<StackFolderListing> {
  const entries: StackEntry[] = [];
  const warnings: StackFolderWarning[] = [];
  const folderOpenStartedAt = performance.now();
  let sessionId: string | undefined;
  let offset = 0;
  let total = 0;
  let responsePath = folderPath;
  let firstPaintDurationMs = 0;

  while (true) {
    const limit = offset === 0 ? STACK_FOLDER_INITIAL_PAGE_LIMIT : STACK_FOLDER_SUBSEQUENT_PAGE_LIMIT;
    const pageStartedAt = performance.now();
    const page = await invoke<StackFolderPage>(IPC_COMMANDS.readStackFolder, {
      path: folderPath,
      offset,
      limit,
      sessionId: sessionId ?? null
    });
    const pageDurationMs = Math.max(0, performance.now() - pageStartedAt);
    sessionId = page.sessionId ?? sessionId;
    const listingPage = stackFolderListingPageFromPage(page);
    responsePath = page.path;
    total = page.total;
    entries.push(...listingPage.entries);
    warnings.push(...listingPage.warnings);
    await onPage?.(listingPage);
    const folderOpenDurationMs = Math.max(0, performance.now() - folderOpenStartedAt);
    if (!firstPaintDurationMs && entries.length > 0) {
      firstPaintDurationMs = folderOpenDurationMs;
      emitStackFolderListingDiagnostics({
        phase: 'first-paint',
        path: folderPath,
        pageOffset: page.offset,
        requestedLimit: limit,
        pageDurationMs,
        folderOpenDurationMs,
        firstPaintDurationMs,
        metadataListingCompleteDurationMs: 0,
        iconQueueCompleteDurationMs: 0,
        pageItemCount: page.diagnostics?.pageItemCount ?? listingPage.entries.length,
        iconResolutionCount: page.diagnostics?.iconResolutionCount ?? 0,
        iconResolutionDurationMs: page.diagnostics?.iconResolutionDurationMs ?? 0,
        iconCacheHits: page.diagnostics?.iconCacheHits ?? 0,
        iconCacheMisses: page.diagnostics?.iconCacheMisses ?? 0,
        iconFallbackCount: page.diagnostics?.iconFallbackCount ?? 0,
        payloadItemCount: page.diagnostics?.payloadItemCount ?? listingPage.entries.length,
        totalItems: page.total,
        hasMore: page.hasMore
      });
    }

    emitStackFolderListingDiagnostics({
      phase: 'page',
      path: folderPath,
      pageOffset: page.offset,
      requestedLimit: limit,
      pageDurationMs,
      folderOpenDurationMs,
      firstPaintDurationMs,
      metadataListingCompleteDurationMs: 0,
      iconQueueCompleteDurationMs: 0,
      pageItemCount: page.diagnostics?.pageItemCount ?? listingPage.entries.length,
      iconResolutionCount: page.diagnostics?.iconResolutionCount ?? 0,
      iconResolutionDurationMs: page.diagnostics?.iconResolutionDurationMs ?? 0,
      iconCacheHits: page.diagnostics?.iconCacheHits ?? 0,
      iconCacheMisses: page.diagnostics?.iconCacheMisses ?? 0,
      iconFallbackCount: page.diagnostics?.iconFallbackCount ?? 0,
      payloadItemCount: page.diagnostics?.payloadItemCount ?? listingPage.entries.length,
      totalItems: page.total,
      hasMore: page.hasMore
    });
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

  const metadataListingCompleteDurationMs = Math.max(0, performance.now() - folderOpenStartedAt);
  emitStackFolderListingDiagnostics({
    phase: 'metadata-complete',
    path: folderPath,
    pageOffset: offset,
    requestedLimit: 0,
    pageDurationMs: 0,
    folderOpenDurationMs: metadataListingCompleteDurationMs,
    firstPaintDurationMs,
    metadataListingCompleteDurationMs,
    iconQueueCompleteDurationMs: 0,
    pageItemCount: entries.length,
    iconResolutionCount: 0,
    iconResolutionDurationMs: 0,
    iconCacheHits: 0,
    iconCacheMisses: 0,
    iconFallbackCount: 0,
    payloadItemCount: entries.length,
    totalItems: total,
    hasMore: false
  });
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

export function newStackTextFile(parent: string): Promise<StackEntry> {
  return invoke<StackItem>(IPC_COMMANDS.newStackTextFile, { parent }).then(stackEntryFromItem);
}

export function revealStackItem(path: string): Promise<void> {
  return invoke(IPC_COMMANDS.revealStackItem, { path });
}

export type StackArchiveDestinationMode = 'here' | 'folder';
export type StackArchiveExtractor = 'builtin' | 'sevenZip';

export function extractStackArchive(
  archivePath: string,
  destinationMode: StackArchiveDestinationMode,
  extractor: StackArchiveExtractor = 'builtin'
): Promise<void> {
  return invoke(IPC_COMMANDS.extractStackArchive, { archivePath, destinationMode, extractor });
}

export function showStackItemProperties(path: string): Promise<void> {
  return invoke(IPC_COMMANDS.showStackItemProperties, { path });
}

export function openStackItem(path: string): Promise<void> {
  return invoke(IPC_COMMANDS.openStackItem, { path });
}

export function openStackItemWithPicker(path: string): Promise<void> {
  return invoke(IPC_COMMANDS.openStackItemWithPicker, { path });
}

export function listStackOpenWithCandidates(path: string): Promise<StackOpenWithCandidate[]> {
  return invoke<StackOpenWithCandidate[]>(IPC_COMMANDS.listStackOpenWithCandidates, { path });
}

export function resolveStackItemIcons(paths: string[]): Promise<StackItemIconResolutionBatch> {
  return invoke<StackItemIconResolutionBatch>(IPC_COMMANDS.resolveStackItemIcons, { paths });
}

export function getStackGitStatus(folderPath: string): Promise<StackGitStatus | null> {
  return invoke<StackGitStatus | null>(IPC_COMMANDS.getStackGitStatus, { path: folderPath });
}

export function suggestStackPaths(request: StackPathSuggestionRequest): Promise<StackPathSuggestion[]> {
  return invoke<StackPathSuggestion[]>(IPC_COMMANDS.suggestStackPaths, {
    parentPath: request.parentPath,
    segment: request.segment,
    limit: request.limit ?? 20
  });
}

export function openStackItemWithApp(path: string, appId: string): Promise<void> {
  return invoke(IPC_COMMANDS.openStackItemWithApp, { path, appId });
}

export function prepareStackFileDrag(paths: string[]): Promise<StackNativeDragPreparation> {
  return invoke<StackNativeDragPreparation>(IPC_COMMANDS.prepareStackFileDrag, { paths });
}

export function openStackTerminalHere(path: string): Promise<void> {
  return invoke(IPC_COMMANDS.openStackTerminalHere, { path });
}

export function openStackFolderInVscode(path: string): Promise<void> {
  return invoke(IPC_COMMANDS.openStackFolderInVscode, { path });
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

export function emitStackFolderListingDiagnostics(diagnostics: StackFolderListingDiagnostics) {
  if (typeof console?.debug !== 'function') {
    return;
  }
  console.debug('stack-folder-listing-diagnostics', diagnostics);
}
