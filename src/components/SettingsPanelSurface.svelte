<script lang="ts">
  // @ts-ignore: CSS side-effect import handled by bundler
  import './SettingsPanelSurface.css';
  import { onMount } from 'svelte';
  import MeltActionButton from './melt/MeltActionButton.svelte';
  import MeltRadioGroup from './melt/MeltRadioGroup.svelte';
  import MeltSelect from './melt/MeltSelect.svelte';
  import MeltToggle from './melt/MeltToggle.svelte';
  import {
    formatShellDate,
    formatShellTime,
    getInitialShellPreferences,
    installGoogleFontPreference,
    patchShellPreferences,
    setShellPreferences,
    shellFontOptions,
    type ShellPreferences
  } from '../lib/shellPreferences';
  import {
    hideSettingsPanel,
    triggerSystemPowerAction,
    type SystemPowerAction
  } from '../lib/settingsPanel';
  import {
    normalizeStackTerminalProfile,
    STACK_TERMINAL_PROFILE_OPTIONS,
    type StackTerminalProfile
  } from '../lib/stackPopup';
  import {
    defaultShellSettings,
    loadShellSettings,
    saveShellBarLock,
    saveShellSettings,
    type ShellSettings
  } from '../lib/settings';
  import {
    getInitialShellThemeId,
    normalizeShellThemeId,
    setShellTheme,
    shellThemeOptions,
    type ShellThemeId
  } from '../lib/themes';

  const themeOptions = shellThemeOptions();
  const themeSelectOptions = themeOptions.map((theme) => ({ value: theme.id, label: theme.label }));
  const stackTerminalProfileOptions = STACK_TERMINAL_PROFILE_OPTIONS.map((option) => ({
    value: option.value,
    label: option.label
  }));
  const dateFormatExamples = [
    'EEE, MMM d',
    'EEEE, MMMM d',
    'yyyy-MM-dd',
    'MM/dd/yyyy'
  ];
  const dateFormatOptions = dateFormatExamples.map((format) => ({ value: format, label: format }));
  let preferences: ShellPreferences = getInitialShellPreferences();
  let shellSettings: ShellSettings = defaultShellSettings();
  let selectedThemeId: ShellThemeId = getInitialShellThemeId();
  let now = new Date();
  let pendingPowerAction: SystemPowerAction | null = null;
  let powerError = '';
  let powerBusy = false;
  let selectedStackTerminalProfile: StackTerminalProfile = 'windowsTerminal';
  let settingsError = '';
  let googleFontLink = '';
  let googleFontStatus = '';
  let googleFontError = '';

  const powerActionLabels: Record<SystemPowerAction, string> = {
    sleep: 'Sleep',
    restart: 'Restart',
    shutdown: 'Turn Off'
  };

  $: fontSelectOptions = shellFontOptions(preferences.customFonts).map((font) => ({ value: font.id, label: font.label }));
  $: datePreview = formatShellDate(now, preferences.dateFormat);
  $: timePreview = formatShellTime(now, preferences);

  onMount(() => {
    void loadJsonShellSettings();
  });
  loadShellSettings()
    .then((settings) => {
      shellSettings = settings;
    })
    .catch((error) => {
      console.error('Failed to load shell settings', error);
    });

  function updatePreferences(patch: Partial<ShellPreferences>) {
    preferences = patchShellPreferences(patch);
  }

  function updateShellBarLock(edge: 'top' | 'bottom', locked: boolean) {
    shellSettings = {
      ...shellSettings,
      ui: {
        ...shellSettings.ui,
        ...(edge === 'top' ? { lockTopBarHeight: locked } : { lockBottomBarHeight: locked })
      }
    };
    void saveShellBarLock(edge, locked)
      .then((settings) => {
        shellSettings = settings;
      })
      .catch((error) => {
        console.error('Failed to save shell bar lock setting', error);
      });
  }

  function handleThemeChange(value: string) {
    selectedThemeId = normalizeShellThemeId(value);
    setShellTheme(selectedThemeId);
  }

  function handleFontChange(value: string) {
    updatePreferences({ fontId: value as ShellPreferences['fontId'] });
    googleFontStatus = '';
    googleFontError = '';
  }

  function handleGoogleFontLinkInput(event: Event) {
    const target = event.currentTarget instanceof HTMLInputElement ? event.currentTarget : null;
    googleFontLink = target?.value ?? '';
    googleFontStatus = '';
    googleFontError = '';
  }

  function installGoogleFont() {
    try {
      preferences = installGoogleFontPreference(googleFontLink, preferences);
      const selected = shellFontOptions(preferences.customFonts).find((font) => font.id === preferences.fontId);
      googleFontStatus = selected ? `Installed and applied ${selected.label}.` : 'Installed and applied font.';
      googleFontError = '';
      googleFontLink = '';
    } catch (error) {
      googleFontStatus = '';
      googleFontError = error instanceof Error ? error.message : 'Paste a valid https://fonts.google.com font link.';
    }
  }

  async function loadJsonShellSettings() {
    try {
      const settings = await loadShellSettings();
      shellSettings = settings;
      selectedStackTerminalProfile = normalizeStackTerminalProfile(settings.stackBrowser?.terminalProfile);
      settingsError = '';
    } catch (error) {
      console.error('Failed to load shell settings', error);
      selectedStackTerminalProfile = 'windowsTerminal';
      settingsError = error instanceof Error ? error.message : 'Shell settings unavailable.';
    }
  }

  async function handleStackTerminalProfileChange(value: string) {
    selectedStackTerminalProfile = normalizeStackTerminalProfile(value);
    try {
      const settings = shellSettings ?? await loadShellSettings();
      shellSettings = await saveShellSettings({
        ...settings,
        stackBrowser: {
          ...(settings.stackBrowser ?? { terminalProfile: 'windowsTerminal' }),
          terminalProfile: selectedStackTerminalProfile
        }
      });
      settingsError = '';
    } catch (error) {
      console.error('Failed to save Stack Browser terminal profile', error);
      settingsError = error instanceof Error ? error.message : 'Terminal profile unavailable.';
    }
  }

  function handleDateFormatInput(event: Event) {
    const target = event.currentTarget instanceof HTMLInputElement ? event.currentTarget : null;
    updatePreferences({ dateFormat: target?.value ?? '' });
  }

  function resetPresentation() {
    preferences = setShellPreferences({
      fontId: 'open-sans',
      customFonts: preferences.customFonts,
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

  function requestPowerAction(action: SystemPowerAction) {
    pendingPowerAction = action;
    powerError = '';
  }

  function cancelPowerAction() {
    if (powerBusy) {
      return;
    }
    pendingPowerAction = null;
    powerError = '';
  }

  async function confirmPowerAction() {
    if (!pendingPowerAction || powerBusy) {
      return;
    }

    powerBusy = true;
    powerError = '';
    try {
      await triggerSystemPowerAction({ action: pendingPowerAction });
      pendingPowerAction = null;
    } catch (error) {
      powerError = error instanceof Error ? error.message : 'Power action failed.';
    } finally {
      powerBusy = false;
    }
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

    <div class="google-font-installer">
      <label>
        <span>Install Google Font</span>
        <input
          type="text"
          value={googleFontLink}
          placeholder="https://fonts.google.com/specimen/Roboto"
          spellcheck="false"
          aria-describedby="google-font-help google-font-status"
          on:input={handleGoogleFontLinkInput}
        />
      </label>
      <MeltActionButton onClick={installGoogleFont}>Install font</MeltActionButton>
    </div>
    <p id="google-font-help" class="settings-help">
      Only https://fonts.google.com specimen or family links are accepted.
    </p>
    {#if googleFontStatus}
      <p id="google-font-status" class="settings-success" role="status">{googleFontStatus}</p>
    {:else if googleFontError}
      <p id="google-font-status" class="settings-error" role="alert">{googleFontError}</p>
    {/if}

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

  <section class="settings-section" aria-labelledby="json-shell-heading">
    <h2 id="json-shell-heading">JSON shell settings</h2>
    <MeltSelect
      label="Stack Browser terminal"
      value={selectedStackTerminalProfile}
      options={stackTerminalProfileOptions}
      onChange={handleStackTerminalProfileChange}
    />
    {#if settingsError}
      <p class="settings-error" role="alert">{settingsError}</p>
    {/if}
  </section>

  <section class="settings-section" aria-labelledby="shell-bars-heading">
    <h2 id="shell-bars-heading">Shell bars</h2>
    <div class="settings-toggle-grid two">
      <MeltToggle
        checked={shellSettings.ui.lockTopBarHeight}
        label="Lock top bar"
        onChange={(lockTopBarHeight) => updateShellBarLock('top', lockTopBarHeight)}
      />
      <MeltToggle
        checked={shellSettings.ui.lockBottomBarHeight}
        label="Lock bottom bar"
        onChange={(lockBottomBarHeight) => updateShellBarLock('bottom', lockBottomBarHeight)}
      />
    </div>
  </section>

  <section class="settings-section" aria-labelledby="power-heading">
    <h2 id="power-heading">Power</h2>
    <p class="settings-help">System power actions require confirmation before running.</p>
    <div class="settings-power-actions">
      <MeltActionButton onClick={() => requestPowerAction('sleep')}>Sleep</MeltActionButton>
      <MeltActionButton onClick={() => requestPowerAction('restart')}>Restart</MeltActionButton>
      <MeltActionButton onClick={() => requestPowerAction('shutdown')}>Turn Off</MeltActionButton>
    </div>
    {#if pendingPowerAction}
      <div
        class="settings-power-confirm"
        role="alertdialog"
        aria-labelledby="power-confirm-heading"
        aria-describedby="power-confirm-description"
      >
        <h3 id="power-confirm-heading">Confirm {powerActionLabels[pendingPowerAction]}</h3>
        <p id="power-confirm-description">
          This will {powerActionLabels[pendingPowerAction].toLowerCase()} this PC.
        </p>
        {#if powerError}
          <p class="settings-power-error" role="alert">{powerError}</p>
        {/if}
        <div class="settings-power-confirm-actions">
          <MeltActionButton onClick={cancelPowerAction}>Cancel</MeltActionButton>
          <MeltActionButton onClick={confirmPowerAction}>
            {powerBusy ? 'Working...' : `Confirm ${powerActionLabels[pendingPowerAction]}`}
          </MeltActionButton>
        </div>
      </div>
    {/if}
  </section>

  <footer class="settings-panel-footer">
    <MeltActionButton onClick={resetPresentation}>Reset</MeltActionButton>
    <MeltActionButton onClick={closePanel}>Done</MeltActionButton>
  </footer>
</main>
