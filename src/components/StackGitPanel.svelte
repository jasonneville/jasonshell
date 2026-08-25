<script lang="ts">
  import { afterUpdate, onMount, tick } from 'svelte';
  import * as stackPopup from '../lib/stackPopup';
  import {
    canCommitGitStatus,
    canStageGitSelection,
    canUnstageGitSelection,
    confirmStackGitDiscard,
    groupStackGitEntries
  } from '../lib/stackGitPanelState';
  import type {
    StackGitBranches,
    StackGitDiff,
    StackGitFileStatus,
    StackGitLog,
    StackGitStashEntry,
    StackGitStashes,
    StackGitStatus
  } from '../lib/stackPopup';

  type StackGitView = 'changes' | 'history' | 'stashes' | 'branches';
  type StackGitConfirmKind = 'discard' | 'stash-pop' | 'stash-drop' | 'checkout' | 'create-branch';

  export let folderPath = '';
  export let initialStatus: StackGitStatus | null = null;
  export let initialChangeFilter: StackGitFileStatus['status'] | 'all' = 'all';
  export let onClose: (() => void) | null = null;
  export let onRefresh: (() => void) | null = null;

  const stagedChangeGroupId = 'stack-git-staged-changes';
  const unstagedChangeGroupId = 'stack-git-unstaged-changes';
  const maxRenderedDiffLines = 4_000;

  const dateFormatter = new Intl.DateTimeFormat(undefined, { dateStyle: 'medium', timeStyle: 'short' });

  let activeView: StackGitView = 'changes';
  let status: StackGitStatus | null = initialStatus;
  let history: StackGitLog['entries'] = [];
  let branches: StackGitBranches['branches'] = [];
  let stashes: StackGitStashes['entries'] = [];
  let commitMessage = '';
  let stashMessage = '';
  let branchDraft = '';
  let newBranchDraft = '';
  let selectedChangePaths: string[] = [];
  let selectedHistoryHash = '';
  let selectedStashRef = '';
  let selectedBranchName = '';
  let diffText = '';
  let diffLines: string[] = ['Loading diff...'];
  let diffTitle = '';
  let diffDrawerOpen = false;
  let diffDrawerStaged = false;
  let diffLoading = false;
  let statusLoading = false;
  let viewLoading = false;
  let operationBusy = false;
  let commitTextarea: HTMLTextAreaElement | null = null;
  let collapsedChangeGroups = new Set<'staged' | 'unstaged'>();
  let statusMessage = '';
  let errorMessage = '';
  let dirtyCheckoutWarning = '';
  let pendingConfirm: {
    kind: StackGitConfirmKind;
    title: string;
    message: string;
    paths?: string[];
    branchName?: string;
    stashRef?: string;
  } | null = null;
  let pendingConfirmCancelButton: HTMLButtonElement | null = null;
  let pendingConfirmConfirmButton: HTMLButtonElement | null = null;
  let pendingConfirmDialogElement: HTMLDivElement | null = null;
  let pendingConfirmFocusOrigin: HTMLElement | null = null;
  let pendingConfirmWasOpen = false;
  let appliedInitialChangeFilter: StackGitFileStatus['status'] | 'all' = initialChangeFilter;

  let statusToken = 0;
  let viewToken = 0;
  let diffToken = 0;
  let mounted = false;

  $: groupedEntries = groupStackGitEntries(status?.entries ?? []);
  $: stagedEntries = groupedEntries.staged;
  $: unstagedEntries = groupedEntries.unstaged;
  $: canCommit = canCommitGitStatus(status);
  $: hasDirtyWorkingTree = groupedEntries.totalCount > 0;
  $: remoteUrl = status?.remoteRepositoryUrl ?? null;

  $: if (mounted && folderPath) {
    void refreshStatus();
  }

  onMount(() => {
    mounted = true;
  });

  afterUpdate(() => {
    resizeCommitTextarea();
  });

  function formatDate(value: string | number | null | undefined) {
    if (!value) return '';
    const parsed = typeof value === 'number' ? value : Date.parse(value);
    return Number.isFinite(parsed) ? dateFormatter.format(new Date(parsed)) : String(value);
  }

  function gitStatusSymbol(statusKind: StackGitFileStatus['status'] | null | undefined) {
    if (statusKind === 'added') return '+';
    if (statusKind === 'deleted') return '-';
    if (statusKind === 'modified') return 'M';
    if (statusKind === 'untracked') return '?';
    if (statusKind === 'conflict') return '!';
    return '·';
  }

  function gitStatusLabel(statusKind: StackGitFileStatus['status'] | null | undefined) {
    if (statusKind === 'added') return 'Added';
    if (statusKind === 'deleted') return 'Deleted';
    if (statusKind === 'modified') return 'Modified';
    if (statusKind === 'untracked') return 'Untracked';
    if (statusKind === 'conflict') return 'Conflict';
    return 'Changed';
  }

  function normalizeRef(entry: StackGitStashEntry) {
    return entry.stashRef ?? entry.ref ?? '';
  }

  function normalizeStashLabel(entry: StackGitStashEntry) {
    return entry.message ?? entry.ref ?? `stash@{${entry.index}}`;
  }

  function normalizeBranchLabel(entry: StackGitBranches['branches'][number]) {
    return entry.name.replace(/^refs\/heads\//, '');
  }

  function currentChangeSelection() {
    return status?.entries.filter((entry) => selectedChangePaths.includes(entry.path)) ?? [];
  }

  function openChangeDiff(entry: StackGitFileStatus, staged = entry.staged) {
    if (diffDrawerOpen && selectedChangePaths.includes(entry.path) && diffDrawerStaged === staged) {
      closeDiffDrawer();
      return;
    }
    diffDrawerOpen = true;
    diffDrawerStaged = staged;
    selectChangePath(entry.path);
    diffTitle = entry.relativePath;
    void loadDiff(entry.path, staged, entry.status);
  }

  function selectChangePath(path: string, additive = false) {
    selectedChangePaths = additive
      ? selectedChangePaths.includes(path)
        ? selectedChangePaths.filter((item) => item !== path)
        : [...selectedChangePaths, path]
      : [path];
  }

  function ensureChangeSelection() {
    const available = new Set(status?.entries.map((entry) => entry.path) ?? []);
    selectedChangePaths = selectedChangePaths.filter((path) => available.has(path));
    if (!selectedChangePaths.length) {
      const matching = initialChangeFilter === 'all'
        ? []
        : status?.entries.filter((entry) => entry.status === initialChangeFilter).map((entry) => entry.path) ?? [];
      const first = unstagedEntries[0] ?? stagedEntries[0] ?? status?.entries[0] ?? null;
      selectedChangePaths = matching.length ? matching : first ? [first.path] : [];
    }
  }

  function ensureHistorySelection() {
    if (!selectedHistoryHash && history.length) selectedHistoryHash = history[0].commitHash;
  }

  function ensureStashSelection() {
    if (!selectedStashRef && stashes.length) selectedStashRef = normalizeRef(stashes[0]);
  }

  function ensureBranchSelection() {
    if (!selectedBranchName && branches.length) selectedBranchName = status?.branch ?? branches[0].name;
  }

  async function refreshStatus() {
    const token = ++statusToken;
    if (!folderPath) {
      status = null;
      history = [];
      branches = [];
      stashes = [];
      diffText = '';
      diffTitle = '';
      statusLoading = false;
      viewLoading = false;
      diffLoading = false;
      return;
    }

    statusLoading = true;
    errorMessage = '';
    try {
      const nextStatus = await stackPopup.getStackGitStatus(folderPath);
      if (token !== statusToken) return;
      status = nextStatus;
      if (!nextStatus) {
        history = [];
        branches = [];
        stashes = [];
        selectedChangePaths = [];
        selectedHistoryHash = '';
        selectedStashRef = '';
        selectedBranchName = '';
        diffText = '';
        diffTitle = '';
        statusMessage = 'No repository found';
        dirtyCheckoutWarning = '';
        return;
      }

      branchDraft = nextStatus.branch ?? branchDraft;
      dirtyCheckoutWarning = nextStatus.entries.length
        ? 'Dirty checkout warning: branch switches may overwrite local changes.'
        : '';
      ensureChangeSelection();

      if (activeView === 'changes' && diffDrawerOpen) {
        const drawerPath = selectedChangePaths[0];
        const drawerRowExists = nextStatus.entries.some((entry) =>
          entry.path === drawerPath && (diffDrawerStaged ? entry.staged : entry.unstaged)
        );
        if (drawerRowExists) void refreshChangesDiff();
        else closeDiffDrawer();
      }
      if (activeView === 'history') void refreshHistory();
      if (activeView === 'stashes') void refreshStashes();
      if (activeView === 'branches') void refreshBranches();
      statusMessage = `${nextStatus.branch} · ${nextStatus.repositoryRoot}`;
    } catch (error) {
      if (token === statusToken) {
        status = null;
        statusMessage = 'Git unavailable';
        errorMessage = error instanceof Error ? error.message : 'Git status unavailable';
      }
    } finally {
      if (token === statusToken) statusLoading = false;
    }
  }

  async function refreshHistory() {
    const token = ++viewToken;
    if (!status || !folderPath) return;
    viewLoading = true;
    try {
      const log = await stackPopup.stackGitLog(folderPath, 80);
      if (token !== viewToken) return;
      history = log.entries;
      ensureHistorySelection();
      await refreshHistoryDiff();
    } catch (error) {
      if (token === viewToken) errorMessage = error instanceof Error ? error.message : 'Git history unavailable';
    } finally {
      if (token === viewToken) viewLoading = false;
    }
  }

  async function refreshBranches() {
    const token = ++viewToken;
    if (!status || !folderPath) return;
    viewLoading = true;
    try {
      const next = await stackPopup.stackGitBranches(folderPath);
      if (token !== viewToken) return;
      branches = next.branches;
      if (!selectedBranchName || !branches.some((branch) => branch.name === selectedBranchName)) {
        selectedBranchName = next.currentBranch ?? status.branch ?? branches[0]?.name ?? '';
      }
      ensureBranchSelection();
    } catch (error) {
      if (token === viewToken) errorMessage = error instanceof Error ? error.message : 'Git branches unavailable';
    } finally {
      if (token === viewToken) viewLoading = false;
    }
  }

  async function refreshStashes() {
    const token = ++viewToken;
    if (!status || !folderPath) return;
    viewLoading = true;
    try {
      const next = await stackPopup.stackGitStashes(folderPath);
      if (token !== viewToken) return;
      stashes = next.entries;
      ensureStashSelection();
      await refreshStashDiff();
    } catch (error) {
      if (token === viewToken) errorMessage = error instanceof Error ? error.message : 'Git stashes unavailable';
    } finally {
      if (token === viewToken) viewLoading = false;
    }
  }

  async function refreshChangesDiff() {
    const selected = currentChangeSelection();
    const targetPath = selected[0]?.path ?? status?.entries[0]?.path ?? '';
    if (!targetPath) {
      diffLoading = false;
      diffText = 'Select a file to see a diff.';
      diffTitle = 'Diff';
      return;
    }

    diffTitle = selected.length > 1 ? `${selected.length} files selected` : selected[0]?.relativePath ?? targetPath;
    await loadDiff(targetPath, diffDrawerStaged, selected[0]?.status ?? null);
  }

  async function refreshHistoryDiff() {
    const entry = history.find((item) => item.commitHash === selectedHistoryHash) ?? history[0] ?? null;
    if (!entry) {
      diffText = 'Select a commit to inspect the diff.';
      diffTitle = 'History diff';
      return;
    }

    diffTitle = `${entry.shortHash} · ${entry.subject}`;
    diffText = [
      `Commit ${entry.commitHash}`,
      `${entry.authorName} · ${formatDate(entry.authoredAt)}`,
      'Working-tree diff preview stays on file rows.'
    ].join('\n');
  }

  async function refreshStashDiff() {
    const entry = stashes.find((item) => normalizeRef(item) === selectedStashRef) ?? stashes[0] ?? null;
    if (!entry) {
      diffText = 'Select a stash to inspect the diff.';
      diffTitle = 'Stash diff';
      return;
    }

    diffTitle = normalizeStashLabel(entry);
    diffText = [
      `Stash ${normalizeRef(entry)}`,
      `Branch ${entry.branch ?? 'unknown'}`,
      'Working-tree diff preview stays on file rows.'
    ].join('\n');
  }

  async function loadDiff(target: string, staged = false, statusKind: StackGitFileStatus['status'] | null = null) {
    const token = ++diffToken;
    const requestFolderPath = folderPath;
    diffLoading = true;
    diffText = '';
    if (statusKind === 'untracked' && !staged) {
      diffText = 'Preview unavailable until staged.';
      diffLoading = false;
      return;
    }
    try {
      const result = await stackPopup.stackGitDiff(folderPath, target, staged);
      if (token !== diffToken || requestFolderPath !== folderPath) return;
      diffText = normalizeDiffResult(result) || 'No diff content.';
    } catch (error) {
      if (token === diffToken) diffText = error instanceof Error ? error.message : 'Diff unavailable';
    } finally {
      if (token === diffToken) diffLoading = false;
    }
  }

  function normalizeDiffResult(result: StackGitDiff | null | undefined) {
    return result?.content ?? '';
  }

  $: diffLines = getDiffLines(diffText || 'Loading diff...');

  type DiffLineKind = 'meta' | 'hunk' | 'addition' | 'deletion' | 'context' | 'empty';

  type DiffLineRender = {
    kind: DiffLineKind;
    text: string;
    prefix: string;
    body: string;
  };

  function classifyDiffLine(line: string): DiffLineKind {
    if (!line) return 'empty';
    if (line.startsWith('+++ ') || line.startsWith('--- ') || line.startsWith('diff --git ') || line.startsWith('index ') || line.startsWith('new file mode') || line.startsWith('deleted file mode') || line.startsWith('similarity index') || line.startsWith('rename from') || line.startsWith('rename to') || line.startsWith('old mode') || line.startsWith('new mode') || line.startsWith('Binary files ')) {
      return 'meta';
    }
    if (line.startsWith('@@')) return 'hunk';
    if (line.startsWith('+')) return 'addition';
    if (line.startsWith('-')) return 'deletion';
    return 'context';
  }

  function renderDiffLineContent(line: string): DiffLineRender {
    const kind = classifyDiffLine(line);
    if (kind === 'meta') {
      return { kind, text: line || ' ', prefix: '', body: '' };
    }

    if (kind === 'hunk') {
      return {
        kind,
        text: line.length ? line : ' ',
        prefix: line.slice(0, 2) || '@@',
        body: line.slice(2) || ' '
      };
    }

    const prefix = line.slice(0, 1) || ' ';
    const body = line.slice(1) || ' ';
    return { kind, text: line.length ? line : ' ', prefix, body };
  }

  function getDiffLines(text: string) {
    const lines = text.split(/\r?\n/);
    if (lines.length <= maxRenderedDiffLines) return lines;
    return [
      ...lines.slice(0, maxRenderedDiffLines),
      `Diff truncated after ${maxRenderedDiffLines.toLocaleString()} lines (${lines.length.toLocaleString()} total).`
    ];
  }

  async function stagePaths(paths: string[]) {
    if (!paths.length || operationBusy) return;
    if (!stackPopup.stackGitAddPaths || !status) return;
    operationBusy = true;
    try {
      const result = await stackPopup.stackGitAddPaths(folderPath, paths);
      statusMessage = result.summary;
      await refreshAfterMutation();
    } catch (error) {
      errorMessage = error instanceof Error ? error.message : 'Stage failed';
    } finally {
      operationBusy = false;
    }
  }

  async function unstagePaths(paths: string[]) {
    if (!paths.length || operationBusy || !status) return;
    operationBusy = true;
    try {
      const result = await stackPopup.stackGitUnstagePaths(folderPath, paths);
      statusMessage = result.summary;
      await refreshAfterMutation();
    } catch (error) {
      errorMessage = error instanceof Error ? error.message : 'Unstage failed';
    } finally {
      operationBusy = false;
    }
  }

  async function discardPaths(entries: StackGitFileStatus[]) {
    if (!entries.length || operationBusy) return;
    const confirmation = confirmStackGitDiscard(entries);
    if (!confirmation) return;
    if (confirmation.blocked) {
      errorMessage = confirmation.message;
      return;
    }
    openPendingConfirm({ kind: 'discard', title: confirmation.title, message: confirmation.message, paths: confirmation.paths });
  }

  async function commitChanges(pushAfter = false) {
    if (!status || !commitMessage.trim() || !canCommit || operationBusy) return;
    operationBusy = true;
    try {
      const result = await stackPopup.stackGitCommit(folderPath, commitMessage.trim(), stagedEntries.map((entry) => entry.path));
      if (pushAfter) {
        if (!status.remoteRepositoryUrl) throw new Error('No remote configured');
        const pushResult = await stackPopup.stackGitPush(folderPath);
        statusMessage = `${result.summary}; ${pushResult.summary}`;
      } else {
        statusMessage = result.summary;
      }
      commitMessage = '';
      await refreshAfterMutation();
    } catch (error) {
      statusMessage = '';
      errorMessage = error instanceof Error ? error.message : 'Commit failed';
    } finally {
      operationBusy = false;
    }
  }

  async function refreshAfterMutation() {
    await refreshStatus();
    onRefresh?.();
  }

  async function syncRemote(operation: 'fetch' | 'pull' | 'push') {
    if (!status || operationBusy) return;
    operationBusy = true;
    try {
      const result = operation === 'fetch'
        ? await stackPopup.stackGitFetch(folderPath)
        : operation === 'pull'
          ? await stackPopup.stackGitPull(folderPath)
          : await stackPopup.stackGitPush(folderPath);
      statusMessage = result.summary;
      await refreshAfterMutation();
    } catch (error) {
      errorMessage = error instanceof Error ? error.message : `Git ${operation} failed`;
    } finally {
      operationBusy = false;
    }
  }

  async function checkoutBranch(branch: string) {
    if (!branch || operationBusy || !status) return;
    if (hasDirtyWorkingTree) {
      openPendingConfirm({
        kind: 'checkout',
        title: 'Checkout dirty branch?',
        message: dirtyCheckoutWarning || 'Dirty checkout warning: branch switches may overwrite local changes.',
        branchName: branch
      });
      return;
    }
    await confirmCheckout(branch);
  }

  async function confirmCheckout(branch: string) {
    if (!branch || operationBusy) return;
    operationBusy = true;
    try {
      const result = await stackPopup.stackGitCheckoutBranch(folderPath, branch);
      statusMessage = result.summary;
      branchDraft = '';
      await refreshAfterMutation();
    } catch (error) {
      errorMessage = error instanceof Error ? error.message : 'Checkout failed';
    } finally {
      operationBusy = false;
    }
  }

  async function createBranch() {
    const name = newBranchDraft.trim();
    if (!name || operationBusy || !status) return;
    if (hasDirtyWorkingTree) {
      openPendingConfirm({
        kind: 'create-branch',
        title: 'Create branch in dirty tree?',
        message: dirtyCheckoutWarning || 'Dirty checkout warning: creating a branch may affect local changes.',
        branchName: name
      });
      return;
    }
    operationBusy = true;
    try {
      const result = await stackPopup.stackGitCreateBranch(folderPath, name, true);
      statusMessage = result.summary;
      newBranchDraft = '';
      await refreshAfterMutation();
    } catch (error) {
      errorMessage = error instanceof Error ? error.message : 'Branch creation failed';
    } finally {
      operationBusy = false;
    }
  }

  async function stashChanges() {
    if (!status || operationBusy) return;
    operationBusy = true;
    try {
      const result = await stackPopup.stackGitStash(folderPath, stashMessage.trim() || 'WIP', status.entries.some((entry) => entry.status === 'untracked'));
      statusMessage = result.summary;
      stashMessage = '';
      await refreshAfterMutation();
    } catch (error) {
      errorMessage = error instanceof Error ? error.message : 'Stash failed';
    } finally {
      operationBusy = false;
    }
  }

  async function applyStash(stashRef: string) {
    if (!stashRef || operationBusy) return;
    operationBusy = true;
    try {
      const result = await stackPopup.stackGitStashApply(folderPath, stashRef);
      statusMessage = result.summary;
      await refreshAfterMutation();
    } catch (error) {
      errorMessage = error instanceof Error ? error.message : 'Stash apply failed';
    } finally {
      operationBusy = false;
    }
  }

  async function popStash(stashRef: string) {
    if (!stashRef || operationBusy) return;
    openPendingConfirm({ kind: 'stash-pop', title: 'Pop stash?', message: 'Pop applies the stash and removes it from the list.', stashRef });
  }

  async function dropStash(stashRef: string) {
    if (!stashRef || operationBusy) return;
    openPendingConfirm({ kind: 'stash-drop', title: 'Drop stash?', message: 'Drop removes this stash permanently.', stashRef });
  }

  async function confirmPendingAction() {
    const action = pendingConfirm;
    closePendingConfirm();
    if (!action) return;

    if (action.kind === 'discard' && action.paths) {
      operationBusy = true;
      try {
        const result = await stackPopup.stackGitRevertPaths({ folderPath, paths: action.paths });
        statusMessage = result.summary;
        await refreshAfterMutation();
      } catch (error) {
        errorMessage = error instanceof Error ? error.message : 'Discard failed';
      } finally {
        operationBusy = false;
      }
      return;
    }

    if (action.kind === 'stash-pop' && action.stashRef) {
      operationBusy = true;
      try {
        const result = await stackPopup.stackGitStashPop(folderPath, action.stashRef);
        statusMessage = result.summary;
        await refreshAfterMutation();
      } catch (error) {
        errorMessage = error instanceof Error ? error.message : 'Stash pop failed';
      } finally {
        operationBusy = false;
      }
      return;
    }

    if (action.kind === 'stash-drop' && action.stashRef) {
      operationBusy = true;
      try {
        const result = await stackPopup.stackGitStashDrop(folderPath, action.stashRef);
        statusMessage = result.summary;
        await refreshAfterMutation();
      } catch (error) {
        errorMessage = error instanceof Error ? error.message : 'Stash drop failed';
      } finally {
        operationBusy = false;
      }
      return;
    }

    if (action.kind === 'checkout' && action.branchName) {
      await confirmCheckout(action.branchName);
      return;
    }

    if (action.kind === 'create-branch' && action.branchName) {
      operationBusy = true;
      try {
        const result = await stackPopup.stackGitCreateBranch(folderPath, action.branchName, true);
        statusMessage = result.summary;
        newBranchDraft = '';
        await refreshAfterMutation();
      } catch (error) {
        errorMessage = error instanceof Error ? error.message : 'Branch creation failed';
      } finally {
        operationBusy = false;
      }
    }
  }

  function closePanel() {
    onClose?.();
  }

  function selectBranch(branch: string) {
    selectedBranchName = branch;
    branchDraft = branch;
  }

  function selectHistory(commitHash: string) {
    selectedHistoryHash = commitHash;
    void refreshHistoryDiff();
  }

  function selectStash(stashRef: string) {
    selectedStashRef = stashRef;
    void refreshStashDiff();
  }

  function handleTabChange(view: StackGitView) {
    if (activeView === 'changes' && view !== 'changes') closeDiffDrawer();
    activeView = view;
    if (view === 'changes') {
      ensureChangeSelection();
      if (diffDrawerOpen) void refreshChangesDiff();
    } else if (view === 'history') {
      ensureHistorySelection();
      void refreshHistory();
    } else if (view === 'stashes') {
      ensureStashSelection();
      void refreshStashes();
    } else {
      ensureBranchSelection();
      void refreshBranches();
    }
  }

  function pendingConfirmLabel() {
    return pendingConfirm?.message ?? '';
  }

  function statusBadgeClass(kind: StackGitFileStatus['status']) {
    return `stack-git-badge git-status-${kind}`;
  }

  function currentBranchLabel() {
    return status?.branch ?? 'Detached';
  }

  function clearMessages() {
    errorMessage = '';
    statusMessage = '';
  }

  async function refreshAll() {
    clearMessages();
    await refreshStatus();
    onRefresh?.();
  }

  function canDiscardEntry(entry: StackGitFileStatus) {
    return entry.status !== 'untracked' && !entry.staged;
  }

  function isChangeGroupCollapsed(group: 'staged' | 'unstaged') {
    return collapsedChangeGroups.has(group);
  }

  function toggleChangeGroup(group: 'staged' | 'unstaged') {
    const next = new Set(collapsedChangeGroups);
    if (next.has(group)) next.delete(group);
    else next.add(group);
    collapsedChangeGroups = next;
  }

  function changeRowActionLabel(group: 'staged' | 'unstaged') {
    return group === 'staged' ? 'Unstage' : 'Stage';
  }

  function changeRowActionSymbol(group: 'staged' | 'unstaged') {
    return group === 'staged' ? '−' : '+';
  }

  function handleChangeRowAction(entry: StackGitFileStatus, group: 'staged' | 'unstaged') {
    if (group === 'staged') void unstagePaths([entry.path]);
    else void stagePaths([entry.path]);
  }

  function handleChangeRowKeydown(event: KeyboardEvent, entry: StackGitFileStatus, group: 'staged' | 'unstaged') {
    if (event.key === 'Enter') {
      event.preventDefault();
      openChangeDiff(entry, group === 'staged');
      return;
    }
    if (event.key === ' ') {
      event.preventDefault();
      handleChangeRowAction(entry, group);
    }
  }

  function closeDiffDrawer() {
    diffToken += 1;
    diffDrawerOpen = false;
    diffLoading = false;
    diffText = '';
    diffTitle = '';
  }

  function resizeCommitTextarea() {
    if (!commitTextarea) return;
    commitTextarea.style.height = 'auto';
    const nextHeight = Math.min(Math.max(commitTextarea.scrollHeight, 38), 200);
    commitTextarea.style.height = `${nextHeight}px`;
    commitTextarea.style.overflowY = commitTextarea.scrollHeight > 200 ? 'auto' : 'hidden';
  }

  function handleEscape(event: KeyboardEvent) {
    if (event.key !== 'Escape') return;
    event.preventDefault();
    event.stopImmediatePropagation();
    if (pendingConfirm) {
      closePendingConfirm();
      return;
    }
    if (diffDrawerOpen) {
      closeDiffDrawer();
      return;
    }
    closePanel();
  }

  function openPendingConfirm(next: NonNullable<typeof pendingConfirm>) {
    pendingConfirmFocusOrigin = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    pendingConfirm = next;
  }

  function closePendingConfirm() {
    pendingConfirm = null;
  }

  function handlePendingConfirmKeydown(event: KeyboardEvent) {
    if (event.key !== 'Tab') return;
    const focusables = [pendingConfirmCancelButton, pendingConfirmConfirmButton].filter(
      (button): button is HTMLButtonElement => Boolean(button && !button.disabled)
    );
    if (!focusables.length) return;

    const currentIndex = focusables.findIndex((button) => button === document.activeElement);
    const nextIndex = event.shiftKey
      ? (currentIndex <= 0 ? focusables.length - 1 : currentIndex - 1)
      : (currentIndex < 0 || currentIndex === focusables.length - 1 ? 0 : currentIndex + 1);
    event.preventDefault();
    focusables[nextIndex]?.focus();
  }

  $: if (initialChangeFilter !== appliedInitialChangeFilter) {
    appliedInitialChangeFilter = initialChangeFilter;
    selectedChangePaths = [];
    if (status && activeView === 'changes') {
      ensureChangeSelection();
      if (diffDrawerOpen) void refreshChangesDiff();
    }
  }

  $: if (pendingConfirm && !pendingConfirmWasOpen) {
    pendingConfirmFocusOrigin = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    void tick().then(() => pendingConfirmCancelButton?.focus());
  }

  $: if (!pendingConfirm && pendingConfirmWasOpen) {
    const origin = pendingConfirmFocusOrigin;
    pendingConfirmFocusOrigin = null;
    void tick().then(() => {
      const target = origin as HTMLElement | null;
      target?.focus();
    });
  }

  $: pendingConfirmWasOpen = Boolean(pendingConfirm);

  $: if (status && activeView === 'changes') ensureChangeSelection();
  $: if (status && activeView === 'history') ensureHistorySelection();
  $: if (status && activeView === 'stashes') ensureStashSelection();
  $: if (status && activeView === 'branches') ensureBranchSelection();
</script>

<svelte:window on:keydown={handleEscape} />

<section class="stack-git-panel" aria-label="Git panel" aria-busy={statusLoading || viewLoading || diffLoading ? 'true' : 'false'}>
  <header class="stack-git-panel-header">
    <div class="stack-git-panel-row stack-git-panel-row--primary">
      <button type="button" class="stack-git-branch-selector" aria-label={`Branch ${currentBranchLabel()}`} title={currentBranchLabel()} on:click={() => handleTabChange('branches')}>
        <span class="stack-git-branch-selector__icon" aria-hidden="true">⑂</span>
        <span class="stack-git-branch-selector__label">{currentBranchLabel()}</span>
      </button>

      <div class="stack-git-panel-header-actions">
        <button type="button" class="stack-git-icon-button" aria-label="Refresh git panel" title="Refresh" on:click={() => void refreshAll()}>↻</button>
        <button type="button" class="stack-git-icon-button" aria-label="Close git panel" title="Close" on:click={closePanel}>✕</button>
      </div>
    </div>

    <div class="stack-git-panel-row stack-git-panel-row--secondary">
      {#if status && (status.ahead != null || status.behind != null)}
        <span class="stack-git-upstream-pill" aria-label="Upstream status">
          {#if status.ahead || status.behind}
            {status.ahead ? `↑${status.ahead}` : ''}{status.ahead && status.behind ? ' ' : ''}{status.behind ? `↓${status.behind}` : ''}
          {:else}
            Synced
          {/if}
        </span>
      {/if}

      <details class="stack-git-repository-menu">
        <summary aria-label="Repository views" title="Repository views"><span aria-hidden="true">⋯</span></summary>
        <div class="stack-git-repository-menu__list" aria-label="Repository views">
          <button type="button" aria-current={activeView === 'changes' ? 'page' : undefined} class:active={activeView === 'changes'} on:click={() => handleTabChange('changes')}>Changes</button>
          <button type="button" aria-current={activeView === 'history' ? 'page' : undefined} class:active={activeView === 'history'} on:click={() => handleTabChange('history')}>History</button>
          <button type="button" aria-current={activeView === 'stashes' ? 'page' : undefined} class:active={activeView === 'stashes'} on:click={() => handleTabChange('stashes')}>Stashes</button>
          <button type="button" aria-current={activeView === 'branches' ? 'page' : undefined} class:active={activeView === 'branches'} on:click={() => handleTabChange('branches')}>Branches</button>
        </div>
      </details>

      <div class="stack-git-sync-actions" aria-label="Git sync actions">
        <button type="button" class="stack-git-icon-button" disabled={!status || operationBusy} aria-label="Fetch" title="Fetch" on:click={() => void syncRemote('fetch')}>↓</button>
        <button type="button" class="stack-git-icon-button" disabled={!status || operationBusy} aria-label="Pull" title="Pull" on:click={() => void syncRemote('pull')}>⇣</button>
        <button type="button" class="stack-git-icon-button" disabled={!status || operationBusy || !remoteUrl} aria-label="Push" title="Push" on:click={() => void syncRemote('push')}>⇡</button>
      </div>
    </div>
  </header>

  <div class="stack-git-panel-content">
    <section class="stack-git-panel-scroll" aria-label={activeView === 'changes' ? 'Git changes' : activeView === 'history' ? 'Git history' : activeView === 'stashes' ? 'Git stashes' : 'Git branches'}>
      {#if activeView === 'changes'}
        <div class="stack-git-change-groups" role="list">
          <section id={stagedChangeGroupId} class="stack-git-change-group" aria-label="Staged changes">
            <header class="stack-git-change-group-header">
              <button type="button" class="stack-git-change-group-header__bulk" aria-label="Unstage all staged files" title="Unstage all" disabled={!canUnstageGitSelection(stagedEntries) || operationBusy} on:click={() => void unstagePaths(stagedEntries.map((entry) => entry.path))}>−</button>
              <button type="button" class="stack-git-change-group-header__title" aria-controls={stagedChangeGroupId} aria-expanded={!isChangeGroupCollapsed('staged')} on:click={() => toggleChangeGroup('staged')}>
                <span>Staged</span>
                <span>{groupedEntries.stagedCount}</span>
                <span aria-hidden="true">{isChangeGroupCollapsed('staged') ? '▸' : '▾'}</span>
              </button>
            </header>

            {#if !isChangeGroupCollapsed('staged')}
              {#if stagedEntries.length}
                {#each stagedEntries as entry (entry.path + (entry.unstaged ? '-both-staged' : '-staged'))}
                  <div class="stack-git-change-row staged" role="listitem">
                    <button type="button" class="stack-git-change-row__action" aria-label="Unstage" title={changeRowActionLabel('staged')} disabled={operationBusy} on:click={() => handleChangeRowAction(entry, 'staged')}>{changeRowActionSymbol('staged')}</button>
                    <button type="button" class="stack-git-change-row__content" aria-pressed={diffDrawerOpen && selectedChangePaths.includes(entry.path) && diffDrawerStaged} aria-expanded={diffDrawerOpen && selectedChangePaths.includes(entry.path) && diffDrawerStaged} on:click={() => openChangeDiff(entry, true)} on:keydown={(event) => handleChangeRowKeydown(event, entry, 'staged')}>
                      <span class={statusBadgeClass(entry.status)} aria-label={gitStatusLabel(entry.status)}>{gitStatusSymbol(entry.status)}</span>
                      <span class="stack-git-change-row__path" title={entry.relativePath}>
                        {#if entry.relativePath.includes('/')}
                          {@const lastSlash = entry.relativePath.lastIndexOf('/')}
                          <span class="stack-git-change-row__dir">{entry.relativePath.slice(0, lastSlash)}</span><span class="stack-git-change-row__sep">/</span><span class="stack-git-change-row__name">{entry.relativePath.slice(lastSlash + 1)}</span>
                        {:else}
                          <span class="stack-git-change-row__name">{entry.relativePath}</span>
                        {/if}
                      </span>
                    </button>
                  </div>
                  {#if diffDrawerOpen && entry.path === selectedChangePaths[0] && diffDrawerStaged}
                    <div class="stack-git-change-diff-drawer" role="region" aria-label={`Diff for ${entry.relativePath}`}>
                      <pre class="stack-git-diff-view" aria-label={`Unified diff for ${entry.relativePath}`}>{#each diffLines as line, index (index)}{@const rendered = renderDiffLineContent(line)}<span class={`stack-git-diff-line stack-git-diff-line--${rendered.kind}`} data-kind={rendered.kind}>{#if rendered.kind === 'meta'}<span class="stack-git-diff-line__meta">{rendered.text}</span>{:else}<span class="stack-git-diff-line__prefix">{rendered.prefix}</span><span class="stack-git-diff-line__body">{rendered.body}</span>{/if}</span>{/each}</pre>
                    </div>
                  {/if}
                {/each}
              {:else}
                <div class="stack-git-empty">No staged files</div>
              {/if}
            {/if}
          </section>

          <section id={unstagedChangeGroupId} class="stack-git-change-group" aria-label="Unstaged changes">
            <header class="stack-git-change-group-header">
              <button type="button" class="stack-git-change-group-header__bulk" aria-label="Stage all unstaged files" title="Stage all" disabled={!canStageGitSelection(unstagedEntries) || operationBusy} on:click={() => void stagePaths(unstagedEntries.map((entry) => entry.path))}>+</button>
              <button type="button" class="stack-git-change-group-header__title" aria-controls={unstagedChangeGroupId} aria-expanded={!isChangeGroupCollapsed('unstaged')} on:click={() => toggleChangeGroup('unstaged')}>
                <span>Unstaged</span>
                <span>{groupedEntries.unstagedCount}</span>
                <span aria-hidden="true">{isChangeGroupCollapsed('unstaged') ? '▸' : '▾'}</span>
              </button>
            </header>

            {#if !isChangeGroupCollapsed('unstaged')}
              {#if unstagedEntries.length}
                {#each unstagedEntries as entry (entry.path + (entry.staged ? '-both-unstaged' : '-unstaged'))}
                  <div class="stack-git-change-row unstaged" role="listitem">
                    <button type="button" class="stack-git-change-row__action" aria-label="Stage" title={changeRowActionLabel('unstaged')} disabled={operationBusy} on:click={() => handleChangeRowAction(entry, 'unstaged')}>{changeRowActionSymbol('unstaged')}</button>
                    <button type="button" class="stack-git-change-row__content" aria-pressed={diffDrawerOpen && selectedChangePaths.includes(entry.path) && !diffDrawerStaged} aria-expanded={diffDrawerOpen && selectedChangePaths.includes(entry.path) && !diffDrawerStaged} on:click={() => openChangeDiff(entry, false)} on:keydown={(event) => handleChangeRowKeydown(event, entry, 'unstaged')}>
                      <span class={statusBadgeClass(entry.status)} aria-label={gitStatusLabel(entry.status)}>{gitStatusSymbol(entry.status)}</span>
                      <span class="stack-git-change-row__path" title={entry.relativePath}>
                        {#if entry.relativePath.includes('/')}
                          {@const lastSlash = entry.relativePath.lastIndexOf('/')}
                          <span class="stack-git-change-row__dir">{entry.relativePath.slice(0, lastSlash)}</span><span class="stack-git-change-row__sep">/</span><span class="stack-git-change-row__name">{entry.relativePath.slice(lastSlash + 1)}</span>
                        {:else}
                          <span class="stack-git-change-row__name">{entry.relativePath}</span>
                        {/if}
                      </span>
                    </button>
                    {#if canDiscardEntry(entry)}
                      <button type="button" class="stack-git-change-row__discard" aria-label="Discard" title={`Discard ${entry.relativePath}`} disabled={operationBusy} on:click={() => void discardPaths([entry])}>↺</button>
                    {/if}
                  </div>
                  {#if diffDrawerOpen && entry.path === selectedChangePaths[0] && !diffDrawerStaged}
                    <div class="stack-git-change-diff-drawer" role="region" aria-label={`Diff for ${entry.relativePath}`}>
                      <pre class="stack-git-diff-view" aria-label={`Unified diff for ${entry.relativePath}`}>{#each diffLines as line, index (index)}{@const rendered = renderDiffLineContent(line)}<span class={`stack-git-diff-line stack-git-diff-line--${rendered.kind}`} data-kind={rendered.kind}>{#if rendered.kind === 'meta'}<span class="stack-git-diff-line__meta">{rendered.text}</span>{:else}<span class="stack-git-diff-line__prefix">{rendered.prefix}</span><span class="stack-git-diff-line__body">{rendered.body}</span>{/if}</span>{/each}</pre>
                    </div>
                  {/if}
                {/each}
              {:else}
                <div class="stack-git-empty">No unstaged files</div>
              {/if}
            {/if}
          </section>
        </div>
      {:else if activeView === 'history'}
        <div class="stack-git-stream" role="list">
          <header class="stack-git-stream-header">
            <h3>History</h3>
            <span>{history.length} commits</span>
          </header>
          {#if viewLoading && !history.length}
            <div class="stack-git-empty">Loading history...</div>
          {:else if history.length}
            {#each history as entry (entry.commitHash)}
              <div class="stack-git-row-shell" role="listitem">
                <button type="button" class:selected={selectedHistoryHash === entry.commitHash} class="stack-git-stream-row" on:click={() => selectHistory(entry.commitHash)} on:keydown={(event) => {
                  if (event.key === 'Enter' || event.key === ' ') {
                    event.preventDefault();
                    selectHistory(entry.commitHash);
                  }
                }}>
                  <code>{entry.shortHash}</code>
                  <div>
                    <strong title={entry.subject}>{entry.subject}</strong>
                    <small>{entry.authorName} · {formatDate(entry.authoredAt)}</small>
                  </div>
                </button>
                <div class="stack-git-row-actions">
                  <button type="button" on:click={() => selectHistory(entry.commitHash)}>Diff</button>
                </div>
              </div>
            {/each}
          {:else}
            <div class="stack-git-empty">No commits</div>
          {/if}
        </div>
      {:else if activeView === 'stashes'}
        <div class="stack-git-stream" role="list">
          <header class="stack-git-stream-header">
            <h3>Stashes</h3>
            <span>{stashes.length} items</span>
          </header>
          {#if viewLoading && !stashes.length}
            <div class="stack-git-empty">Loading stashes...</div>
          {:else if stashes.length}
            {#each stashes as stash (normalizeRef(stash))}
              {@const stashRef = normalizeRef(stash)}
              <div class="stack-git-row-shell" role="listitem">
                <button type="button" class:selected={selectedStashRef === stashRef} class="stack-git-stream-row" on:click={() => selectStash(stashRef)} on:keydown={(event) => {
                  if (event.key === 'Enter' || event.key === ' ') {
                    event.preventDefault();
                    selectStash(stashRef);
                  }
                }}>
                  <code>{stashRef}</code>
                  <div>
                    <strong title={normalizeStashLabel(stash)}>{normalizeStashLabel(stash)}</strong>
                    <small>{stash.branch ? `Branch ${stash.branch}` : `stash@{${stash.index}}`}</small>
                  </div>
                </button>
                <div class="stack-git-row-actions">
                  <button type="button" on:click={() => void applyStash(stashRef)}>Apply</button>
                  <button type="button" on:click={() => void popStash(stashRef)}>Pop</button>
                  <button type="button" on:click={() => void dropStash(stashRef)}>Drop</button>
                </div>
              </div>
            {/each}
          {:else}
            <div class="stack-git-empty">No stashes</div>
          {/if}
        </div>
      {:else}
        <div class="stack-git-stream" role="list">
          <header class="stack-git-stream-header">
            <h3>Branches</h3>
            <span>{branches.length} items</span>
          </header>
          {#if dirtyCheckoutWarning}
            <div class="stack-git-warning">{dirtyCheckoutWarning}</div>
          {/if}
          {#if viewLoading && !branches.length}
            <div class="stack-git-empty">Loading branches...</div>
          {:else if branches.length}
            {#each branches as branch (branch.name)}
              <div class="stack-git-row-shell" role="listitem">
                <button type="button" class:selected={selectedBranchName === branch.name} class="stack-git-stream-row" on:click={() => selectBranch(branch.name)} on:keydown={(event) => {
                  if (event.key === 'Enter' || event.key === ' ') {
                    event.preventDefault();
                    selectBranch(branch.name);
                  }
                }}>
                  <code>{branch.current ? '*' : branch.remote ? 'rem' : 'loc'}</code>
                  <div>
                    <strong title={normalizeBranchLabel(branch)}>{normalizeBranchLabel(branch)}</strong>
                    <small>{branch.current ? 'Current branch' : branch.remote ? 'Remote branch' : 'Local branch'}</small>
                  </div>
                </button>
                <div class="stack-git-row-actions">
                  <button type="button" disabled={branch.current || operationBusy} on:click={() => void checkoutBranch(branch.name)}>Checkout</button>
                </div>
              </div>
            {/each}
          {:else}
            <div class="stack-git-empty">No branches</div>
          {/if}

          <form class="stack-git-branch-form" on:submit|preventDefault={() => void checkoutBranch(branchDraft.trim())}>
            <label>
              <span>Checkout</span>
              <input value={branchDraft} placeholder="branch name" on:input={(event) => branchDraft = event.currentTarget.value} />
            </label>
            <button type="submit" disabled={!branchDraft.trim() || operationBusy}>Checkout</button>
          </form>

          <form class="stack-git-branch-form" on:submit|preventDefault={() => void createBranch()}>
            <label>
              <span>Create</span>
              <input value={newBranchDraft} placeholder="new branch" on:input={(event) => newBranchDraft = event.currentTarget.value} />
            </label>
            <button type="submit" disabled={!newBranchDraft.trim() || operationBusy}>Create</button>
          </form>
        </div>
      {/if}

    </section>
  </div>

  {#if errorMessage || statusMessage}
    <div class:error={Boolean(errorMessage)} class="stack-git-operation-status" role="status" aria-live="polite">
      {errorMessage || statusMessage}
    </div>
  {/if}

  {#if activeView === 'changes'}
    <form class="stack-git-commit" on:submit|preventDefault={() => void commitChanges(false)}>
      <div class="stack-git-commit-header">
        <h3>Commit</h3>
        {#if !canCommit}
          <span>Stage files</span>
        {/if}
      </div>

      <textarea
        id="stack-git-commit-message"
        bind:this={commitTextarea}
        bind:value={commitMessage}
        aria-label="Commit message"
        placeholder={canCommit ? `${stagedEntries.length} staged file(s)` : 'Stage files before commit'}
        rows="1"
        on:input={resizeCommitTextarea}
        on:keydown|stopPropagation
      ></textarea>

      <div class="stack-git-commit-actions">
        <button type="button" class="stack-git-commit-stash" disabled={!status || operationBusy || !hasDirtyWorkingTree} on:click={() => void stashChanges()}>Stash</button>
        <div class="stack-git-commit-actions__primary">
          <button type="submit" class="stack-git-button--outline" disabled={!commitMessage.trim() || !canCommit || operationBusy}>Commit</button>
          <button type="button" class="stack-git-button--primary" disabled={!commitMessage.trim() || !canCommit || operationBusy || !remoteUrl} on:click={() => void commitChanges(true)}>Commit &amp; Push</button>
        </div>
      </div>
    </form>
  {/if}

  {#if pendingConfirm}
    <div class="stack-git-confirm-backdrop" role="presentation">
      <button type="button" class="stack-git-confirm-backdrop-hitbox" aria-label="Dismiss confirmation dialog" on:click={closePendingConfirm}></button>
      <div bind:this={pendingConfirmDialogElement} class="stack-git-confirm-dialog" role="dialog" aria-modal="true" tabindex="-1" aria-labelledby="stack-git-confirm-title" aria-describedby="stack-git-confirm-message" on:keydown={handlePendingConfirmKeydown}>
        <h3 id="stack-git-confirm-title">{pendingConfirm.title}</h3>
        <p id="stack-git-confirm-message">{pendingConfirmLabel()}</p>
        <div class="stack-git-confirm-actions">
          <button bind:this={pendingConfirmCancelButton} type="button" on:click={closePendingConfirm}>Cancel</button>
          <button bind:this={pendingConfirmConfirmButton} type="button" class="danger" disabled={operationBusy} on:click={() => void confirmPendingAction()}>Confirm</button>
        </div>
      </div>
    </div>
  {/if}
</section>

<style>
  .stack-git-panel {
    background: var(--js-bg-surface);
    border: 0;
    border-radius: 0;
    box-shadow: none;
    color: var(--js-color-text);
    container-type: inline-size;
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
    overflow: hidden;
    padding: 0;
    position: relative;
  }

  .stack-git-panel button,
  .stack-git-panel input,
  .stack-git-panel textarea,
  .stack-git-panel summary {
    font: inherit;
  }

  .stack-git-panel button {
    background: color-mix(in srgb, var(--js-color-surface-overlay) 68%, transparent);
    border: none;
    border-radius: var(--js-radius-sm);
    color: var(--js-color-text);
    min-height: 32px;
    padding: 0 10px;
  }

  .stack-git-panel button:disabled {
    color: var(--js-color-text-subtle);
    opacity: 0.72;
  }

  .stack-git-panel button:not(:disabled):hover,
  .stack-git-panel button:not(:disabled):focus-visible,
  .stack-git-panel button.active {
    background: color-mix(in srgb, var(--js-color-control-hover) 84%, var(--js-color-accent-border));
    border-color: var(--js-color-accent-border);
    color: var(--js-color-text-strong);
    outline: 0;
  }

  .stack-git-panel-header {
    background: linear-gradient(180deg, color-mix(in srgb, var(--js-color-surface-overlay) 84%, transparent), color-mix(in srgb, var(--js-color-surface-overlay) 68%, transparent));
    border-bottom: 1px solid color-mix(in srgb, var(--js-color-border-soft) 88%, transparent);
    display: grid;
    gap: 10px;
    padding: 10px 12px 8px;
  }

  .stack-git-panel-row {
    align-items: center;
    display: flex;
    gap: 8px;
    min-width: 0;
  }

  .stack-git-panel-row--primary {
    justify-content: space-between;
  }

  .stack-git-panel-row--secondary {
    align-items: center;
    flex-wrap: wrap;
    height: auto;
    margin-top: 10px;
  }

  .stack-git-branch-selector {
    align-items: center;
    background: color-mix(in srgb, var(--js-color-surface-raised) 72%, transparent);
    border: 1px solid color-mix(in srgb, var(--js-color-border-soft) 86%, transparent);
    border-radius: 999px;
    box-shadow: var(--js-shadow-ambient, 0 1px 0 rgba(255, 255, 255, 0.04) inset);
    display: inline-flex;
    gap: 8px;
    min-width: 0;
    padding-inline: 12px;
    width: auto;
  }

  .stack-git-branch-selector__icon,
  .stack-git-branch-selector__label {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .stack-git-branch-selector__icon {
    font-size: 0.85rem;
  }

  .stack-git-panel-header-actions,
  .stack-git-sync-actions,
  .stack-git-commit-actions,
  .stack-git-commit-actions__primary,
  .stack-git-row-actions {
    align-items: center;
    display: flex;
    gap: 8px;
  }

  .stack-git-icon-button {
    align-items: center;
    display: inline-flex;
    height: 32px;
    justify-content: center;
    padding: 0;
    width: 32px;
  }

  .stack-git-icon-button--small {
    height: 24px;
    width: 24px;
  }

  .stack-git-upstream-pill {
    align-items: center;
    background: color-mix(in srgb, var(--js-color-surface-overlay) 78%, transparent);
    border: 1px solid color-mix(in srgb, var(--js-color-border-soft) 92%, transparent);
    border-radius: var(--js-radius-xs);
    color: var(--js-color-text-muted);
    display: inline-flex;
    height: 32px;
    padding: 0 10px;
    white-space: nowrap;
  }

  .stack-git-repository-menu {
    position: relative;
  }

  .stack-git-repository-menu > summary {
    align-items: center;
    background: color-mix(in srgb, var(--js-color-surface-raised) 72%, transparent);
    border: 1px solid color-mix(in srgb, var(--js-color-border-soft) 86%, transparent);
    border-radius: 999px;
    cursor: pointer;
    display: inline-flex;
    height: 32px;
    justify-content: center;
    list-style: none;
    padding: 0;
    width: 32px;
  }

  .stack-git-repository-menu > summary::-webkit-details-marker {
    display: none;
  }

  .stack-git-repository-menu__list {
    background: color-mix(in srgb, var(--js-color-surface-raised) 94%, #101827);
    border: 1px solid color-mix(in srgb, var(--js-color-border) 72%, var(--js-color-accent-border));
    border-radius: var(--js-radius-sm);
    box-shadow: var(--js-shadow-raised);
    display: grid;
    gap: 4px;
    left: 0;
    min-width: 11rem;
    padding: 8px;
    position: absolute;
    top: calc(100% + 6px);
    z-index: 20;
  }

  .stack-git-repository-menu__list button {
    justify-content: flex-start;
    width: 100%;
  }

  .stack-git-panel-content {
    display: flex;
    flex: 1 1 auto;
    min-height: 0;
    overflow: hidden;
  }

  .stack-git-operation-status {
    color: var(--js-color-text-muted);
    font-size: 0.75rem;
    overflow: hidden;
    padding: 4px 12px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .stack-git-operation-status.error {
    color: var(--js-color-danger-text, #ff8a8a);
  }

  .stack-git-panel-scroll {
    align-content: start;
    display: grid;
    flex: 1 1 auto;
    gap: 12px;
    min-height: 0;
    overflow-x: hidden;
    overflow-y: auto;
    overscroll-behavior: contain;
    padding: 12px;
    scrollbar-gutter: stable;
  }

  .stack-git-change-groups,
  .stack-git-stream {
    display: grid;
    gap: 12px;
    min-width: 0;
  }

  .stack-git-change-group {
    display: grid;
    gap: 0;
  }

  .stack-git-change-group-header {
    align-items: center;
    background: linear-gradient(180deg, color-mix(in srgb, var(--js-color-surface-overlay) 82%, transparent), color-mix(in srgb, var(--js-color-surface-overlay) 64%, transparent));
    backdrop-filter: blur(8px);
    display: flex;
    gap: 8px;
    min-height: 32px;
    padding: 8px;
    position: sticky;
    top: 0;
    z-index: 2;
  }

  .stack-git-panel button.stack-git-change-group-header__bulk,
  .stack-git-panel button.stack-git-change-row__action,
  .stack-git-panel button.stack-git-change-row__discard {
    appearance: none;
    background: transparent;
    border: 0;
    border-radius: 0;
    color: inherit;
    cursor: pointer;
    font: inherit;
    height: 20px;
    align-items: center;
    justify-content: center;
    min-height: 0;
    outline: none;
    padding: 0;
    display: inline-flex;
    width: 20px;
  }

  .stack-git-panel button.stack-git-change-group-header__bulk:hover:not(:disabled),
  .stack-git-panel button.stack-git-change-group-header__bulk:active:not(:disabled),
  .stack-git-panel button.stack-git-change-row__action:hover:not(:disabled),
  .stack-git-panel button.stack-git-change-row__action:active:not(:disabled),
  .stack-git-panel button.stack-git-change-row__discard:hover:not(:disabled),
  .stack-git-panel button.stack-git-change-row__discard:active:not(:disabled) {
    background: transparent;
    color: inherit;
  }

  .stack-git-panel button.stack-git-change-group-header__bulk:focus-visible,
  .stack-git-panel button.stack-git-change-row__action:focus-visible,
  .stack-git-panel button.stack-git-change-row__discard:focus-visible {
    box-shadow: var(--js-focus-ring);
  }

  .stack-git-change-group-header__title {
    align-items: center;
    background: transparent;
    border: 0;
    display: flex;
    flex: 1 1 auto;
    gap: 8px;
    justify-content: flex-start;
    min-width: 0;
    padding: 0;
  }

  .stack-git-change-group-header__title span:first-child {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .stack-git-change-row {
    align-items: center;
    background: color-mix(in srgb, var(--js-color-surface-overlay) 68%, transparent);
    display: flex;
    gap: 8px;
    height: 34px;
    min-height: 34px;
    padding: 0 8px;
  }

  .stack-git-change-row:hover,
  .stack-git-change-row:focus-within {
    background: color-mix(in srgb, var(--js-color-control-hover) 84%, var(--js-color-accent-border));
  }

  .stack-git-panel button.stack-git-change-row__content,
  .stack-git-panel button.stack-git-change-row__content:hover:not(:disabled),
  .stack-git-panel button.stack-git-change-row__content:focus-visible {
    align-items: center;
    background: transparent;
    border: 0;
    color: inherit;
    cursor: pointer;
    display: flex;
    flex: 1 1 auto;
    gap: 8px;
    height: 34px;
    min-width: 0;
    outline: 0;
    padding: 0 4px;
    text-align: left;
  }

  .stack-git-change-row__status {
    align-items: center;
    display: inline-flex;
    height: 16px;
    justify-content: center;
    width: 16px;
  }

  .stack-git-file-glyph {
    border: 1px solid color-mix(in srgb, var(--js-color-text-muted) 60%, transparent);
    border-radius: 3px;
    display: inline-block;
    flex: 0 0 auto;
    height: 14px;
    opacity: 0.72;
    position: relative;
    width: 12px;
  }

  .stack-git-file-glyph::after {
    border-top: 1px solid color-mix(in srgb, var(--js-color-text-muted) 60%, transparent);
    border-right: 1px solid color-mix(in srgb, var(--js-color-text-muted) 60%, transparent);
    content: '';
    height: 4px;
    position: absolute;
    right: -1px;
    top: -1px;
    width: 4px;
  }

  .stack-git-change-row__path {
    align-items: center;
    display: flex;
    flex: 1 1 auto;
    gap: 0;
    min-width: 0;
    overflow: hidden;
  }

  .stack-git-change-row__dir,
  .stack-git-change-row__sep {
    color: var(--js-color-text-muted);
  }

  .stack-git-change-row__dir,
  .stack-git-change-row__name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .stack-git-change-row__name {
    color: var(--js-color-text);
  }

  .stack-git-stream-row {
    align-items: center;
    background: color-mix(in srgb, var(--js-color-surface-overlay) 56%, transparent);
    border: 1px solid color-mix(in srgb, var(--js-color-border-soft) 88%, transparent);
    border-radius: var(--js-radius-sm);
    display: flex;
    flex: 1 1 auto;
    gap: 8px;
    height: 34px;
    min-width: 0;
    padding: 0 12px;
    text-align: left;
  }

  .stack-git-row-shell {
    align-items: center;
    display: flex;
    gap: 8px;
    min-height: 34px;
  }

  .stack-git-row-actions {
    opacity: 0;
    transition: opacity 120ms ease;
  }

  .stack-git-row-shell:hover .stack-git-row-actions,
  .stack-git-row-shell:focus-within .stack-git-row-actions {
    opacity: 1;
  }

  .stack-git-badge {
    align-items: center;
    border-radius: var(--js-radius-xs);
    display: inline-flex;
    font-size: 0.65rem;
    font-weight: 800;
    height: 1rem;
    justify-content: center;
    width: 1rem;
  }

  .stack-git-empty,
  .stack-git-warning {
    background: color-mix(in srgb, var(--js-color-surface-overlay) 62%, transparent);
    border: 1px solid color-mix(in srgb, var(--js-color-border-soft) 92%, transparent);
    border-radius: var(--js-radius-sm);
    color: var(--js-color-text-muted);
    font-size: 0.7rem;
    padding: 10px 12px;
  }

  .stack-git-warning {
    color: var(--js-color-text);
  }

  .stack-git-change-diff-drawer {
    background: var(--js-color-surface);
    border-left: 2px solid var(--js-color-accent-border);
    box-sizing: border-box;
    display: grid;
    min-width: 0;
    font: 0.67rem/1.45 ui-monospace, "Cascadia Mono", Consolas, monospace;
    margin: 0;
    overflow: auto;
    padding: 0.45rem;
    user-select: text;
    white-space: pre-wrap;
    cursor: text;
    width: 100%;
    animation: stack-git-drawer-in 160ms cubic-bezier(0.22, 1, 0.36, 1) both;
    transform-origin: top center;
  }

  .stack-git-stream-header h3 {
    margin: 0;
  }

  .stack-git-change-diff-drawer pre {
    background: transparent;
    border: 0;
    color: inherit;
    font: inherit;
    margin: 0;
    min-height: 0;
    overflow: auto;
    padding: 0;
    tab-size: 4;
    white-space: inherit;
  }

  .stack-git-diff-view { color: var(--js-color-text); }

  .stack-git-diff-line {
    align-items: start;
    border-left: 3px solid transparent;
    display: grid;
    grid-template-columns: 1.25ch minmax(0, 1fr);
    gap: 0.25rem;
    min-height: 1.45em;
    margin: 0;
    padding: 0.06rem 0.35rem 0.06rem 0.45rem;
    white-space: pre-wrap;
  }

  .stack-git-diff-line__prefix {
    font-weight: 700;
    text-align: right;
  }

  .stack-git-diff-line__body,
  .stack-git-diff-line__meta {
    min-width: 0;
  }

  .stack-git-diff-line--meta {
    background: var(--js-color-accent-soft);
    border-left-color: var(--js-color-accent-border);
    color: var(--js-color-text);
    display: block;
    padding-inline: 0.45rem;
  }

  .stack-git-diff-line--meta .stack-git-diff-line__meta {
    display: block;
    overflow-wrap: anywhere;
    white-space: pre-wrap;
  }

  .stack-git-diff-line--hunk {
    background: var(--js-color-warning);
    border-left-color: var(--js-color-warning-border);
    color: var(--js-color-text-strong);
    font-weight: 700;
  }

  .stack-git-diff-line--addition {
    background: var(--js-color-success);
    border-left-color: var(--js-color-success-border);
    color: var(--js-color-text-strong);
  }

  .stack-git-diff-line--deletion {
    background: var(--js-color-error);
    border-left-color: var(--js-color-error-border);
    color: var(--js-color-text-strong);
  }

  .stack-git-diff-line--addition .stack-git-diff-line__prefix {
    color: color-mix(in srgb, var(--js-color-success-border) 75%, var(--js-color-text-strong));
  }

  .stack-git-diff-line--deletion .stack-git-diff-line__prefix {
    color: color-mix(in srgb, var(--js-color-error-border) 75%, var(--js-color-text-strong));
  }

  .stack-git-diff-line--hunk .stack-git-diff-line__prefix {
    color: color-mix(in srgb, var(--js-color-warning-border) 75%, var(--js-color-text-strong));
  }

  .stack-git-diff-line--context {
    color: var(--js-color-text);
  }

  .stack-git-diff-line--empty {
    color: transparent;
    min-height: 0.9em;
  }

  .stack-git-commit {
    border-top: 1px solid color-mix(in srgb, var(--js-color-border-soft) 88%, transparent);
    background: linear-gradient(180deg, color-mix(in srgb, var(--js-color-surface-overlay) 76%, transparent), color-mix(in srgb, var(--js-color-surface-overlay) 64%, transparent));
    display: grid;
    gap: 10px;
    padding: 12px;
  }

  .stack-git-commit-header {
    align-items: baseline;
    display: flex;
    gap: 8px;
    justify-content: space-between;
  }

  .stack-git-commit-header h3,
  .stack-git-commit-header span {
    margin: 0;
  }

  .stack-git-commit-header h3 {
    font-size: 0.68rem;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }

  .stack-git-commit textarea {
    background: color-mix(in srgb, var(--js-color-surface-raised) 90%, transparent);
    border: 1px solid color-mix(in srgb, var(--js-color-border) 84%, transparent);
    border-radius: var(--js-radius-md);
    color: var(--js-color-text);
    min-height: 38px;
    max-height: 200px;
    padding: 8px 12px;
    resize: none;
  }

  .stack-git-commit textarea:focus,
  .stack-git-branch-form input:focus,
  .stack-git-panel button:focus-visible {
    border-color: var(--js-color-accent-border);
    box-shadow: var(--js-focus-ring);
    outline: 0;
  }

  .stack-git-commit-actions {
    justify-content: space-between;
    flex-wrap: wrap;
  }

  .stack-git-commit-actions__primary {
    margin-left: auto;
    flex-wrap: wrap;
  }

  .stack-git-button--outline {
    background: transparent;
  }

  .stack-git-button--primary {
    background: var(--js-color-accent-border);
    color: var(--js-color-text-strong);
  }

  .stack-git-commit-stash {
    background: color-mix(in srgb, var(--js-color-surface-overlay) 60%, transparent);
  }

  .stack-git-branch-form {
    display: grid;
    gap: 4px;
    grid-template-columns: minmax(0, 1fr) auto;
  }

  .stack-git-branch-form label {
    display: grid;
    gap: 4px;
    min-width: 0;
  }

  .stack-git-branch-form span {
    color: var(--js-color-text-muted);
    font-size: 0.58rem;
    font-weight: 800;
    text-transform: uppercase;
  }

  .stack-git-branch-form input {
    background: var(--js-color-surface-sunken);
    border: 1px solid var(--js-color-border);
    border-radius: var(--js-radius-sm);
    color: var(--js-color-text);
    min-height: 32px;
    padding: 0 10px;
  }

  .stack-git-confirm-backdrop {
    background: rgba(0, 0, 0, 0.56);
    display: grid;
    inset: 0;
    place-items: center;
    padding: 12px;
    position: fixed;
    z-index: 100;
  }

  .stack-git-confirm-backdrop-hitbox {
    background: transparent;
    border: 0;
    inset: 0;
    min-height: 0;
    min-width: 0;
    padding: 0;
    position: absolute;
  }

  .stack-git-confirm-dialog {
    background: color-mix(in srgb, var(--js-color-surface-raised) 94%, #101827);
    border: 1px solid color-mix(in srgb, var(--js-color-border) 72%, var(--js-color-accent-border));
    border-radius: var(--js-radius-md);
    box-shadow: var(--js-shadow-raised);
    display: grid;
    gap: 12px;
    max-width: min(28rem, calc(100vw - 2rem));
    padding: 12px;
    position: relative;
    z-index: 1;
  }

  .stack-git-confirm-dialog h3,
  .stack-git-confirm-dialog p {
    margin: 0;
  }

  .stack-git-confirm-dialog p {
    color: var(--js-color-text-muted);
    font-size: 0.72rem;
    line-height: 1.45;
  }

  .stack-git-confirm-actions {
    justify-content: flex-end;
  }

  .stack-git-confirm-actions .danger {
    background: var(--js-color-error);
    border-color: var(--js-color-error-border);
    color: var(--js-color-text-strong);
  }

  @container (max-width: 42rem) {
    .stack-git-panel-row--secondary {
      align-items: flex-start;
      height: auto;
      flex-wrap: wrap;
    }

    .stack-git-commit-actions {
      align-items: flex-start;
    }

    .stack-git-commit-actions__primary {
      margin-left: 0;
    }
  }

  @container (max-width: 32rem) {
    .stack-git-change-diff-drawer {
      padding: 0.35rem;
    }

    .stack-git-diff-line {
      gap: 0.2rem;
      padding-inline: 0.3rem;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .stack-git-change-diff-drawer {
      animation: none;
    }
  }

  @keyframes stack-git-drawer-in {
    from {
      opacity: 0;
      transform: scaleY(0.96);
    }

    to {
      opacity: 1;
      transform: scaleY(1);
    }
  }
</style>
