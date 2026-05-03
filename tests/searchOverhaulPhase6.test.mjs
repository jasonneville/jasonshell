import assert from 'node:assert/strict';
import { existsSync, readFileSync } from 'node:fs';
import { test } from 'node:test';

const repoRoot = new URL('..', import.meta.url);
const fixture = readJson('tests/fixtures/searchOverhaulPhase6.fixture.json');

function repoPath(relativePath) {
  return new URL(relativePath.replaceAll('\\', '/'), repoRoot);
}

function readJson(relativePath) {
  return JSON.parse(readFileSync(repoPath(relativePath), 'utf8'));
}

function readSource(relativePath) {
  return readFileSync(repoPath(relativePath), 'utf8');
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

function firstIndexOfAny(source, patterns) {
  const indexes = patterns
    .map((pattern) => source.search(new RegExp(pattern)))
    .filter((index) => index >= 0);
  return indexes.length === 0 ? -1 : Math.min(...indexes);
}

test('phase 6 fixture defines input-pending-latest-stale pipeline and required seams', () => {
  assert.deepEqual(fixture.pipeline, [
    'input echo',
    'pending payload publish',
    'deferred provider request',
    'latest response apply',
    'stale response ignore'
  ]);
  assert.equal(fixture.pendingPayload.phase, 'typing');
  assert.equal(fixture.latestResponse.sequence > fixture.staleResponse.sequence, true);
  assert.equal(fixture.requiredCodeSeams.includes('createLatestSearchQueryController'), true);
  assert.equal(fixture.requiredCodeSeams.includes('shouldApplySearchEngineResponse'), true);
});

test('phase 6 input handler updates query before any provider ranking storage filesystem or native show work', () => {
  const topBar = readSource('src/components/TopBar.svelte');
  const inputHandler = extractFunction(topBar, 'handleSearchInput');
  const inputEchoIndex = inputHandler.indexOf('publishImmediateSearchInputState(nextQuery)');
  assert.notEqual(inputEchoIndex, -1, 'input value is forwarded into immediate visible state first');
  assert.notEqual(inputHandler.indexOf('startImmediateSearchQueryExecution(request)'), -1, 'search work starts after visible state update');
  assert.notEqual(
    inputHandler.indexOf('const nextQuery = (event.currentTarget as HTMLInputElement).value;'),
    -1,
    'input event value is captured before queued work'
  );

  for (const group of fixture.forbiddenBeforeInputEcho) {
    const forbiddenIndex = firstIndexOfAny(inputHandler, group.patterns);
    assert.equal(
      forbiddenIndex === -1 || forbiddenIndex > inputEchoIndex,
      true,
      `${group.name} must not run before input echo`
    );
  }
});

test('phase 6 top-bar publishes pending payload before deferred engine request and applies latest response only', () => {
  const topBar = readSource('src/components/TopBar.svelte');
  const publishImmediate = extractFunction(topBar, 'publishImmediateSearchInputState');
  const startImmediate = extractFunction(topBar, 'startImmediateSearchQueryExecution');

  assert.match(publishImmediate, /queueSearchPanelPublish\(\{/);
  assert.match(startImmediate, /publishPendingSearchPayload\(request\.sequence, searchResults\)/);
  assert.ok(startImmediate.indexOf('publishPendingSearchPayload') < startImmediate.indexOf('void loadSearchEngineResults(request)'), 'pending payload is queued before provider dispatch');
  assert.equal(/phase:\s*['"]typing['"]/.test(topBar), true, 'typing pending phase is emitted');
  assert.equal(/searchEngine\(/.test(topBar), true, 'new search engine wrapper is called');
  assert.equal(/shouldApplySearchEngineResponse\(/.test(topBar), true, 'latest-response gate is used');
  assert.equal(/searchEngineResponseToPanelPayload\(/.test(topBar), true, 'engine response maps to panel payload');
  assert.equal(
    /if \(!shouldApplySearchEngineResponse\([\s\S]*?\)\) \{[\s\S]*?return;/.test(topBar),
    true,
    'stale engine response returns before applying results'
  );
  assert.equal(/searchEngineInFlight/.test(topBar), false, 'stale provider work is not serialized ahead of latest queries');
  assert.equal(/queuedSearchEngineRequest/.test(topBar), false, 'latest query dispatch is not delayed behind stale in-flight work');
});

test('phase 6 activation preserves window focus and control panel action contracts', () => {
  const topBar = readSource('src/components/TopBar.svelte');
  const activateResult = extractFunction(topBar, 'activateResult');

  assert.match(activateResult, /result\.actionId === 'runControlPanel'[\s\S]*runControlPanel\(result\.actionArgs\)/);
  assert.match(activateResult, /result\.kind === 'window'[\s\S]*openWindows\.find\(\(item\) => result\.id === `window:\$\{item\.hwnd\}`\)/);
  assert.match(activateResult, /activateTaskWindow\(taskWindow\.hwnd, taskWindow\.isActive\)/);
});

test('phase 6 top-bar hot path imports new search engine and no legacy catalog ranking or system-search wrappers', () => {
  const topBar = readSource('src/components/TopBar.svelte');

  for (const importPath of fixture.requiredTopBarImports) {
    assert.equal(
      new RegExp(`from ['"]${importPath.replaceAll('/', '\\/')}['"]`).test(topBar),
      true,
      `${importPath} imported`
    );
  }
  for (const importPath of fixture.legacyHotPathImports) {
    assert.equal(
      new RegExp(`from ['"]${importPath.replaceAll('/', '\\/')}['"]`).test(topBar),
      false,
      `${importPath} absent from TopBar hot path`
    );
  }
});

test('phase 6 hot path does not use legacy source files or browser storage for visible result production', () => {
  const hotPathSources = [
    'src/components/TopBar.svelte',
    'src/lib/searchEngine.ts',
    'src/features/search/searchQueryController.ts'
  ]
    .filter((relativePath) => existsSync(repoPath(relativePath)))
    .map(readSource)
    .join('\n');

  assert.notEqual(hotPathSources, '', 'search hot path sources exist');
  assert.equal(/from ['"].*searchCatalog['"]/.test(hotPathSources), false, 'searchCatalog import absent');
  assert.equal(/from ['"].*searchRanking['"]/.test(hotPathSources), false, 'searchRanking import absent');
  assert.equal(/from ['"].*systemSearch['"]/.test(hotPathSources), false, 'systemSearch import absent');
  assert.equal(/localStorage/.test(hotPathSources), false, 'localStorage absent from hot path');
  assert.equal(
    /search_sources|windows_search|warmedCache|search-index-v1/.test(hotPathSources),
    false,
    'legacy Rust/cache tokens absent from hot path'
  );
});
