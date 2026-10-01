<script lang="ts">
  export let path: string;
  export let onDirtyChange: (dirty: boolean) => void;
  export let onDismiss: (dirty: boolean) => void;
</script>
<button type="button" aria-label={`Make draft dirty: ${path}`} on:click={() => onDirtyChange(true)}>Make draft dirty</button>
<button type="button" on:click={() => onDismiss(true)}>Editor back</button>
