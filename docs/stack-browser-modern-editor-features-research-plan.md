# Stack Browser Modern Editor Features — Research and Implementation Plan

**Date:** 2026-09-16  
**Status:** Proposal / research plan only. No implementation approval.  
**Owner:** Documentation owner.  
**Decision state:** CodeMirror 6 is the preferred candidate for small/full resident documents and bounded future projections, pending explicit experiments below. No gate is newly passed.  
**Boundary:** This document changes no product behavior, source route, save/recovery status, accessibility proof, package manifest, test suite, Rust command, or config.

## 1. Current-state baseline

FACTS:

- `master_spec.md:24` defines current Stack Quick View/Edit boundary: `StackPopupSurface.svelte` routes allowlisted normal text extensions to `StackTextEditor.svelte`; backend `read_stack_basic_text_file` is read-only, UTF-8 only, regular-file/reparse/NUL guarded, and capped at 1 MiB.
- `src/components/StackTextEditor.svelte:23-86` is a bound `<textarea>` draft with parent-owned dirty exit, no save, no local persistence, and no recovery.
- `src/components/StackPopupSurface.svelte:129-131` owns extension allowlist and editor switching.
- `package.json:37-46` pins CodeMirror core modules (`codemirror`, `@codemirror/commands`, `@codemirror/language`, `@codemirror/search`, `@codemirror/state`, `@codemirror/view`) but no syntax language packages.
- Existing coverage anchors: `tests/stackBasicTextEditorUi.test.mjs` and `tests/stackBasicTextFileContract.test.mjs`.
- Canonical deeper plan remains `docs/stack-browser-quick-view-editor-implementation-plan.md`: `stack-text-editor.v2` is canonical; P02 accepted only for non-scale storage; P03 projection/input and P04 save/recovery remain blocked/not passed as recorded; P05-P09 remain unavailable.

NON-GOALS for this plan:

- No claim of save, recovery, product integration beyond current textarea route, huge-file readiness, accessibility pass, XML correctness, or phase promotion.
- No app code, test, package, lockfile, Rust, config, or current behavior edits in this research pass.
- No implementation experiment executed here.

## 2. Recommendation

PROPOSAL: prefer **CodeMirror 6** as renderer/input engine for:

1. Current small/full resident documents inside the 1 MiB draft route.
2. Future bounded projection view if P03 proves selection, IME, accessibility, undo/history, and anchor mapping.

CONSTRAINT: CodeMirror must not be treated as out-of-core/full-file storage. Rust text-document sessions remain backend authority for future document-wide operations, save, conflict, recovery, source bytes, encoding, and privacy.

### Decision table

| Candidate | Fit for current 1 MiB draft | Fit for future projection | Risks / rejects | Decision |
|---|---|---|---|---|
| CodeMirror 6 | Strong: installed core, modular features, Svelte adapter feasible, low chrome ownership | Possible but unproven; needs bounded projection adapter and P03 proof | Not storage engine; careless whole-doc binding breaks scale; language packages unselected | Preferred candidate |
| Monaco | Rich built-in editor UX | Heavy, VS Code model assumptions, worker/bundle/chrome overhead, harder shell fit | Overkill for current small route; no proof for huge projection | Defer unless CodeMirror fails |
| Enhanced textarea | Lowest risk to current behavior | Poor folds, syntax, structured commands, extension model | Can add small affordances but not modern editor feature set | Fallback only |
| Custom/native renderer | Max control for projection/out-of-core | Highest implementation cost; IME/AT/selection hard | Use only if P03 proves CM impossible and product keeps huge-file requirement | Last resort |

## 3. Feature model

All features below are proposals until implemented and verified.

### Language registry and detection

- Add explicit language registry keyed by extension, filename, and optional content sniffing.
- Initial candidates: JSON, JS/TS, Svelte, CSS, HTML, XML, YAML, Markdown, plaintext/log/CSV.
- Use `LanguageDescription.load()` for async language loading. Background loading must not gate first edit.
- JSON package and XML parser package/config remain unselected until compatibility experiments pass.

### Candidate feature matrix

| Feature | Current target | Evidence needed |
|---|---|---|
| Syntax highlighting | Token-style contract using app theme tokens; no semantic guarantee | Visual/theme tests, token class contract, high-contrast review |
| JSON folds | Objects/arrays, valid and malformed JSON degradation | Nested fold tests, malformed docs remain editable |
| XML folds | Elements only, if parser POC passes | XML declaration, namespaces, CDATA, arbitrary self-closing elements, malformed nested tags |
| Fold controls | Gutter affordances plus per-node keyboard expand/collapse | Keyboard, focus, screen reader label/state checks |
| Matching/auto-closing tags | HTML evidence not enough for XML | XML parser gate before XML claim |
| Bracket matching/indentation | Baseline editor feature | Valid/malformed docs, no typing block |
| Line numbers/active line/selection | Current small doc only first; projection later | Theme, selection, reflow, performance |
| Find/replace | CodeMirror search candidate | Shortcut precedence, dirty state, replace edits |
| Autocomplete/linting | Staged/deferred | Scope decision, bundle/perf, diagnostics ownership |
| Formatting/LSP/minimap/semantic diagnostics | Deferred | Separate product decision and backend/service design |

