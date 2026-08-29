import type { StackGitFileStatus, StackGitStatus } from './stackPopup';

export type StackGitDiffRowKind = 'meta' | 'hunk' | 'addition' | 'deletion' | 'context' | 'empty';

export type StackGitDiffRow = {
  kind: StackGitDiffRowKind;
  text: string;
  prefix: string;
  body: string;
  oldLineNumber: number | '';
  newLineNumber: number | '';
};

export type StackGitEntryGroup = {
  staged: StackGitFileStatus[];
  unstaged: StackGitFileStatus[];
  stagedCount: number;
  unstagedCount: number;
  totalCount: number;
};

export type StackGitDiscardConfirmation = {
  blocked: boolean;
  title: string;
  message: string;
  paths: string[];
};

export function groupStackGitEntries(entries: readonly StackGitFileStatus[]): StackGitEntryGroup {
  const staged: StackGitFileStatus[] = [];
  const unstaged: StackGitFileStatus[] = [];
  for (const entry of entries) {
    if (entry.staged) staged.push(entry);
    if (entry.unstaged) unstaged.push(entry);
  }
  return {
    staged,
    unstaged,
    stagedCount: staged.length,
    unstagedCount: unstaged.length,
    totalCount: staged.length + unstaged.length
  };
}

export function canStageGitSelection(selection: readonly StackGitFileStatus[]) {
  return selection.some((entry) => entry.unstaged);
}

export function canUnstageGitSelection(selection: readonly StackGitFileStatus[]) {
  return selection.some((entry) => entry.staged);
}

export function canCommitGitStatus(status: StackGitStatus | null | undefined) {
  if (!status) {
    return false;
  }
  return status.entries.some((entry) => entry.staged) && status.conflicts === 0;
}

export function canDiscardGitSelection(selection: readonly StackGitFileStatus[]) {
  return selection.length > 0 && selection.every((entry) => entry.unstaged && entry.status !== 'untracked');
}

export function confirmStackGitDiscard(selection: readonly StackGitFileStatus[]): StackGitDiscardConfirmation | null {
  if (!selection.length) {
    return null;
  }

  const untracked = selection.filter((entry) => entry.status === 'untracked');
  if (untracked.length) {
    return {
      blocked: true,
      title: 'Untracked files cannot be discarded',
      message: 'Stage, delete, or stash untracked files first.',
      paths: untracked.map((entry) => entry.path)
    };
  }

  if (!canDiscardGitSelection(selection)) {
    return null;
  }

  const count = selection.length;
  return {
    blocked: false,
    title: count === 1 ? 'Discard selected file?' : `Discard ${count} selected files?`,
    message: 'This reverts working-tree changes and cannot be undone.',
    paths: selection.map((entry) => entry.path)
  };
}

export function parseStackGitUnifiedDiffRows(lines: readonly string[]): StackGitDiffRow[] {
  const rows: StackGitDiffRow[] = [];
  let oldLineNumber: number | '' = '';
  let newLineNumber: number | '' = '';
  let inHunk = false;

  const blankRow = (line: string, kind: StackGitDiffRowKind): StackGitDiffRow => ({
    kind,
    text: line,
    prefix: kind === 'hunk' ? line.slice(0, 2) || '@@' : '',
    body: kind === 'meta' ? line : kind === 'hunk' ? line.slice(2) : line.slice(1),
    oldLineNumber: '',
    newLineNumber: ''
  });

  for (const line of lines) {
    if (!line) {
      rows.push({ kind: 'empty', text: '', prefix: '', body: '', oldLineNumber: '', newLineNumber: '' });
      continue;
    }

    if (line.startsWith('@@')) {
      const match = /^@@ -(\d+)(?:,(\d+))? \+(\d+)(?:,(\d+))? @@/.exec(line);
      if (match) {
        oldLineNumber = Number(match[1]);
        newLineNumber = Number(match[3]);
        inHunk = true;
      } else {
        oldLineNumber = '';
        newLineNumber = '';
        inHunk = false;
      }
      continue;
    }

    const isFileBoundary = line.startsWith('diff --') || line.startsWith('Binary files ') || line === 'GIT binary patch';
    const isFileMarker = !inHunk && (line.startsWith('+++ ') || line.startsWith('--- '));
    const isMeta = isFileMarker || isFileBoundary || line.startsWith('index ') || line.startsWith('new file mode') || line.startsWith('deleted file mode') || line.startsWith('similarity index') || line.startsWith('dissimilarity index') || line.startsWith('rename from') || line.startsWith('rename to') || line.startsWith('copy from') || line.startsWith('copy to') || line.startsWith('old mode') || line.startsWith('new mode') || /^literal \d+$/.test(line) || /^delta \d+$/.test(line) || line.startsWith('\\ No newline at end of file') || line.startsWith('Diff truncated after ');
    if (isMeta) {
      if (isFileBoundary) {
        oldLineNumber = '';
        newLineNumber = '';
        inHunk = false;
      }
      const isVisibleNotice = line.startsWith('Binary files ') || line === 'GIT binary patch' || line.startsWith('\\ No newline at end of file') || line.startsWith('Diff truncated after ');
      if (isVisibleNotice) {
        rows.push(blankRow(line, 'meta'));
      }
      continue;
    }

    if (!inHunk) {
      rows.push({ kind: line.startsWith('+') ? 'addition' : line.startsWith('-') ? 'deletion' : line === '' ? 'empty' : 'context', text: line, prefix: line.startsWith('+') || line.startsWith('-') ? line.slice(0, 1) : ' ', body: line.startsWith('+') || line.startsWith('-') ? line.slice(1) : line, oldLineNumber: '', newLineNumber: '' });
      continue;
    }

    if (line.startsWith('+')) {
      rows.push({ kind: 'addition', text: line, prefix: '+', body: line.slice(1), oldLineNumber: '', newLineNumber: newLineNumber === '' ? '' : newLineNumber++ });
      continue;
    }

    if (line.startsWith('-')) {
      rows.push({ kind: 'deletion', text: line, prefix: '-', body: line.slice(1), oldLineNumber: oldLineNumber === '' ? '' : oldLineNumber++, newLineNumber: '' });
      continue;
    }

    rows.push({ kind: 'context', text: line, prefix: ' ', body: line.length > 1 ? line.slice(1) : ' ', oldLineNumber: oldLineNumber === '' ? '' : oldLineNumber++, newLineNumber: newLineNumber === '' ? '' : newLineNumber++ });
  }

  return rows;
}
