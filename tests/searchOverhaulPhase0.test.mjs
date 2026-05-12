import assert from 'node:assert/strict';
import { existsSync, readdirSync, readFileSync, statSync } from 'node:fs';
import { test } from 'node:test';
import { shouldApplySystemSearchResponse } from '../dist-tests/lib/systemSearchState.js';

const repoRoot = new URL('..', import.meta.url);
const queryFixture = readJson('tests/fixtures/searchOverhaulQueries.fixture.json');
const performanceFixture = readJson('tests/fixtures/searchOverhaulPerformance.fixture.json');
const legacyFixture = readJson('tests/fixtures/searchOverhaulLegacyRemnants.fixture.json');

function repoPath(relativePath) {
  return new URL(relativePath.replaceAll('\\', '/'), repoRoot);
}

function readJson(relativePath) {
  return JSON.parse(readFileSync(repoPath(relativePath), 'utf8'));
}

function readSource(relativePath) {
  return readFileSync(repoPath(relativePath), 'utf8');
}

function markdownText(source) {
  return source.replaceAll('`', '');
}

function readExistingSources(relativePaths) {
  return relativePaths
    .filter((relativePath) => existsSync(repoPath(relativePath)))
    .map((relativePath) => readSource(relativePath))
    .join('\n');
}

function listFilesRecursive(relativePath, predicate = () => true) {
  const root = repoPath(relativePath);
  if (!existsSync(root)) {
    return [];
  }

  const entries = [];
  for (const entry of readdirSync(root, { withFileTypes: true })) {
    const childPath = `${relativePath.replaceAll('\\', '/')}/${entry.name}`;
    if (entry.isDirectory()) {
      entries.push(...listFilesRecursive(childPath, predicate));
    } else if (predicate(childPath)) {
      entries.push(childPath);
    }
  }
  return entries;
}

function importedSearchSourceModules(source) {
  const modules = new Set();
  for (const match of source.matchAll(/crate::search_sources::([a-zA-Z_][a-zA-Z0-9_]*)/g)) {
    modules.add(match[1]);
  }
  for (const match of source.matchAll(/crate::search_sources::\{([^}]+)\}/g)) {
    for (const part of match[1].split(',')) {
      const moduleName = part.trim().split(/\s+/)[0];
      if (/^[a-zA-Z_][a-zA-Z0-9_]*$/.test(moduleName)) {
        modules.add(moduleName);
      }
    }
  }
  return [...modules].sort();
}

function extractFunction(source, functionName) {
  const start = source.indexOf(`function ${functionName}`);
  assert.notEqual(start, -1, `${functionName} exists`);
  const braceStart = source.indexOf('{', start);
  let depth = 0;
  for (let index = braceStart; index < source.length; index += 1) {
    const char = source[index];
    if (char === '{') {
      depth += 1;
    } else if (char === '}') {
      depth -= 1;
      if (depth === 0) {
        return source.slice(start, index + 1);
      }
    }
  }
  assert.fail(`${functionName} body closes`);
}

test('phase 0 query fixture covers requested baseline problem queries', () => {
  const queries = queryFixture.queries.map((entry) => entry.query);
  assert.deepEqual(queries, [
    'display settings',
    'sound settings',
    'jnev1',
    'spotify',
    'control panel',
    'windows settings',
    'C:\\dev',
    'C://dev',
    'c dev',
    'dev'
  ]);

  for (const entry of queryFixture.queries) {
    assert.equal(typeof entry.expectedTop.kind, 'string', `${entry.query} has expected kind`);
    assert.ok(entry.expectedTop.title || entry.expectedTop.titleContains, `${entry.query} has title expectation`);
  }
});

test('phase 0 performance fixture documents forbidden input-handler dependencies', () => {
  assert.equal(performanceFixture.inputEchoBudget.includes('Input value'), true);
  assert.equal(performanceFixture.firstVisiblePayloadBudgetMs, 50);
  assert.equal(performanceFixture.rendererSynchronousBudgetMs, 8);
  assert.equal(performanceFixture.forbiddenInputHandlerDependencies.length >= 5, true);
  assert.deepEqual(performanceFixture.requiredOrdering, [
    'input assignment',
    'pending or current-best payload publish',
    'deferred provider dispatch',
    'latest-only response application'
  ]);
});

