<script lang="ts">
  let { path, onDirtyChange, onDismiss, onRevisionChange = () => {} }: {
    path: string;
    onDirtyChange: (dirty: boolean) => void;
    onDismiss: (dirty: boolean) => void;
    onRevisionChange?: () => void;
  } = $props();
  let dirty = $state(false);
  let revision = $state(1);
  export function hasExitOperationPending() { return false; }
  export function snapshotExitDraft() { return { revision, clean: !dirty, busy: false, invalid: false }; }
  function edit() { revision += 1; dirty = true; onRevisionChange(); onDirtyChange(true); }
</script>
<button type="button" aria-label={`Make draft dirty: ${path}`} onclick={edit}>Make draft dirty</button>
<button type="button" onclick={() => onDismiss(true)}>Editor back</button>
