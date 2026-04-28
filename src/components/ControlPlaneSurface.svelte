<script lang="ts">
  import './ControlPlaneSurface.css';
  import {
    buildControlPlaneViewModel,
    controlPlaneActionLabel,
    controlPlaneKeyActionFromEvent,
    controlPlaneSectionTabLabel,
    filterControlPlaneSections,
    nextControlPlaneSectionId,
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

  function handleSectionKeydown(event: KeyboardEvent) {
    const action = controlPlaneKeyActionFromEvent(event);
    if (action === 'none') {
      return;
    }

    if (action === 'focus-next-section' || action === 'focus-previous-section') {
      event.preventDefault();
      activeSectionId = nextControlPlaneSectionId(
        visibleSections,
        activeSectionId,
        action === 'focus-next-section' ? 'next' : 'previous'
      );
      return;
    }

    if (action === 'focus-first-section' && visibleSections[0]) {
      event.preventDefault();
      activeSectionId = visibleSections[0].id;
      return;
    }

    if (action === 'focus-last-section' && visibleSections.at(-1)) {
      event.preventDefault();
      activeSectionId = visibleSections.at(-1)!.id;
    }
  }

  function handleThemeChange(event: Event) {
    const target = event.currentTarget instanceof HTMLSelectElement ? event.currentTarget : null;
    selectedThemeId = normalizeShellThemeId(target?.value);
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
    <label class="theme-picker">
      <span>Theme</span>
      <select bind:value={selectedThemeId} on:change={handleThemeChange} aria-label="Shell theme">
        {#each themeOptions as theme}
          <option value={theme.id}>{theme.label}</option>
        {/each}
      </select>
    </label>
  </div>

  <div
    class="control-plane-tabs"
    aria-label="Control-plane sections"
    role="tablist"
  >
    {#each visibleSections as section}
      <button
        type="button"
        role="tab"
        aria-selected={section.id === activeSectionId}
        aria-controls={`control-plane-section-${section.id}`}
        tabindex={section.id === activeSectionId ? 0 : -1}
        class:active={section.id === activeSectionId}
        aria-label={controlPlaneSectionTabLabel(section, section.id === activeSectionId)}
        on:keydown={handleSectionKeydown}
        on:click={() => activeSectionId = section.id}
      >
        <strong>{section.label}</strong>
        <span>{section.count}</span>
      </button>
    {/each}
  </div>

  <div class="control-plane-grid">
    {#each visibleSections as section}
      <section
        id={`control-plane-section-${section.id}`}
        class:active-card={section.id === activeSectionId}
        aria-labelledby={`control-plane-heading-${section.id}`}
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
            <button
              type="button"
              disabled={action.disabled}
              aria-label={action.ariaLabel}
              title={controlPlaneActionLabel(section, action)}
            >
              {action.label}
            </button>
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
