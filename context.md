# Code Context

## Files Retrieved
1. `src/components/StackPopupSurface.svelte` (lines 260-263, 416-493, 2378-2409) - path input state, key handling, suggestion refresh, Tab cycling, inline accept, and markup.
2. `src/lib/stackPopupViewModel.ts` (lines 66-174) - autocomplete query parsing, inline completion builder, and Tab cycle index helper.
3. `tests/stackBrowserPathAutocomplete.test.mjs` (lines 1-133) - current regression coverage/seams for Stack Browser path autocomplete.
4. `src/lib/stackPopup.ts` (lines 209-218, 632-638) - frontend suggestion request/response types and IPC wrapper.
5. `src-tauri/src/stack_popup.rs` (lines 395-424) - backend directory suggestion command.

## Key Code

`src/components/StackPopupSurface.svelte`:
```ts
$: pathInlineCompletion = getStackPathInlineCompletion(
  pathDraft,
  pathSuggestions[pathCompletionCycleIndex >= 0 ? pathCompletionCycleIndex : 0]
);

function handlePathKeydown(event: KeyboardEvent) {
  event.stopPropagation();
  if (pathInlineCompletion && event.key === 'ArrowRight') {
    event.preventDefault();
    acceptInlinePathCompletion();
    return;
  }
  if (event.key === 'Tab' && !event.shiftKey && pathSuggestions.length) {
    event.preventDefault();
    cyclePathCompletion();
    return;
  }
  ...
}
```

`cyclePathCompletion()` prevents default/opening, keeps existing `pathSuggestions`, computes next index with `getNextStackPathCompletionCycleIndex(pathDraft, pathSuggestions, pathCompletionCycleIndex)`, assigns `pathDraft = suggestion.path`, and focuses caret at end. It does **not** refresh suggestions.

`acceptInlinePathCompletion()` sets `pathDraft` to `pathInlineCompletion.commitPath`, clears suggestions, refocuses, then calls `refreshPathSuggestionsForValue(committedPath, committedPath.length)`.

`src/lib/stackPopupViewModel.ts`:
```ts
export function getNextStackPathCompletionCycleIndex(input, suggestions, currentIndex) {
  if (!suggestions.length) return -1;
  const exactCurrentIndex = suggestions.findIndex(
    (suggestion) => normalizeStackPathForAutocomplete(suggestion.path) === normalizeStackPathForAutocomplete(input)
  );
  const baseIndex = currentIndex >= 0 ? currentIndex : exactCurrentIndex;
  return (baseIndex + 1) % suggestions.length;
}
```
This already skips an exact typed directory when that exact path is in the current suggestion set.

Backend `suggest_stack_paths(parent_path, segment, limit)` returns sorted child directories under `parent_path` whose names start with `segment`.

## Architecture

Path input events flow: Svelte input `on:focus`/`on:input` -> `refreshPathSuggestions()` -> `getStackPathAutocompleteQuery(value, caret)` -> IPC `suggestStackPaths()` -> Rust `suggest_stack_paths()` -> `pathSuggestions`. Inline ghost is derived from the first/current suggestion. `ArrowRight` accepts inline completion. `Tab` only cycles if `pathSuggestions.length` is nonzero; `Shift+Tab` is intentionally ignored by this branch.

Likely current bug/regression: Tab does not have a fallback for the visible/current inline completion when `pathSuggestions` is empty/stale/cleared. The current branch is gated by `pathSuggestions.length`, while RightArrow is gated by `pathInlineCompletion`. After `acceptInlinePathCompletion()` suggestions are explicitly cleared and repopulated async, so immediate Tab after accepting a directory can be dropped until the async refresh returns. Also, cycling sibling directories for an already typed exact directory depends on suggestions for that exact prefix being present; if they are not loaded yet, Tab does nothing.

## Start Here

Open `src/components/StackPopupSurface.svelte` at `handlePathKeydown()`/`cyclePathCompletion()` first. Smallest likely implementation: make Tab handle `pathInlineCompletion`/current suggestion robustly instead of requiring only `pathSuggestions.length`; ensure if suggestions are missing after a typed exact directory/prefix, Tab triggers/uses refreshed suggestions for `getStackPathAutocompleteQuery(pathDraft, pathDraft.length)` and then cycles in place without opening. Keep `Shift+Tab` unchanged. Reuse `getNextStackPathCompletionCycleIndex()`; it already implements exact-prefix skip.

Regression-test seam: extend `tests/stackBrowserPathAutocomplete.test.mjs` with source-level assertions around the Tab branch plus pure helper tests. Better seam if adding logic: extract a small pure helper in `src/lib/stackPopupViewModel.ts` for resolving Tab behavior/next suggestion from `{input, suggestions, currentIndex, inlineCompletion}` and test it directly. Existing tests import from `../dist-tests/lib/stackPopupViewModel.js` and already cover exact-directory skip; add cases for Tab accepting currently suggested inline completion and Tab on exact typed prefix cycling to sibling when suggestions include exact + siblings.

## Constraints/Risks/Open Questions

- Do not make Tab submit/open a folder; current tests assert no `openFolder(committedPath)`.
- Preserve RightArrow behavior and Shift+Tab non-handling.
- Async refresh race guard (`requestSeq !== pathSuggestionRequestSeq || value !== pathDraft`) may discard responses if Tab mutates `pathDraft`; avoid clearing original suggestion set during cycle.
- If implementing async Tab fetch, be careful to prevent default immediately and not lose focus/caret.
