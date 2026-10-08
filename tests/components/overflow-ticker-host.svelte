<script lang="ts">
  import OverflowTicker from '../../src/components/OverflowTicker.svelte';
  let { text }: { text: string } = $props();
</script>

<button type="button"><span aria-hidden="true">Application icon</span><OverflowTicker {text} /></button>
