<script lang="ts">
  import './SettingsPanelSurface.css';
  import {
    formatShellDate,
    formatShellTime,
    getInitialShellPreferences,
    patchShellPreferences,
    setShellPreferences,
    shellFontOptions,
    type ShellPreferences
  } from '../lib/shellPreferences';
  import { hideSettingsPanel } from '../lib/settingsPanel';
  import {
    getInitialShellThemeId,
    normalizeShellThemeId,
    setShellTheme,
    shellThemeOptions,
    type ShellThemeId
  } from '../lib/themes';

  const themeOptions = shellThemeOptions();
  const fontOptions = shellFontOptions();
  const dateFormatExamples = [
    'EEE, MMM d',
    'EEEE, MMMM d',
    'yyyy-MM-dd',
    'MM/dd/yyyy'
  ];

  let preferences: ShellPreferences = getInitialShellPreferences();
  let selectedThemeId: ShellThemeId = getInitialShellThemeId();
  let now = new Date();

  $: datePreview = formatShellDate(now, preferences.dateFormat);
  $: timePreview = formatShellTime(now, preferences);

  function updatePreferences(patch: Partial<ShellPreferences>) {
    preferences = patchShellPreferences(patch);
  }

  function handleThemeChange(event: Event) {
    const target = event.currentTarget instanceof HTMLSelectElement ? event.currentTarget : null;
    selectedThemeId = normalizeShellThemeId(target?.value);
    setShellTheme(selectedThemeId);
  }

  function handleFontChange(event: Event) {
    const target = event.currentTarget instanceof HTMLSelectElement ? event.currentTarget : null;
    updatePreferences({ fontId: target?.value as ShellPreferences['fontId'] });
  }

  function handleDateFormatInput(event: Event) {
    const target = event.currentTarget instanceof HTMLInputElement ? event.currentTarget : null;
    updatePreferences({ dateFormat: target?.value ?? '' });
  }

  function resetPresentation() {
    preferences = setShellPreferences({
      fontId: 'open-sans',
      dateFormat: 'EEE, MMM d',
      use24HourTime: false,
      showSeconds: true,
      compactDensity: false,
      strongFocusRing: false,
      reducedTransparency: false,
      showSearchShortcutHint: true
    });
    selectedThemeId = 'base-dark';
    setShellTheme(selectedThemeId);
  }

  function closePanel() {
    void hideSettingsPanel().catch((error) => {
      console.error('Failed to hide settings panel', error);
    });
  }
</script>

<svelte:window
  on:keydown={(event) => {
    if (event.key === 'Escape') {
      event.preventDefault();
      closePanel();
    }
  }}
/>

<main class="settings-panel" aria-labelledby="settings-panel-title">
  <header class="settings-panel-header">
    <div>
      <p>JasonShell</p>
      <h1 id="settings-panel-title">Settings</h1>
    </div>
    <button type="button" aria-label="Close settings" on:click={closePanel}>x</button>
  </header>

  <section class="settings-section" aria-labelledby="appearance-heading">
    <h2 id="appearance-heading">Appearance</h2>
    <label>
      <span>Theme</span>
      <select bind:value={selectedThemeId} on:change={handleThemeChange}>
        {#each themeOptions as theme}
          <option value={theme.id}>{theme.label}</option>
        {/each}
      </select>
    </label>

    <label>
      <span>Font</span>
      <select value={preferences.fontId} on:change={handleFontChange}>
        {#each fontOptions as font}
          <option value={font.id}>{font.label}</option>
        {/each}
      </select>
    </label>

    <div class="settings-toggle-grid">
      <label>
        <input
          type="checkbox"
          checked={preferences.compactDensity}
          on:change={(event) => updatePreferences({ compactDensity: event.currentTarget.checked })}
        />
        <span>Compact density</span>
      </label>
      <label>
        <input
          type="checkbox"
          checked={preferences.strongFocusRing}
          on:change={(event) => updatePreferences({ strongFocusRing: event.currentTarget.checked })}
        />
        <span>Strong focus rings</span>
      </label>
      <label>
        <input
          type="checkbox"
          checked={preferences.reducedTransparency}
          on:change={(event) => updatePreferences({ reducedTransparency: event.currentTarget.checked })}
        />
        <span>Reduce transparency</span>
      </label>
      <label>
        <input
          type="checkbox"
          checked={preferences.showSearchShortcutHint}
          on:change={(event) => updatePreferences({ showSearchShortcutHint: event.currentTarget.checked })}
        />
        <span>Search shortcut hint</span>
      </label>
    </div>
  </section>

  <section class="settings-section" aria-labelledby="clock-heading">
    <h2 id="clock-heading">Clock</h2>
    <label>
      <span>Date format</span>
      <input
        value={preferences.dateFormat}
        maxlength="64"
        spellcheck="false"
        aria-describedby="date-format-help"
        on:input={handleDateFormatInput}
      />
    </label>
    <p id="date-format-help" class="settings-help">
      Tokens: yyyy, yy, MMMM, MMM, MM, M, dd, d, EEEE, EEE
    </p>
    <div class="settings-chip-row" aria-label="Date format examples">
      {#each dateFormatExamples as format}
        <button type="button" on:click={() => updatePreferences({ dateFormat: format })}>{format}</button>
      {/each}
    </div>
    <div class="settings-toggle-grid two">
      <label>
        <input
          type="checkbox"
          checked={preferences.use24HourTime}
          on:change={(event) => updatePreferences({ use24HourTime: event.currentTarget.checked })}
        />
        <span>24-hour time</span>
      </label>
      <label>
        <input
          type="checkbox"
          checked={preferences.showSeconds}
          on:change={(event) => updatePreferences({ showSeconds: event.currentTarget.checked })}
        />
        <span>Show seconds</span>
      </label>
    </div>
    <div class="settings-preview" aria-label="Clock preview">
      <strong>{timePreview}</strong>
      <span>{datePreview}</span>
    </div>
  </section>

  <footer class="settings-panel-footer">
    <button type="button" on:click={resetPresentation}>Reset</button>
    <button type="button" on:click={closePanel}>Done</button>
  </footer>
</main>
