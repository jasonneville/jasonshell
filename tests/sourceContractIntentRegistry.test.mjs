import assert from 'node:assert/strict';
import { existsSync, readdirSync, readFileSync } from 'node:fs';
import path from 'node:path';
import test from 'node:test';

const SOURCE_CONTRACT_INTENTS = {
  architectureContract: [
    'audioControls.test.mjs',
    'centeredSearchSurface.test.mjs',
    'commandPanelTheme.test.mjs',
    'commandPanelWiring.test.mjs',
    'contextMenuPosition.test.mjs',
    'controlPlaneRouting.test.mjs',
    'controlPlaneState.test.mjs',
    'developerProviders.test.mjs',
    'devTools.test.mjs',
    'frontendUiPolicy.test.mjs',
    'meltMigrationWiring.test.mjs',
    'overlayDismissalWiring.test.mjs',
    'persistentSurfaceLifecycle.test.mjs',
    'processManagerWiring.test.mjs',
    'quickCommands.test.mjs',
    'quickIcons.test.mjs',
    'quickIconsPhase3.test.mjs',
    'quickLaunchReliability.test.mjs',
    'searchAppCacheRefresh.test.mjs',
    'searchClearButtons.test.mjs',
    'searchCloseReset.test.mjs',
    'searchOverhaulPhase0.test.mjs',
    'searchOverhaulPhase6.test.mjs',
    'searchPanelState.test.mjs',
    'searchTypingFreezePhase1.test.mjs',
    'settingsPanelWiring.test.mjs',
    'shellBarResize.test.mjs',
    'shellOpenCloseEvents.test.mjs',
    'shellPopupLayoutScrollPhase1.test.mjs',
    'shellPreferences.test.mjs',
    'stackBrowserMarginSelection.test.mjs',
    'stackBrowserGitStatus.test.mjs',
    'stackBrowserTerminal.test.mjs',
    'stackBrowserPathAutocomplete.test.mjs',
    'stackBrowserTopBarPinFlow.test.mjs',
    'stackPopupContextMenu.test.mjs',
    'stackPopupDiagnosticsWiring.test.mjs',
    'stackPopupGitStatus.test.mjs',
    'stackPopupMarqueeWiring.test.mjs',
    'stackPopupNewTextFileFlow.test.mjs',
    'stackPopupPagingPhase1Wiring.test.mjs',
    'stackPopupPagingPhase2Wiring.test.mjs',
    'stackPopupPagingPhase3Wiring.test.mjs',
    'stackPopupPagingPhase4Wiring.test.mjs',
    'stackPopupPagingPhase5Wiring.test.mjs',
    'stackPopupPagingPhase6Responsiveness.test.mjs',
    'surfaceCodeSplitting.test.mjs',
    'taskbarPreviewContract.test.mjs',
    'taskbarLauncherReliability.test.mjs',
    'taskbarLauncherReorder.test.mjs',
    'taskbarUxState.test.mjs',
    'taskPreviewRetention.test.mjs',
    'taskPreviewTextPolish.test.mjs',
    'themeRegistry.test.mjs',
    'topBarCalendar.test.mjs',
    'topBarFolderReorder.test.mjs',
    'topBarPins.test.mjs',
    'topBarTimeoutHygiene.test.mjs',
    'trayPanelRetention.test.mjs',
    'trayPanelWiring.test.mjs',
    'vscodeFolderOpenPhase4.test.mjs',
    'windowsKeyOverride.test.mjs',
    'workspaces.test.mjs'
  ],
  registryParity: [
    'changelogPolicy.test.mjs',
    'changelogPolicyHygiene.test.mjs',
    'contractsSettings.test.mjs',
    'distTestsHygiene.test.mjs',
    'multiFixPhase6SpecValidation.test.mjs'
  ],
  securityBoundary: [
    'automationProviders.test.mjs',
    'backendBlockingLockBoundaries.test.mjs',
    'backendBlockingLockBoundariesP6.test.mjs',
    'searchEngineContracts.test.mjs',
    'settingsPowerActionsPhase5.test.mjs'
  ],
  temporaryLegacyGuard: [
    'audioAutoRefresh.test.mjs',
    'masterSpecSearchHygiene.test.mjs'
  ]
};

const VALID_INTENTS = new Set(Object.keys(SOURCE_CONTRACT_INTENTS));

function registeredIntentByFile() {
  const byFile = new Map();
  for (const [intent, files] of Object.entries(SOURCE_CONTRACT_INTENTS)) {
    assert.ok(VALID_INTENTS.has(intent), `unknown source-contract intent ${intent}`);
    for (const file of files) {
      assert.equal(byFile.has(file), false, `${file} has duplicate source-contract intent`);
      byFile.set(file, intent);
    }
  }
  return byFile;
}

function sourceLikeTestFiles() {
  return readdirSync(new URL('.', import.meta.url))
    .filter((name) => name.endsWith('.test.mjs'))
    .filter((name) => {
      const source = readFileSync(new URL(name, import.meta.url), 'utf8');
      return /readFileSync\(new URL\('\.\.\//.test(source) || /readFileSync\('[^']*src/.test(source);
    })
    .sort();
}

test('source-text tests have explicit intent tags in the registry', () => {
  const byFile = registeredIntentByFile();
  const missing = sourceLikeTestFiles().filter((file) => file !== path.basename(import.meta.url) && !byFile.has(file));
  assert.deepEqual(missing, []);
});

test('source-contract registry keeps boundary tests separate from behavior legacy guards', () => {
  const byFile = registeredIntentByFile();
  assert.equal(byFile.get('contractsSettings.test.mjs'), 'registryParity');
  assert.equal(byFile.get('searchEngineContracts.test.mjs'), 'securityBoundary');
  assert.equal(byFile.get('masterSpecSearchHygiene.test.mjs'), 'temporaryLegacyGuard');
  assert.equal(byFile.get('topBarTimeoutHygiene.test.mjs'), 'architectureContract');
});

test('source-contract registry entries point at existing tests', () => {
  const missing = Object.values(SOURCE_CONTRACT_INTENTS)
    .flat()
    .filter((file) => !existsSync(new URL(file, import.meta.url)));

  assert.deepEqual(missing, []);
});
