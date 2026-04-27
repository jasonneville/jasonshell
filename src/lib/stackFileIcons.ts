import type { StackEntry } from './stackPopup';

export type StackFileIcon = {
  kind: 'folder' | 'app' | 'image' | 'audio' | 'video' | 'archive' | 'code' | 'document' | 'file';
  label: string;
};

const EXTENSION_ICONS: Record<string, StackFileIcon> = {
  exe: { kind: 'app', label: 'Application' },
  msi: { kind: 'app', label: 'Installer package' },
  bat: { kind: 'app', label: 'Batch script' },
  cmd: { kind: 'app', label: 'Command script' },
  ps1: { kind: 'code', label: 'PowerShell script' },
  png: { kind: 'image', label: 'Image file' },
  jpg: { kind: 'image', label: 'Image file' },
  jpeg: { kind: 'image', label: 'Image file' },
  gif: { kind: 'image', label: 'Image file' },
  webp: { kind: 'image', label: 'Image file' },
  svg: { kind: 'image', label: 'Image file' },
  mp3: { kind: 'audio', label: 'Audio file' },
  wav: { kind: 'audio', label: 'Audio file' },
  flac: { kind: 'audio', label: 'Audio file' },
  mp4: { kind: 'video', label: 'Video file' },
  mov: { kind: 'video', label: 'Video file' },
  mkv: { kind: 'video', label: 'Video file' },
  zip: { kind: 'archive', label: 'Archive file' },
  rar: { kind: 'archive', label: 'Archive file' },
  '7z': { kind: 'archive', label: 'Archive file' },
  ts: { kind: 'code', label: 'TypeScript file' },
  js: { kind: 'code', label: 'JavaScript file' },
  json: { kind: 'code', label: 'JSON file' },
  rs: { kind: 'code', label: 'Rust source file' },
  html: { kind: 'code', label: 'HTML file' },
  css: { kind: 'code', label: 'CSS file' },
  txt: { kind: 'document', label: 'Text document' },
  md: { kind: 'document', label: 'Markdown document' },
  pdf: { kind: 'document', label: 'PDF document' },
  doc: { kind: 'document', label: 'Word document' },
  docx: { kind: 'document', label: 'Word document' },
  xls: { kind: 'document', label: 'Excel workbook' },
  xlsx: { kind: 'document', label: 'Excel workbook' }
};

export function stackFileIconForEntry(entry: Pick<StackEntry, 'entryType' | 'name' | 'typeLabel'>): StackFileIcon {
  if (entry.entryType === 'Folder') {
    return { kind: 'folder', label: 'Folder' };
  }

  const extension = extensionFromName(entry.name);
  const icon = extension ? EXTENSION_ICONS[extension] : undefined;
  if (icon) {
    return icon;
  }

  const fallbackLabel = entry.typeLabel?.trim() || 'File';
  return { kind: 'file', label: fallbackLabel };
}

function extensionFromName(name: string) {
  const trimmed = name.trim();
  const lastSlash = Math.max(trimmed.lastIndexOf('\\'), trimmed.lastIndexOf('/'));
  const basename = trimmed.slice(lastSlash + 1);
  const dot = basename.lastIndexOf('.');
  return dot > 0 && dot < basename.length - 1 ? basename.slice(dot + 1).toLocaleLowerCase() : '';
}
