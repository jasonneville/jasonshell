<script lang="ts">
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { onMount } from 'svelte';
  import { installShellPreferencesSync } from './lib/shellPreferences';
  import {
    resolveSurfaceFromLabel,
    shellSurfaceMetadata,
    type ShellSurface
  } from './lib/shellSurface';
  import { loadSurfaceComponent, type SurfaceComponent as LoadedSurfaceComponent } from './lib/surfaceLoader';
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
    let mounted = true;
    const uninstallThemeSync = installShellThemeSync();
    const uninstallPreferencesSync = installShellPreferencesSync();
    const loadableSurface = loadSurfaceComponent(surface);

    if (loadableSurface) {
      void loadableSurface
        .then((module) => {
          if (mounted) {
            SurfaceComponent = module.default;
          }
        })
        .catch((error) => {
          console.error(`JasonShell failed to load surface component for ${surface}`, error);
          if (mounted) {
            surfaceLoadFailed = true;
          }
        });
    }

    return () => {
      mounted = false;
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
{:else if surfaceLoadFailed || surface === 'unknown'}
  <main class="unsupported-surface" data-surface={surface} data-label={label}>
    <div class="panel">
      <strong>{metadata.title}</strong>
      <p>{metadata.subtitle}</p>
      {#if surfaceLoadFailed}
        <p class="diagnostic">Failed to load the {surface} surface. Check developer console diagnostics.</p>
      {/if}
    </div>
  </main>
{:else}
  <main class="unsupported-surface loading-surface" data-surface={surface} data-label={label} aria-busy="true">
    <div class="panel">
      <strong>{metadata.title}</strong>
      <p>Loading {metadata.subtitle.toLowerCase()}…</p>
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

  .panel .diagnostic {
    color: rgba(255, 180, 180, 0.82);
    margin-top: 0.65rem;
  }
</style>
