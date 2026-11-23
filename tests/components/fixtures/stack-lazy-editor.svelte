<script lang="ts">
  // Test-owned stand-in for StackTextEditor with the same parent-facing contract.
  let { path, onDirtyChange, onDismiss }: {
    path: string;
    onDirtyChange: (dirty: boolean) => void;
    onDismiss: (dirty: boolean) => void;
    onSaved?: (() => void) | null;
  } = $props();
  let dirty = $state(false);
  export function hasPendingFileOperation() { return false; }
  export function handleMouseNavigation(_direction: -1 | 1) { onDismiss(dirty); return 'consumed' as const; }
  function edit() { dirty = true; onDirtyChange(true); }
</script>
<section role="region" aria-label="Lazy editor fixture">
  <button type="button" onclick={edit}>Make lazy draft dirty: {path}</button>
  <button type="button" onclick={() => onDismiss(dirty)}>Editor back</button>
</section>