test('phase 0 legacy-remnant checklist covers old search interference files', () => {
  const paths = legacyFixture.remnants.map((entry) => entry.path);
  assert.equal(paths.includes('src/lib/searchCatalog.ts'), true);
  assert.equal(paths.includes('src/lib/searchRanking.ts'), true);
  assert.equal(paths.includes('src-tauri/src/search_sources/index.rs'), true);
  assert.equal(paths.includes('src-tauri/src/search_sources/windows_search.rs'), true);
  assert.equal(
    legacyFixture.remnants.every((entry) => entry.mustNotBeImportedByHotPath),
    true
  );
});

test('phase 7 audit expectations are explicit and validated', { todo: 'Historical search_overhaul.md audit was superseded by search_upgrade_plan.md and removed from the active repo' });
/*
test('phase 7 audit expectations are explicit and validated', () => {
  const plan = readSource('search_overhaul.md');
  const statusLine = plan.split('\n').find((line) => line.startsWith('Status:')) ?? '';

  assert.match(statusLine, /phases 0, 1, 2, 3, 4, 5, 6, 7, and 8 implemented and validated/i);
  assert.match(plan, /### Phase 7: Remove Legacy Remnants/);
  assert.match(plan, /### Phase 8: QA, Live Smoke, and Performance Budget/);
  assert.match(plan, /Legacy-remnant audit/);
  assert.match(plan, /AC16\.[\s\S]*old warmed-cache display fallback/);
  assert.match(plan, /AC17\.[\s\S]*diagnostic-only/);
  assert.match(plan, /AC18\.[\s\S]*narrow infrastructure helper/);

  assert.deepEqual(legacyFixture.phase7AuditExpectations.bannedRustSearchSourceModules, [
    'everything',
    'index',
    'provider',
    'windows_search',
    'files'
  ]);
  assert.ok(
    legacyFixture.phase7AuditExpectations.requiredValidationEvidence.some((entry) =>
      entry.includes('rg checks')
    )
  );
});
*/

test('phase 8 QA and performance expectations are testable from the overhaul plan', { todo: 'Historical search_overhaul.md QA checks were superseded by active upgrade-plan validation' });
/*
test('phase 8 QA and performance expectations are testable from the overhaul plan', () => {
  const plan = markdownText(readSource('search_overhaul.md'));

  for (const deliverable of legacyFixture.phase8AuditExpectations.requiredDeliverables) {
    assert.match(plan, new RegExp(deliverable.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')));
  }
  for (const query of legacyFixture.phase8AuditExpectations.liveSmokeQueries) {
    assert.match(plan, new RegExp(query.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')));
  }

  assert.equal(legacyFixture.phase8AuditExpectations.performanceBudgetsMs.firstVisiblePayload, 50);
  assert.equal(legacyFixture.phase8AuditExpectations.performanceBudgetsMs.warmLocalIntent, 10);
  assert.equal(legacyFixture.phase8AuditExpectations.performanceBudgetsMs.everythingTypical, 150);
  assert.equal(legacyFixture.phase8AuditExpectations.performanceBudgetsMs.rendererSyncWork, 8);
});
*/

test('phase 1 contracts exist in new search modules', () => {
  const contractSources = readExistingSources([
    'src/lib/searchEngine.ts',
    'src/features/search/searchEngine.ts',
    'src-tauri/src/search/contracts.rs',
    'src-tauri/src/search_engine/contracts.rs',
    'src-tauri/src/search_engine.rs',
    'src-tauri/src/search.rs'
  ]);

  assert.notEqual(contractSources, '', 'new search contract source exists');
  for (const contractName of [
    'SearchQueryRequest',
    'SearchEngineResponse',
    'SearchProgressPayload',
    'SearchResult',
    'SearchProviderTiming'
  ]) {
    assert.match(contractSources, new RegExp(`\\b${contractName}\\b`), `${contractName} is defined`);
  }
});

test('phase 2 settings provider dataset backs required Windows settings intents', () => {
  const settingsSources = readExistingSources([
    'src-tauri/src/search/providers/settings.rs',
    'src-tauri/src/search_engine/providers/settings.rs',
    'src-tauri/src/search/settings.rs',
    'src/lib/searchSettingsProvider.ts',
    'src/features/search/settingsProvider.ts',
    'src/search/settings.json'
  ]);

  assert.notEqual(settingsSources, '', 'settings provider source exists');
  for (const required of [
    'Display Settings',
    'ms-settings:display',
    'Sound Settings',
    'ms-settings:sound',
    'Windows Settings',
    'ms-settings:',
    'Control Panel',
    'control.exe'
  ]) {
    assert.match(settingsSources, new RegExp(required.replaceAll(':', '\\:')), `${required} present`);
  }
});

