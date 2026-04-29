<script lang="ts">
  import './ControlPlaneSurface.css';
  import { Tabs } from 'melt/builders';
  import MeltActionButton from './melt/MeltActionButton.svelte';
  import MeltSelect from './melt/MeltSelect.svelte';
  import {
    buildControlPlaneViewModel,
    controlPlaneActionLabel,
    controlPlaneSectionTabLabel,
    filterControlPlaneSections,
    type ControlPlaneLoadState,
    type ControlPlaneProviderBudget,
    type ControlPlaneSectionId
  } from '../features/control-plane/controlPlaneState';
  import type { GitWorkspaceStatus, TaskHistoryEntry, TaskProcessMetadata } from '../lib/devTools';
  import type { ProcessInfo } from '../lib/processManager';
  import type { ShellSettings } from '../lib/settings';
  import {
    getInitialShellThemeId,
    normalizeShellThemeId,
    setShellTheme,
    shellThemeOptions,
    type ShellThemeId
  } from '../lib/themes';
  import type { DeveloperSearchResponse } from '../features/search/developerProviders';
  import type { WorkspaceProfile } from '../lib/workspaces';

  export let settings: ShellSettings | null = null;
  export let settingsState: ControlPlaneLoadState = 'idle';
  export let workspaces: WorkspaceProfile[] = [];
  export let activeWorkspaceId: string | null = null;
  export let gitStatuses: Record<string, GitWorkspaceStatus | undefined> = {};
  export let taskHistory: TaskHistoryEntry[] = [];
  export let taskProcessMetadata: TaskProcessMetadata[] = [];
  export let processes: ProcessInfo[] = [];
  export let providerResponse: DeveloperSearchResponse | null = null;
  export let providerBudget: Partial<ControlPlaneProviderBudget> = {};

  let activeSectionId: ControlPlaneSectionId = 'settings';
  let filterQuery = '';
  let selectedThemeId: ShellThemeId = getInitialShellThemeId();
  const themeOptions = shellThemeOptions();
  const themeSelectOptions = themeOptions.map((theme) => ({ value: theme.id, label: theme.label }));
  const sectionTabs = new Tabs<ControlPlaneSectionId>({
    value: () => activeSectionId,
    onValueChange: (sectionId) => {
      activeSectionId = sectionId;
    },
    orientation: 'horizontal',
    loop: true,
    selectWhenFocused: true
  });

  $: viewModel = buildControlPlaneViewModel({
    settings,
    settingsState,
    workspaces,
    activeWorkspaceId,
    gitStatuses,
    taskHistory,
    taskProcessMetadata,
    processes,
    providerResponse,
    providerBudget
  });
  $: visibleSections = filterControlPlaneSections(viewModel.sections, filterQuery);
  $: if (visibleSections.length && !visibleSections.some((section) => section.id === activeSectionId)) {
    activeSectionId = visibleSections[0].id;
  }

  function handleThemeChange(value: string) {
    selectedThemeId = normalizeShellThemeId(value);
    setShellTheme(selectedThemeId);
  }
</script>

<main class="control-plane-surface" aria-labelledby="control-plane-title">
  <header class="control-plane-hero">
    <div>
      <p class="eyebrow">Control Plane</p>
      <h1 id="control-plane-title">Settings & Developer Dashboard</h1>
      <p>
        Frontend summary over existing settings, workspace, git, task, process, and provider
        contracts. Secrets and unbounded source lists stay out of this surface.
      </p>
    </div>
    <dl aria-label="Control-plane totals">
      <div>
        <dt>Workspaces</dt>
        <dd>{viewModel.totals.workspaceCount}</dd>
      </div>
      <div>
        <dt>Tasks</dt>
        <dd>{viewModel.totals.runningTaskCount}</dd>
      </div>
      <div>
        <dt>Processes</dt>
        <dd>{viewModel.totals.processCount}</dd>
      </div>
      <div>
        <dt>Provider hits</dt>
        <dd>{viewModel.totals.providerResultCount}</dd>
      </div>
    </dl>
  </header>

  <div class="control-plane-toolbar">
    <label>
      <span>Filter sections</span>
      <input
        type="search"
        bind:value={filterQuery}
        placeholder="settings, git, process..."
        aria-describedby="control-plane-filter-hint"
      />
    </label>
    <p id="control-plane-filter-hint">
      Use arrow keys on section tabs to move, Home/End for edges, and Ctrl+R for refresh intent.
    </p>
    <MeltSelect
      class="theme-picker"
      label="Theme"
      value={selectedThemeId}
      options={themeSelectOptions}
      placeholder="Shell theme"
      onChange={handleThemeChange}
    />
  </div>

  <div
    class="control-plane-tabs"
    aria-label="Control-plane sections"
    {...sectionTabs.triggerList}
  >
    {#each visibleSections as section}
      <button
        {...sectionTabs.getTrigger(section.id)}
        type="button"
        class:active={section.id === activeSectionId}
        aria-label={controlPlaneSectionTabLabel(section, section.id === activeSectionId)}
      >
        <strong>{section.label}</strong>
        <span>{section.count}</span>
      </button>
    {/each}
  </div>

  <div class="control-plane-grid">
    {#each visibleSections as section}
      {@const tabContent = sectionTabs.getContent(section.id)}
      <section
        {...tabContent}
        hidden={false}
        class:active-card={section.id === activeSectionId}
        aria-describedby={`control-plane-heading-${section.id}`}
      >
        <div class="section-head">
          <div>
            <h2 id={`control-plane-heading-${section.id}`}>{section.label}</h2>
            <p>{section.description}</p>
          </div>
          <span class="section-status" aria-label={section.ariaLabel}>{section.status}</span>
        </div>

        <div class="section-actions" aria-label={`${section.label} actions`}>
          {#each section.actions as action}
            <MeltActionButton
              disabled={action.disabled}
              ariaLabel={action.ariaLabel}
              title={controlPlaneActionLabel(section, action)}
            >
              {action.label}
            </MeltActionButton>
          {/each}
        </div>

        {#if section.items.length}
          <ul class="summary-list" aria-label={`${section.label} summary items`}>
            {#each section.items as item}
              <li class={`tone-${item.tone}`}>
                <strong>{item.title}</strong>
                <span>{item.meta}</span>
                {#if item.detail}
                  <p>{item.detail}</p>
                {/if}
              </li>
            {/each}
          </ul>
        {:else}
          <p class="empty-state">No bounded summary data is available yet.</p>
        {/if}
      </section>
    {/each}
  </div>
</main>
