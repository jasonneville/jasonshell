<script lang="ts">
  // Test-owned stand-in for StackGitPanel with the same parent-facing contract.
  let { folderPath = '', onClose = null }: {
    folderPath?: string;
    initialStatus?: unknown;
    initialChangeFilter?: string;
    onClose?: (() => void) | null;
    onRefresh?: unknown;
  } = $props();
  export function handleMouseNavigation(direction: -1 | 1) {
    if (direction === -1) onClose?.();
    return 'consumed' as const;
  }
</script>
<section role="region" aria-label="Lazy git panel fixture" data-folder={folderPath}>
  <button type="button" onclick={() => onClose?.()}>Close lazy git panel</button>
</section>
