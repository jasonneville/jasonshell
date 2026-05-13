<script lang="ts">
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { onMount } from 'svelte';
  import { loadSurfaceComponent, type SurfaceComponent as LoadedSurfaceComponent } from './lib/surfaceLoader';
  import { installShellPreferencesSync } from './lib/shellPreferences';
  import {
    resolveSurfaceFromLabel,
    shellSurfaceMetadata,
    type ShellSurface
  } from './lib/shellSurface';
  import { installShellThemeSync } from './lib/themes';

  let label = 'bottom-bar';

  try {
    label = getCurrentWindow().label;
  } catch (_error) {
    label = 'bottom-bar';
  }

  const surface: ShellSurface = resolveSurfaceFromLabel(label);
  const metadata = shellSurfaceMetadata[surface];
  let SurfaceComponent: LoadedSurfaceComponent | null = null;
  let surfaceLoadFailed = false;

  function suppressNativeContextMenu(event: MouseEvent) {
    event.preventDefault();
  }

  onMount(() => {
    const uninstallThemeSync = installShellThemeSync();
    const uninstallPreferencesSync = installShellPreferencesSync();

    if (surface !== 'unknown') {
      loadSurfaceComponent(surface)
        ?.then((module) => {
          SurfaceComponent = module.default;
        })
        .catch((error) => {
          surfaceLoadFailed = true;
          console.error(`JasonShell failed to load surface component for ${surface}`, error);
        });
    }

    return () => {
      uninstallThemeSync();
      uninstallPreferencesSync();
    };
  });
</script>

<svelte:head>
  <title>{metadata.title}</title>
</svelte:head>

<svelte:window on:contextmenu={suppressNativeContextMenu} />

{#if SurfaceComponent}
  <SurfaceComponent />
{:else if surface === 'unknown' || surfaceLoadFailed}
  <main class="unsupported-surface">
    <div class="panel">
      <strong>{metadata.title}</strong>
      <p>{metadata.subtitle}</p>
    </div>
  </main>
{/if}

<style>
  .unsupported-surface {
    align-items: center;
    background: #0d1118;
    color: #f3f5ff;
    display: grid;
    height: 100%;
    justify-items: center;
    margin: 0;
    width: 100%;
  }

  .panel {
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 1rem;
    padding: 1.25rem 1.5rem;
  }

  .panel strong {
    display: block;
    font-size: 1rem;
    margin-bottom: 0.35rem;
  }

  .panel p {
    color: rgba(229, 234, 250, 0.72);
    margin: 0;
  }
</style>
