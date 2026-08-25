import type { StackGitFileStatus, StackGitStatus } from './stackPopup';

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
