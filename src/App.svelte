<script lang="ts">
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { onMount } from 'svelte';
  import BottomBar from './components/BottomBar.svelte';
  import AudioPanelSurface from './components/AudioPanelSurface.svelte';
  import CommandPanelSurface from './components/CommandPanelSurface.svelte';
  import ControlPlaneSurface from './components/ControlPlaneSurface.svelte';
  import ProcessManagerSurface from './components/ProcessManagerSurface.svelte';
  import SearchPanelSurface from './components/SearchPanelSurface.svelte';
  import SettingsPanelSurface from './components/SettingsPanelSurface.svelte';
  import StackPopupSurface from './components/StackPopupSurface.svelte';
  import TaskPreviewSurface from './components/TaskPreviewSurface.svelte';
  import TrayPanelSurface from './components/TrayPanelSurface.svelte';
  import TopBar from './components/TopBar.svelte';
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

  function suppressNativeContextMenu(event: MouseEvent) {
    event.preventDefault();
  }

  onMount(() => {
    const uninstallThemeSync = installShellThemeSync();
    const uninstallPreferencesSync = installShellPreferencesSync();
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

{#if surface === 'top-bar'}
  <TopBar />
{:else if surface === 'bottom-bar'}
  <BottomBar />
{:else if surface === 'task-preview'}
  <TaskPreviewSurface />
{:else if surface === 'search-panel'}
  <SearchPanelSurface />
{:else if surface === 'stack-popup'}
  <StackPopupSurface />
{:else if surface === 'process-manager'}
  <ProcessManagerSurface />
{:else if surface === 'control-plane'}
  <ControlPlaneSurface />
{:else if surface === 'settings-panel'}
  <SettingsPanelSurface />
{:else if surface === 'tray-panel'}
  <TrayPanelSurface />
{:else if surface === 'command-panel'}
  <CommandPanelSurface />
{:else if surface === 'audio-panel'}
  <AudioPanelSurface />
{:else}
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