### CodeMirror composition stance

Official research says CodeMirror is modular; `basicSetup` includes baseline keymaps, line numbers, history, folding, bracket matching, autocomplete/linting. PROPOSAL: do not ship blind `basicSetup` until measured. Prefer explicit extension composition so shortcut precedence, theme tokens, bundle size, and accessibility behavior are owned.

## 4. JSON/XML correctness gates

JSON support package selection is unverified. Select only after installed-version compatibility and fold/degradation tests pass.

XML parser selection is a hard gate. `@codemirror/lang-html` supports syntax/highlighting/autocomplete and configurable tag matching/closing, but that does **not** prove conformant XML behavior.

XML parser gate: validate XML declaration, namespaces, CDATA, arbitrary self-closing elements, and malformed nested tags in packaged WebView2; HTML tag auto-close evidence alone does not establish XML correctness.

Malformed or partial documents must remain editable. Parser/decorator/fold failures degrade predictably: no typing block, no lost text, no broken dirty state, no fake structure, and no unbounded foreground work.

## 5. Integration architecture and ownership

PROPOSAL ownership split:

- **Svelte shell owns:** Stack Browser chrome, toolbar/status, app theme, dirty dialog, Escape behavior, file/grid/Git/editor slot switching, focus return, global shortcut/drop precedence.
- **CodeMirror adapter owns:** `EditorView` lifecycle, local resident document state, extension configuration, theme bridge, language loading, local history for draft, listener cleanup, component destruction.
- **Rust text-document session owns future authority:** source identity, file handles, decoding/encoding/BOM/EOL, full-document operations, conflict detection, save publication, recovery, privacy, quotas.

Rules:

- Do not bind whole editor documents through Svelte reactivity.
- Do not move file content through global events or `localStorage`.
- Do not create save-capable UI until P03/P04 and later P05-P09 gates pass.
- Keep current 1 MiB resident draft route separate from future large-file projection. Small-file CodeMirror can land only as draft-only if explicitly scoped.

## 6. Folding strategy

Small resident document:

- CodeMirror fold state may use normal document positions plus transaction mappings.
- `foldGutter` can provide visible fold affordances.
- Expand/collapse commands must update mapped ranges across edits.

Future bounded projection:

- Fold state must use document anchors/mappings owned by projection/session, not stale viewport offsets.
- Fully expanded/collapse-all must not do whole-file sweeps before scale gates.
- Background parser/language loading must not gate first edit.
- No fake unloaded text and no folds spanning unknown content unless backend/projection can prove anchors.

## 7. UX and accessibility contract

PROPOSAL requirements; no accessibility pass claimed.

- Visible gutter, line numbers, fold controls, active line, selection styling, dirty status.
- Discoverable keyboard commands for fold/unfold, find/replace, navigation, close/dismiss.
- Focus enters editor predictably after load; Escape flows through parent dirty guard.
- Screen reader labels for editor, gutter/fold controls, dirty status, find/replace, and fold state changes.
- High contrast, theme changes, DPI, zoom, and narrow reflow must not recreate document or lose caret.
- Existing global shortcut and file-drop precedence must be preserved: editor text operations must not trigger file-grid delete, paste, or filesystem drop behavior.

Required validation includes packaged WebView2, keyboard-only operation, IME composition, NVDA/Narrator or agreed AT procedure, forced-colors/high contrast, DPI/reflow, and dirty close guard.

## 8. Save/recovery path and risk boundary

Folding/syntax may ship only after explicit scope decision. Implementation sequencing may begin after P03/P04 and later P05-P09 pass, but any enabled save-capable product release also requires P10/P11 acceptance. P12/NFR-6 populated scale evidence may remain documented deferred debt only for enabled landing; it never permits a huge/enormous-file-readiness claim before P12 passes.

Backend-owned unresolved risks:

- Source-change detection and conflict classification.
- Encoding/BOM/EOL/final-newline preservation.
- Atomic publication and Windows race/failpoint handling.
- Recovery root privacy, retention, quota, restart classification.
- External mutation during edit/save.
- Privacy of recoverable content and diagnostics.