test('top-bar input handler keeps forbidden work out of the direct input path', () => {
  const topBar = readSource('src/components/TopBar.svelte');
  const inputHandler = extractFunction(topBar, 'handleSearchInput');
  assert.match(inputHandler, /const nextQuery = \(event\.currentTarget as HTMLInputElement\)\.value;/);
  assert.match(inputHandler, /const request = publishImmediateSearchInputState\(nextQuery\)/);
  assert.match(inputHandler, /startImmediateSearchQueryExecution\(request\)/);

  for (const group of performanceFixture.forbiddenInputHandlerDependencies) {
    for (const pattern of group.patterns) {
      assert.doesNotMatch(inputHandler, new RegExp(pattern), `${group.name}: ${pattern}`);
    }
  }
});

test('rapid typing publishes pending or current-best payload before provider resolution and gates stale responses', () => {
  const topBar = readSource('src/components/TopBar.svelte');
  const publishImmediate = extractFunction(topBar, 'publishImmediateSearchInputState');
  const startImmediate = extractFunction(topBar, 'startImmediateSearchQueryExecution');
  const firstProviderDispatch = Math.min(
    ...[
      startImmediate.indexOf('void loadSearchEngineResults(request)'),
      startImmediate.indexOf('scheduleSearchEngine(searchQuery)')
    ].filter((index) => index >= 0)
  );
  assert.notEqual(firstProviderDispatch, Infinity, 'provider dispatch seam is present');
  assert.match(publishImmediate, /queueSearchPanelPublish\(\{/);
  assert.match(startImmediate, /publishPendingSearchPayload\(request\.sequence, searchResults\)/);
  assert.equal(shouldApplySystemSearchResponse('dis', 3, 'display', 4), false);
  assert.equal(shouldApplySystemSearchResponse('display', 4, 'display', 4), true);
});

test('phase 6 removes TypeScript hot path legacy imports recorded by phase 0', () => {
  const topBar = readSource('src/components/TopBar.svelte');
  const legacyImports = [
    "../lib/searchCatalog",
    "../lib/searchRanking",
    "../lib/systemSearch"
  ];
  for (const importPath of legacyImports) {
    assert.doesNotMatch(topBar, new RegExp(`from ['"]${importPath.replaceAll('/', '\\/')}['"]`));
  }
  assert.match(topBar, /from ['"]\.\.\/lib\/searchEngine['"]/);
});

test('top-bar import block has no legacy catalog ranking or legacy system wrapper import', () => {
  const topBar = readSource('src/components/TopBar.svelte');
  const importEnd = topBar.indexOf('  let now = new Date();');
  assert.notEqual(importEnd, -1, 'TopBar import block end is found');
  const importBlock = topBar.slice(0, importEnd);

  assert.doesNotMatch(importBlock, /from ['"][^'"]*searchCatalog['"]/);
  assert.doesNotMatch(importBlock, /from ['"][^'"]*searchRanking['"]/);
  assert.doesNotMatch(importBlock, /from ['"][^'"]*systemSearch['"]/);
  assert.match(importBlock, /from ['"]\.\.\/lib\/searchEngine['"]/);
  assert.match(importBlock, /from ['"]\.\.\/lib\/systemSearchState['"]/);
});

test('phase 1 new frontend wrapper exposes a registered command name', () => {
  const main = readSource('src-tauri/src/main.rs');
  const wrapper = readSource('src/lib/searchEngine.ts');

  assert.match(wrapper, /SEARCH_ENGINE_COMMAND = IPC_COMMANDS\.searchEngine/);
  assert.match(readSource('src/ipc/commands.ts'), /searchEngine:\s*['"]search_engine['"]/);
  assert.match(main, /search::search_engine/);
});

test('future Rust search hot path does not route through legacy search_sources coordinator or fallbacks', () => {
  const newSearchSources = readExistingSources([
    'src-tauri/src/search.rs',
    'src-tauri/src/search_engine.rs',
    'src-tauri/src/search/mod.rs',
    'src-tauri/src/search_engine/mod.rs'
  ]);
  assert.notEqual(newSearchSources, '', 'new Rust search entry point exists');
  for (const legacy of ['search_sources::index', 'windows_search', 'warmed', 'search-index-v1']) {
    assert.doesNotMatch(newSearchSources, new RegExp(legacy));
  }
});

test('new Rust search subtree does not import old search_sources index provider windows_search or files modules', () => {
  const rustFiles = listFilesRecursive('src-tauri/src/search', (relativePath) =>
    relativePath.endsWith('.rs')
  );
  assert.notEqual(rustFiles.length, 0, 'new Rust search subtree exists');

  const bannedModules = new Set(legacyFixture.phase7AuditExpectations.bannedRustSearchSourceModules);
  const violations = [];

  for (const relativePath of rustFiles) {
    assert.equal(statSync(repoPath(relativePath)).isFile(), true, `${relativePath} is a file`);
    for (const moduleName of importedSearchSourceModules(readSource(relativePath))) {
      if (bannedModules.has(moduleName)) {
        violations.push(`${relativePath} imports crate::search_sources::${moduleName}`);
      }
    }
  }

  assert.deepEqual(violations, []);
});

test('legacy search_system command is no longer registered in production command maps', () => {
  const main = readSource('src-tauri/src/main.rs');
  const commands = readSource('src/ipc/commands.ts');
  const contracts = readSource('src-tauri/src/contracts.rs');
  const legacyWrapper = readSource('src/lib/systemSearch.ts');

  assert.doesNotMatch(main, /search_sources::search_system/);
  assert.doesNotMatch(main, /search_sources::warm_search_index/);
  assert.doesNotMatch(commands, /searchSystem:\s*['"]search_system['"]/);
  assert.doesNotMatch(contracts, /SEARCH_SYSTEM:\s*&str\s*=\s*["']search_system["']/);
  assert.match(legacyWrapper, /deprecated; visible search must call searchEngine/);
});

test('phase 3 everything provider has cached health, bounded simple-name request, and timings', { todo: 'Historical literal check predated Phase 5 query modes; covered by Rust Everything provider tests' });
/*
test('phase 3 everything provider has cached health, bounded simple-name request, and timings', () => {
  const source = readSource('src-tauri/src/search/providers/everything.rs');

  assert.match(source, /EVERYTHING_HEALTH_TTL/);
  assert.match(source, /OnceLock<Mutex<Option<CachedEverythingState>>>/);
  assert.match(source, /full_path_search:\s*false/);
  assert.match(source, /content_search_enabled:\s*false/);
  assert.match(source, /SearchProviderTiming/);
  assert.match(source, /duration_ms/);
});
*/

test('phase 4 app and local providers use cached bounded indexes instead of per-query start menu scans', () => {
  const apps = readSource('src-tauri/src/search/providers/apps.rs');
  const local = readSource('src-tauri/src/search/providers/local.rs');
  const coordinator = readSource('src-tauri/src/search/mod.rs');
  const searchAppsStart = apps.indexOf('pub(crate) fn search_apps');
  const searchAppsEnd = apps.indexOf('pub(crate) fn warm_app_index_async', searchAppsStart);
  assert.notEqual(searchAppsStart, -1, 'search_apps exists');
  assert.notEqual(searchAppsEnd, -1, 'search_apps ends before warm_app_index_async');
  const searchAppsBody = apps.slice(searchAppsStart, searchAppsEnd);

  assert.match(apps, /APP_INDEX_TTL/);
  assert.match(apps, /APP_INDEX_CACHE/);
  assert.match(apps, /warm_app_index_async/);
  assert.match(apps, /currentUserStartMenu/);
  assert.match(apps, /allUsersStartMenu/);
  assert.match(apps, /pinnedTaskbar/);
  assert.doesNotMatch(searchAppsBody, /build_app_index/);
  assert.doesNotMatch(searchAppsBody, /read_sorted_dir/);
  assert.match(local, /C:\\\\dev/);
  assert.match(local, /Downloads/);
  assert.match(local, /Documents/);
  assert.match(coordinator, /search_apps\(&query, limit\)/);
  assert.match(coordinator, /search_local\(&query, limit, &request\.context\)/);
  assert.match(coordinator, /search_everything\(&query, limit\)/);
});
