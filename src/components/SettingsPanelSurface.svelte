<script lang="ts">
  // @ts-ignore: CSS side-effect import handled by bundler
  import './SettingsPanelSurface.css';
  import { onMount } from 'svelte';
  import MeltActionButton from './melt/MeltActionButton.svelte';
  import MaterialSymbolIcon from './icons/MaterialSymbolIcon.svelte';
  import MeltRadioGroup from './melt/MeltRadioGroup.svelte';
  import MeltSelect from './melt/MeltSelect.svelte';
  import MeltToggle from './melt/MeltToggle.svelte';
  import { getSpeechModelStatus, importSpeechModel, importSpeechModelFolder, type SpeechModelStatus } from '../lib/speech';
  import {
    formatShellDate,
    formatShellTime,
    getInitialShellPreferences,
    installGoogleFontPreference,
    patchShellPreferences,
    STACK_EDITOR_FONT_OPTIONS,
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
    defaultStandardHotkeySettings,
    defaultShellSettings,
    loadShellSettings,
    saveShellBarLock,
    saveShellSettings,
    type CanonicalHotkeyBinding,
    type StandardHotkeyAction,
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
  const stackEditorFontSelectOptions = STACK_EDITOR_FONT_OPTIONS.map((font) => ({ value: font.id, label: font.label }));
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
  let hotkeyError = '';
  let hotkeyBusy = false;
  let shellSettingsLoaded = false;
  let googleFontLink = '';
  let googleFontStatus = '';
  let googleFontError = '';
  let speechModel: SpeechModelStatus | null = null;
  let speechModelBusy = false;
  let speechModelError = '';
  let speechModelMounted = false;
  let speechModelRequest = 0;
  let speechModelTimer: ReturnType<typeof setTimeout> | undefined;

  const powerActionLabels: Record<SystemPowerAction, string> = {
    sleep: 'Sleep',
    restart: 'Restart',
    shutdown: 'Turn Off'
  };

  $: fontSelectOptions = shellFontOptions(preferences.customFonts).map((font) => ({ value: font.id, label: font.label }));
  $: datePreview = formatShellDate(now, preferences.dateFormat);
  $: timePreview = formatShellTime(now, preferences);
  $: speechModelLabel = speechModel?.state === 'ready'
    ? `Speech model ready (${speechModel.source === 'installed' ? 'installed' : 'bundled'}).`
    : speechModel?.state === 'loading'
      ? 'Loading speech model…'
      : speechModel?.state === 'missing'
        ? 'Speech model not installed.'
        : speechModel?.state === 'error'
          ? 'Speech model unavailable. Import a compatible archive or folder.'
          : 'Checking speech model…';

  onMount(() => {
    void loadJsonShellSettings();
    speechModelMounted = true;
    void refreshSpeechModel();
    return () => {
      speechModelMounted = false;
      speechModelRequest += 1;
      clearTimeout(speechModelTimer);
    };
  });

  function scheduleSpeechModelRefresh() {
    clearTimeout(speechModelTimer);
    if (speechModelMounted && !speechModelBusy && speechModel?.state === 'loading') {
      speechModelTimer = setTimeout(() => void refreshSpeechModel(), 1000);
    }
  }

  async function refreshSpeechModel(preserveImportError = false) {
    const request = ++speechModelRequest;
    try {
      const model = await getSpeechModelStatus();
      if (!speechModelMounted || request !== speechModelRequest) return;
      speechModel = model;
      if (!preserveImportError) speechModelError = model.error ?? '';
      scheduleSpeechModelRefresh();
    } catch (error) {
      if (!speechModelMounted || request !== speechModelRequest) return;
      speechModel = { state: 'error', source: null, error: null };
      if (!preserveImportError) {
        speechModelError = typeof error === 'string' ? error : error instanceof Error ? error.message : 'Could not check speech model. Try importing a compatible archive or folder.';
      }
    }
  }

  async function handleSpeechModelImport() {
    await runSpeechModelImport(importSpeechModel);
  }

  async function handleSpeechModelFolderImport() {
    await runSpeechModelImport(importSpeechModelFolder);
  }

  async function runSpeechModelImport(importModel: typeof importSpeechModel) {
    if (speechModelBusy) return;
    const request = ++speechModelRequest;
    clearTimeout(speechModelTimer);
    speechModelBusy = true;
    speechModelError = '';
    try {
      const result = await importModel();
      if (!speechModelMounted || request !== speechModelRequest) return;
      speechModel = result.model;
      speechModelError = result.model.error ?? '';
    } catch (error) {
      if (!speechModelMounted || request !== speechModelRequest) return;
      speechModelError = typeof error === 'string' ? error : error instanceof Error ? error.message : 'Import failed. Choose a complete Parakeet v2 int8 archive or folder and try again.';
      if (!speechModel) {
        speechModelBusy = false;
        void refreshSpeechModel(true);
      }
    } finally {
      if (speechModelMounted && request === speechModelRequest) {
        speechModelBusy = false;
        scheduleSpeechModelRefresh();
      }
    }
  }

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

  function handleStackEditorFontChange(value: string) {
    updatePreferences({ stackEditorFontId: value as ShellPreferences['stackEditorFontId'] });
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
      shellSettingsLoaded = true;
      settingsError = '';
    } catch (error) {
      console.error('Failed to load shell settings', error);
      selectedStackTerminalProfile = 'windowsTerminal';
      shellSettingsLoaded = false;
      settingsError = error instanceof Error ? error.message : 'Shell settings unavailable.';
    }
  }

  async function handleStackTerminalProfileChange(value: string) {
    if (!shellSettingsLoaded) return;

    selectedStackTerminalProfile = normalizeStackTerminalProfile(value);
    try {
      shellSettings = await saveShellSettings({
        ...shellSettings,
        stackBrowser: {
          ...(shellSettings.stackBrowser ?? { terminalProfile: 'windowsTerminal' }),
          terminalProfile: selectedStackTerminalProfile
        }
      });
      settingsError = '';
    } catch (error) {
      console.error('Failed to save Stack Browser terminal profile', error);
      settingsError = error instanceof Error ? error.message : 'Terminal profile unavailable.';
    }
  }

  const hotkeyActionNames: Record<StandardHotkeyAction, string> = {
    search: 'Search',
    terminal: 'Terminal',
    stackBrowser: 'Stack Browser',
    speechTranscription: 'Speech transcription',
    snipping: 'Screen snipping'
  };

  $: snippingConflictAction = (Object.keys(hotkeyActionNames) as StandardHotkeyAction[])
    .find((action) => action !== 'snipping' && shellSettings.hotkeys[action] === shellSettings.hotkeys.snipping);

  function canonicalKey(event: KeyboardEvent): string | null {
    if (event.code === 'Space') return 'Space';
    if (event.code === 'Backquote') return 'Backquote';
    if (/^Key[A-Z]$/.test(event.code)) return event.code.slice(3);
    if (/^Digit[0-9]$/.test(event.code)) return event.code.slice(5);
    return null;
  }

  async function saveHotkeys(hotkeys: ShellSettings['hotkeys'], failureMessage: string) {
    if (!shellSettingsLoaded || hotkeyBusy) return;
    hotkeyBusy = true;
    hotkeyError = '';
    try {
      const currentSettings = shellSettings;
      shellSettings = await saveShellSettings({ ...currentSettings, hotkeys: { ...hotkeys } });
    } catch (error) {
      console.error(failureMessage, error);
      hotkeyError = error instanceof Error ? error.message : 'Shortcut could not be saved.';
    } finally {
      hotkeyBusy = false;
    }
  }

  function captureHotkeyBinding(event: KeyboardEvent) {
    event.preventDefault();
    event.stopImmediatePropagation();

    const target = event.currentTarget instanceof HTMLButtonElement ? event.currentTarget : null;
    const action = target?.dataset.hotkeyAction as StandardHotkeyAction | undefined;
    if (!action || !shellSettingsLoaded || hotkeyBusy || event.repeat) return;
    if (['Control', 'Alt', 'Shift', 'Meta'].includes(event.key)) return;

    const key = canonicalKey(event);
    if (event.metaKey) {
      hotkeyError = 'Windows key shortcuts are not supported. Use Ctrl or Alt.';
      return;
    }
    if (event.ctrlKey && event.altKey) {
      hotkeyError = 'Ctrl+Alt shortcuts conflict with AltGr. Use Ctrl or Alt.';
      return;
    }
    if (event.shiftKey || (!event.ctrlKey && !event.altKey)) {
      hotkeyError = 'Invalid shortcut. Use Ctrl or Alt with a letter, number, Space, or Backquote.';
      return;
    }
    if (!key) {
      hotkeyError = 'Invalid shortcut key. Use a letter, number, Space, or Backquote.';
      return;
    }

    const binding: CanonicalHotkeyBinding = `${event.ctrlKey ? 'Ctrl' : 'Alt'}+${key}`;
    const duplicate = (Object.keys(shellSettings.hotkeys) as StandardHotkeyAction[])
      .find((candidate) => candidate !== action && shellSettings.hotkeys[candidate] === binding);
    if (duplicate) {
      hotkeyError = `Duplicate shortcut: ${binding} is already assigned to ${hotkeyActionNames[duplicate]}.`;
      return;
    }

    void saveHotkeys(
      { ...shellSettings.hotkeys, [action]: binding },
      `Failed to save ${hotkeyActionNames[action]} shortcut`
    );
  }

  function resetHotkeys() {
    void saveHotkeys(defaultStandardHotkeySettings(), 'Failed to reset shortcuts');
  }

  function handleDateFormatInput(event: Event) {
    const target = event.currentTarget instanceof HTMLInputElement ? event.currentTarget : null;
    updatePreferences({ dateFormat: target?.value ?? '' });
  }

  function resetPresentation() {
    preferences = setShellPreferences({
      fontId: 'open-sans',
      stackEditorFontId: 'google-sans-code',
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
    <MeltActionButton ariaLabel="Close settings" onClick={closePanel}><MaterialSymbolIcon name="close" /></MeltActionButton>
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

    <MeltSelect
      label="Stack text editor font"
      value={preferences.stackEditorFontId}
      options={stackEditorFontSelectOptions}
      onChange={handleStackEditorFontChange}
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
    <fieldset class="settings-select-guard" disabled={!shellSettingsLoaded}>
      <MeltSelect
        label="Stack Browser terminal"
        value={selectedStackTerminalProfile}
        options={stackTerminalProfileOptions}
        onChange={handleStackTerminalProfileChange}
      />
    </fieldset>
    {#if settingsError}
      <p class="settings-error" role="alert">{settingsError}</p>
    {/if}
  </section>

  <section class="settings-section" aria-labelledby="speech-model-heading">
    <h2 id="speech-model-heading">Speech</h2>
    <p class="settings-help">
       Choose a trusted Parakeet v2 int8 .tar, .tar.gz, .tgz, or folder once. JasonShell copies the model for offline use, including after restart. Archives need no manual extraction.
    </p>
    <div class="speech-model-actions" aria-busy={speechModelBusy}>
      <MeltActionButton disabled={speechModelBusy} onClick={handleSpeechModelImport}>
        {speechModelBusy ? 'Installing speech model…' : 'Import speech model'}
      </MeltActionButton>
      <MeltActionButton disabled={speechModelBusy} onClick={handleSpeechModelFolderImport}>
        Import speech model folder
      </MeltActionButton>
    </div>
    <p class:settings-success={speechModel?.state === 'ready'} class="settings-help" role="status" aria-live="polite">
      {speechModelBusy ? 'Choose an archive or folder; installation and validation may take a minute.' : speechModelLabel}
    </p>
    {#if speechModelError}
      <p class="settings-error" role="alert">{speechModelError}</p>
    {/if}
  </section>

  <section class="settings-section hotkey-settings" aria-labelledby="hotkeys-heading">
    <h2 id="hotkeys-heading">Keyboard shortcuts</h2>
    <p id="hotkey-help" class="settings-help">Focus a shortcut, then press Ctrl or Alt with a letter, number, Space, or Backquote.</p>
    <div class="hotkey-list">
      <div class="hotkey-row">
        <span>Search</span>
        <button type="button" class="hotkey-capture" data-hotkey-action="search" aria-label="Capture Search shortcut" aria-describedby={hotkeyError ? 'hotkey-error' : 'hotkey-help'} disabled={!shellSettingsLoaded || hotkeyBusy} on:keydown={captureHotkeyBinding}>
          <kbd>{shellSettings.hotkeys.search}</kbd>
        </button>
      </div>
      <div class="hotkey-row">
        <span>Terminal</span>
        <button type="button" class="hotkey-capture" data-hotkey-action="terminal" aria-label="Capture Terminal shortcut" aria-describedby={hotkeyError ? 'hotkey-error' : 'hotkey-help'} disabled={!shellSettingsLoaded || hotkeyBusy} on:keydown={captureHotkeyBinding}>
          <kbd>{shellSettings.hotkeys.terminal}</kbd>
        </button>
      </div>
      <div class="hotkey-row">
        <span>Stack Browser</span>
        <button type="button" class="hotkey-capture" data-hotkey-action="stackBrowser" aria-label="Capture Stack Browser shortcut" aria-describedby={hotkeyError ? 'hotkey-error' : 'hotkey-help'} disabled={!shellSettingsLoaded || hotkeyBusy} on:keydown={captureHotkeyBinding}>
          <kbd>{shellSettings.hotkeys.stackBrowser}</kbd>
        </button>
      </div>
      <div class="hotkey-row">
        <span>Speech transcription</span>
        <button type="button" class="hotkey-capture" data-hotkey-action="speechTranscription" aria-label="Capture Speech transcription shortcut" aria-describedby={hotkeyError ? 'hotkey-error' : 'hotkey-help'} disabled={!shellSettingsLoaded || hotkeyBusy} on:keydown={captureHotkeyBinding}>
          <kbd>{shellSettings.hotkeys.speechTranscription}</kbd>
        </button>
      </div>
      <div class="hotkey-row">
        <span>Screen snipping</span>
        <button type="button" class="hotkey-capture" data-hotkey-action="snipping" aria-label="Capture Screen snipping shortcut" aria-describedby={hotkeyError ? 'hotkey-error' : 'hotkey-help'} disabled={!shellSettingsLoaded || hotkeyBusy} on:keydown={captureHotkeyBinding}>
          <kbd>{shellSettings.hotkeys.snipping}</kbd>
        </button>
      </div>
    </div>
    {#if snippingConflictAction}
      <p class="settings-error" role="alert">{shellSettings.hotkeys.snipping} is already assigned to {hotkeyActionNames[snippingConflictAction]}. Change either shortcut to enable Screen snipping.</p>
    {/if}
    {#if hotkeyError}
      <p id="hotkey-error" class="settings-error" role="alert">{hotkeyError}</p>
    {/if}
    <button type="button" class="hotkey-reset" disabled={!shellSettingsLoaded || hotkeyBusy} on:click={resetHotkeys}>Reset shortcuts to defaults</button>
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
