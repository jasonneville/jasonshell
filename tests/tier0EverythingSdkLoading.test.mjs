import assert from 'node:assert/strict';
import test from 'node:test';
import { functionBody, productionPart, readRust } from './support/rustSource.mjs';

// SEC-02: Everything SDK DLL must load only from approved absolute paths, with
// restricted search flags, once.
const ffi = productionPart(readRust('src-tauri/src/search_sources/everything_ffi.rs'));

test('DLL is loaded with explicit restricted search flags, never Library::new', () => {
  assert.ok(!/\bLibrary::new\(/.test(ffi), 'Library::new uses the default (hijackable) search order');
  assert.match(ffi, /load_with_flags/);
  assert.match(ffi, /LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR/);
  assert.match(ffi, /LOAD_LIBRARY_SEARCH_SYSTEM32/);
});

test('library handle is cached (loaded once), not per query', () => {
  assert.match(ffi, /OnceLock|OnceCell|LazyLock|static\s+\w*LIBRARY/i, 'expected a process-wide cached library');
  const query = functionBody(ffi, 'query_everything_sdk');
  assert.ok(query, 'query_everything_sdk body not found');
  assert.ok(!/load_with_flags|Library::/.test(query), 'query path must reuse the cached library, not load it');
});

test('dev-only candidates (env var, cwd ancestors) are gated on debug_assertions', () => {
  const candidates = functionBody(ffi, 'sdk_candidates');
  assert.ok(candidates, 'sdk_candidates body not found');
  assert.match(candidates, /everything_dll_candidates\(/, 'sdk_candidates must delegate to the pure policy fn');
  assert.match(candidates, /cfg!\(debug_assertions\)/, 'allow_dev_candidates must be cfg!(debug_assertions)');
});

test('no other module loads the Everything DLL itself', () => {
  for (const file of ['src-tauri/src/search/providers/everything.rs', 'src-tauri/src/search_sources/everything.rs']) {
    const text = productionPart(readRust(file));
    assert.ok(!/Library::new|load_with_flags|LoadLibrary/.test(text), `${file} must not load the DLL directly`);
  }
});
