<script lang="ts">
  import './SettingsPanelSurface.css';
  import MeltActionButton from './melt/MeltActionButton.svelte';
  import MeltRadioGroup from './melt/MeltRadioGroup.svelte';
  import MeltSelect from './melt/MeltSelect.svelte';
  import MeltToggle from './melt/MeltToggle.svelte';
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
  const themeSelectOptions = themeOptions.map((theme) => ({ value: theme.id, label: theme.label }));
  const fontSelectOptions = fontOptions.map((font) => ({ value: font.id, label: font.label }));
  const dateFormatExamples = [
    'EEE, MMM d',
    'EEEE, MMMM d',
    'yyyy-MM-dd',
    'MM/dd/yyyy'
  ];
  const dateFormatOptions = dateFormatExamples.map((format) => ({ value: format, label: format }));

  let preferences: ShellPreferences = getInitialShellPreferences();
  let selectedThemeId: ShellThemeId = getInitialShellThemeId();
  let now = new Date();

  $: datePreview = formatShellDate(now, preferences.dateFormat);
  $: timePreview = formatShellTime(now, preferences);

  function updatePreferences(patch: Partial<ShellPreferences>) {
    preferences = patchShellPreferences(patch);
  }

  function handleThemeChange(value: string) {
    selectedThemeId = normalizeShellThemeId(value);
    setShellTheme(selectedThemeId);
  }

  function handleFontChange(value: string) {
    updatePreferences({ fontId: value as ShellPreferences['fontId'] });
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
    <MeltActionButton ariaLabel="Close settings" onClick={closePanel}>x</MeltActionButton>
  </header>

  <section class="settings-section" aria-labelledby="appearance-heading">
    <h2 id="appearance-heading">Appearance</h2>
    <MeltSelect
      label="Theme"
      value={selectedThemeId}
      options={themeSelectOptions}
      onChange={handleThemeChange}
    />

    <MeltSelect
      label="Font"
      value={preferences.fontId}
      options={fontSelectOptions}
      onChange={handleFontChange}
    />

    <div class="settings-toggle-grid">
      <MeltToggle
        checked={preferences.compactDensity}
        label="Compact density"
        onChange={(compactDensity) => updatePreferences({ compactDensity })}
      />
      <MeltToggle
        checked={preferences.strongFocusRing}
        label="Strong focus rings"
        onChange={(strongFocusRing) => updatePreferences({ strongFocusRing })}
      />
      <MeltToggle
        checked={preferences.reducedTransparency}
        label="Reduce transparency"
        onChange={(reducedTransparency) => updatePreferences({ reducedTransparency })}
      />
      <MeltToggle
        checked={preferences.showSearchShortcutHint}
        label="Search shortcut hint"
        onChange={(showSearchShortcutHint) => updatePreferences({ showSearchShortcutHint })}
      />
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
    <MeltRadioGroup
      class="date-format-presets"
      label="Date format examples"
      value={preferences.dateFormat}
      options={dateFormatOptions}
      onChange={(dateFormat) => updatePreferences({ dateFormat })}
    />
    <div class="settings-toggle-grid two">
      <MeltToggle
        checked={preferences.use24HourTime}
        label="24-hour time"
        onChange={(use24HourTime) => updatePreferences({ use24HourTime })}
      />
      <MeltToggle
        checked={preferences.showSeconds}
        label="Show seconds"
        onChange={(showSeconds) => updatePreferences({ showSeconds })}
      />
    </div>
    <div class="settings-preview" aria-label="Clock preview">
      <strong>{timePreview}</strong>
      <span>{datePreview}</span>
    </div>
  </section>

  <footer class="settings-panel-footer">
    <MeltActionButton onClick={resetPresentation}>Reset</MeltActionButton>
    <MeltActionButton onClick={closePanel}>Done</MeltActionButton>
  </footer>
</main>
