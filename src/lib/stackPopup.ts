import { invoke } from '@tauri-apps/api/core';

export const STACK_POPUP_LABEL = 'stack-popup';
export const STACK_POPUP_OPEN_EVENT = 'stack-popup:open';

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
  sizeBytes: number | null;
  modifiedAt: number | null;
};

type StackFolderPage = {
  items: StackItem[];
};

type StackPasteResult = {
  pasted: StackItem[];
};

type RawShowStackPopupRequest = {
  anchorLeft: number;
  anchorWidth: number;
  path: string;
};

export type StackEntry = {
  id: string;
  name: string;
  path: string;
  entryType: 'Folder' | 'File';
  typeLabel: string;
  size?: number | null;
  modifiedMs?: number | null;
};

export type ShowStackPopupRequest = {
  anchorLeft: number;
  anchorWidth: number;
  folderPath: string;
};

export function showStackPopup(request: ShowStackPopupRequest): Promise<void> {
  return invoke('show_stack_popup', {
    request: {
      anchorLeft: request.anchorLeft,
      anchorWidth: request.anchorWidth,
      path: request.folderPath
    }
  });
}

export function hideStackPopup(): Promise<void> {
  return invoke('hide_stack_popup');
}

export function getStackPopupRequest(): Promise<RawShowStackPopupRequest | null> {
  return invoke('get_stack_popup_request');
}

export function listStackFolder(folderPath: string): Promise<StackEntry[]> {
  return invoke<StackFolderPage>('read_stack_folder', {
    path: folderPath,
    offset: 0,
    limit: 500
  }).then((page) => page.items.map(stackEntryFromItem));
}

export function listStackPins(): Promise<StackPin[]> {
  return invoke('list_pinned_stack_folders');
}

export function pinStackFolder(folderPath: string): Promise<StackPin[]> {
  return invoke('pin_stack_folder', { path: folderPath }).then(() => listStackPins());
}

export function unpinStackFolder(folderPath: string): Promise<StackPin[]> {
  return invoke('unpin_stack_folder', { path: folderPath }).then(() => listStackPins());
}

export function copyStackItems(paths: string[], cut: boolean): Promise<void> {
  return invoke(cut ? 'cut_stack_items' : 'copy_stack_items', { paths });
}

export function pasteStackItems(destinationPath: string): Promise<StackEntry[]> {
  return invoke<StackPasteResult>('paste_stack_items', { destination: destinationPath })
    .then(() => listStackFolder(destinationPath));
}

export function renameStackItem(path: string, newName: string): Promise<StackEntry> {
  return invoke<StackItem>('rename_stack_item', { path, newName }).then(stackEntryFromItem);
}

export function openStackItem(path: string): Promise<void> {
  return invoke('open_stack_item', { path });
}

function stackEntryFromItem(item: StackItem): StackEntry {
  return {
    id: item.path,
    name: item.name,
    path: item.path,
    entryType: item.kind === 'folder' ? 'Folder' : 'File',
    typeLabel: item.typeLabel,
    size: item.sizeBytes,
    modifiedMs: item.modifiedAt
  };
}
