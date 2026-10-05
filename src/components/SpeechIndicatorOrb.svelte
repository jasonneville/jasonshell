<script lang="ts">
  // Rare UI / Swami Malode, copyright 2026. See ../lib/speechIndicatorOrb.LICENSE.
  import { onMount } from 'svelte';
  import { createSpeechIndicatorOrb } from '../lib/speechIndicatorOrb';
  let { active = false }: { active?: boolean } = $props();
  let canvas: HTMLCanvasElement;
  let renderer = $state.raw<ReturnType<typeof createSpeechIndicatorOrb> | null>(null);
  onMount(() => {
    renderer = createSpeechIndicatorOrb(canvas);
    return () => renderer?.dispose();
  });
  $effect(() => { renderer?.setActive(active); });
</script>

<canvas bind:this={canvas} aria-hidden="true"></canvas>

<style>
  canvas { position: absolute; inset: 0; height: 100%; width: 100%; pointer-events: none; }
  @media (forced-colors: active) { canvas { display: none; } }
</style>