No frontend renderer feature may imply durable edit safety.

## 9. Phased roadmap and dependencies

| Phase | Scope | Entry deps | Exit evidence | No-go conditions |
|---|---|---|---|---|
| M00 docs decision | Choose small-file-only CM exploration vs projection-coupled exploration | This plan | Product decision, explicit non-goals | Any save/huge-file claim by implication |
| M01 dependency/version POC | Verify installed CM core versions, ESM/Vite/WebView2 import, no syntax packages assumed | M00 | Minimal isolated adapter experiment plan/results in later task | Bundle/startup/memory regression unexplained |
| M02 small resident JSON | JSON syntax/folds/search in draft-only small file | M01, explicit package selection | Valid/malformed/nested fold tests; no block typing | Malformed JSON breaks typing/dirty state |
| M03 XML parser POC | XML parser/package/config selection | M01 | Declaration/namespaces/CDATA/self-close/malformed evidence | HTML support used as XML proof |
| M04 UX/a11y validation | Keyboard, focus, gutter, SR labels/state, contrast/DPI/reflow | M02/M03 subset | Actual WebView2 keyboard/IME/AT results | Synthetic-only accessibility claim |
| M05 projection compatibility | Bounded projection adapter design | P03 unblocked only | P03 evidence for selection/IME/AT/giant grapheme/history | Whole-doc hidden buffer or fake unloaded text |
| M06 save-capable editor | Save/recovery integration | P03 + P04 + P05-P09 passed for implementation; P10/P11 also required before enabled save-capable release | Backend-owned save/recovery proof; release evidence records any unresolved P12/NFR-6 risk without huge-file-readiness claim | Any UI write path before gates; enabled save-capable release before P10/P11 |

Narrow experiments to run in future task only:

- Installed CodeMirror version compatibility with Svelte/Vite/WebView2.
- JSON valid/malformed/nested fold behavior.
- XML declaration, namespaces, CDATA, self-closing, malformed nested tags.
- Actual WebView2 keyboard, IME, and AT validation.
- Lazy-load, bundle, startup, and memory measurement.
- Component destruction/no stale state/no leaked listeners.

## 10. Test and verification matrix

| Area | FR/NFR tie | Planned checks |
|---|---|---|
| Draft-only small editor | FR-1, FR-9, FR-11 | Current route preserved; dirty dialog; Escape; no save controls |
| Renderer lifecycle | FR-12, NFR-2 | Mount/destroy, async language load cancellation, no stale state |
| Syntax/folds | FR-4, FR-5, NFR-1/2 | JSON/XML fold correctness/degradation, fold mappings after edits |
| Keyboard/IME | FR-5/6/11, NFR-8 | Shortcut precedence, composition, find/replace, fold commands |
| Accessibility | NFR-8 | Labels, SR state, focus, high contrast, DPI, reflow |
| Security/privacy | NFR-9 | No localStorage/global event content, no forged save path, bounded diagnostics |
| Performance | NFR-1/2/3/6 | Startup/import, memory, typing latency, bundle deltas |
| Regression | Existing Stack editor tests | `stackBasicTextEditorUi`, `stackBasicTextFileContract`, changelog hygiene |

Decision gates:

- **Small-file-only gate:** CodeMirror may replace textarea only if draft-only semantics, 1 MiB backend cap, dirty parent guard, extension allowlist, and current no-save truth remain exact.
- **Enormous-file gate:** no huge/enormous-file-readiness claim until P12/NFR-6 populated scale evidence passes. Current P12/NFR-6 debt remains an explicit unresolved risk; documented P12 deferral for enabled landing does not satisfy this gate.
- **No-go:** hidden whole-file buffer for huge files, parser failure blocking typing, save affordance without backend gates, HTML parser treated as XML proof, localStorage/global event content transport, inaccessible fold controls.

## 11. Source traceability

- Current product boundary: `master_spec.md:24`.
- Current textarea implementation: `src/components/StackTextEditor.svelte:23-86`.
- Current extension allowlist: `src/components/StackPopupSurface.svelte:129-131`.
- CodeMirror installed core modules: `package.json:37-46`.
- Current source tests: `tests/stackBasicTextEditorUi.test.mjs`, `tests/stackBasicTextFileContract.test.mjs`.
- Canonical research: `docs/stack-browser-quick-view-editor-research.md`.
- Canonical phased plan/status: `docs/stack-browser-quick-view-editor-implementation-plan.md`.
- Changelog policy: `CHANGELOG_POLICY.md`.
- Upstream docs to re-check before implementation: CodeMirror 6 reference/manual/package docs for `basicSetup`, `LanguageDescription.load()`, `foldGutter`, JSON language package, and any XML parser package selected later.
