# Stack Browser Modern Editor Features — Status and Deferred Plan

**Date:** 2026-09-16  
**Status:** Partial implementation complete: M01 resident adapter and M02 scoped syntax highlighting are implemented for the draft-only route. Later features remain deferred.
**Owner:** Documentation owner.  
**Decision state:** CodeMirror 6 is implemented with async extension-keyed syntax for Markdown, JSON, JS/TS, Svelte, CSS, HTML, XML, and YAML/YML. Folds, projection, save/recovery, accessibility proof, performance proof, and production-readiness gates are not passed.
**Boundary:** This document records truthful status only. It changes no product behavior, source route, save/recovery status, package manifest, test suite, Rust command, config, P03/P04 status, or master spec.

## 1. Status summary

| Area | Status | Truth boundary |
|---|---|---|
| Small resident CM6 adapter | **Implemented (2026-09-16)** | `StackTextEditor.svelte` mounts `stackTextEditorAdapter` after a successful current file read; it remains in-memory and draft-only. |
| Explicit CM extension composition | **Implemented (2026-09-16)** | Adapter uses explicit history, default/history/search keymaps, search, line numbers, active line, draw selection, bracket matching, update listener, and theme bridge. It does not use blind `basicSetup`. |
| Lifecycle/focus/Escape split | **Implemented (2026-09-16)** | Adapter owns `EditorView` lifecycle, local resident draft, local history, cleanup, destroy/remount, Escape-to-parent dismiss, and focus-at-start after Svelte tick plus browser animation frame. |
| Svelte/Rust ownership split | **Implemented to current small-file scope** | Svelte owns load/path staleness, chrome/status, and parent dirty guard. Rust remains unchanged: `read_stack_basic_text_file` is read-only 1 MiB UTF-8 authority. |
| Language registry / syntax packages | **Implemented to M02 syntax scope (2026-09-16)** | Async explicit registry covers JSON, JS/TS, Svelte, CSS, HTML, XML, YAML/YML, and Markdown. TXT/CSV/LOG stay plain. XML is syntax-only; folds/tags remain deferred. |
| Autocomplete/lint/format/LSP/minimap/semantic diagnostics | **Deferred** | No capability added or validated. |
| Accessibility/manual runtime proof | **Deferred** | No packaged WebView2/IME/AT/high-contrast/DPI manual proof claimed. |
| Performance/bundle/memory proof | **Deferred** | No measurement claim. Existing build chunk-size warning remains non-failing. |
| Projection / large files | **Deferred** | No bounded projection, huge-file readiness, or P03 gate change. |
| Save/recovery/conflict/encoding | **Deferred** | No save, persistence, recovery, encoding preservation, conflict handling, or P04 gate change. |

## 2. Current-state baseline

FACTS:

- `master_spec.md` defines the current Stack Quick View/Edit boundary: `StackPopupSurface.svelte` routes allowlisted normal text extensions to `StackTextEditor.svelte`; backend `read_stack_basic_text_file` is read-only, UTF-8 only, regular-file/reparse/NUL guarded, and capped at 1 MiB.
- `src/components/StackTextEditor.svelte` now uses `src/features/stack-browser/stackTextEditorAdapter.ts` for the visible editor after a successful current load.
- The editor remains draft-only: no save, no write IPC, no local persistence, no recovery, no projection, no huge-file readiness.
- `src/components/StackPopupSurface.svelte` still owns extension allowlist and editor switching.
- `package.json` pins CodeMirror core modules plus language packages used by the implemented Markdown, JSON, JS/TS, Svelte, CSS, HTML, XML, and YAML/YML syntax registry.
- Existing coverage anchors include `tests/stackBasicTextEditorUi.test.mjs` and `tests/stackBasicTextFileContract.test.mjs`.
- Canonical deeper plan remains `docs/stack-browser-quick-view-editor-implementation-plan.md`: `stack-text-editor.v2` is canonical; P02 accepted only for non-scale storage; P03 projection/input and P04 save/recovery remain blocked/not passed as recorded; P05-P09 remain unavailable.

NON-GOALS / EXCLUSIONS:

- No claim beyond scoped syntax highlighting: folds, XML correctness/tag completion, autocomplete, lint, formatting, LSP, minimap, semantic diagnostics, save, recovery, huge-file readiness, packaged runtime accessibility pass, and phase promotion remain excluded.
- No P03/P04 artifact, experiment, plan, or gate status changed by this document.

## 3. Recommendation

CURRENT RECOMMENDATION: keep **CodeMirror 6** for the implemented small/full resident 1 MiB draft route, with explicit extension composition and strict no-save truth.

FUTURE RECOMMENDATION: evaluate language packages, folds, accessibility, performance, projection, and save/recovery only through separate gated work. CodeMirror must not be treated as out-of-core/full-file storage. Rust text-document sessions remain backend authority for any future document-wide operations, save, conflict, recovery, source bytes, encoding, and privacy.

### Decision table

| Candidate | Current result | Future status | Risks / rejects | Decision |
|---|---|---|---|---|
| CodeMirror 6 | Implemented for current 1 MiB draft route through adapter and scoped syntax registry | Possible for more features, unproven for projection/save | Not storage engine; folds/XML correctness unproven; no P03/P04 pass | Keep for draft route; gate future work |
| Monaco | Not implemented | Deferred | Heavy, VS Code model assumptions, worker/bundle/chrome overhead | Defer unless CM fails future gates |
| Enhanced textarea | Replaced for current route | Fallback only | Poor modern editor feature model | Keep only as conceptual fallback |
| Custom/native renderer | Not implemented | Last resort | Highest cost; IME/AT/selection hard | Use only if future evidence rejects CM |

## 4. Feature model

### Implemented now

- Resident draft editing through CodeMirror 6 for allowlisted basic text files already accepted by the current small-file route.
- Explicit extensions: history, default/history/search keymaps, search, line numbers, active line, draw selection, bracket matching.
- Adapter-local `EditorView` lifecycle, update listener, resident draft state, history, cleanup, destroy/remount handling, theme bridge, Escape-to-parent dismiss.
- Focus-at-start after successful current load, Svelte paint boundary, and browser animation frame.
- Async extension-keyed syntax for Markdown, JSON, JS/TS, Svelte, CSS, HTML, XML, and YAML/YML; TXT/CSV/LOG/unknown/extensionless paths remain plain. XML is syntax-only.

### Deferred proposals

- Filename/content sniffing beyond the implemented path-extension registry.
- JSON/XML fold behavior and XML correctness/package configuration evidence.

### Candidate feature matrix

| Feature | Status | Evidence needed before claim |
|---|---|---|
| Syntax highlighting | Implemented to scoped languages | Explicit token theme and async registry source contracts; high-contrast/runtime proof remains deferred |
| JSON folds | Deferred | Valid/malformed/nested fold tests; malformed docs remain editable |
| XML folds/tags | Deferred | Declaration, namespaces, CDATA, self-closing elements, malformed nested tags in packaged WebView2 |
| Fold controls | Deferred | Keyboard, focus, screen reader label/state checks |
| Matching/auto-closing tags | Deferred | XML-specific parser gate before XML claim |
| Bracket matching | Implemented baseline extension | Further language-aware behavior remains deferred |
| Line numbers/active line/selection | Implemented baseline extension for small docs | Theme/runtime/manual proof still deferred |
| Find/search | Implemented baseline search extension/keymap | Replace UX and full shortcut/runtime proof deferred |
| Autocomplete/linting | Deferred | Scope decision, bundle/perf, diagnostics ownership |
| Formatting/LSP/minimap/semantic diagnostics | Deferred | Separate product decision and backend/service design |

## 5. JSON/XML correctness gates

JSON syntax package selection is implemented; JSON correctness/fold behavior remains unverified and unclaimed.

XML syntax package selection is implemented, but XML correctness remains a hard gate. HTML or syntax coloration does **not** prove conformant XML behavior.

XML parser gate: validate XML declaration, namespaces, CDATA, arbitrary self-closing elements, and malformed nested tags in packaged WebView2; HTML tag auto-close evidence alone does not establish XML correctness.

Malformed or partial documents must remain editable. Parser/decorator/fold failures must degrade predictably: no typing block, no lost text, no broken dirty state, no fake structure, and no unbounded foreground work.

## 6. Integration architecture and ownership

Implemented current ownership split:

- **Svelte shell owns:** successful-load/current-path sequencing, stale response rejection, Stack Browser chrome/status, dirty dialog/parent guard, file/grid/Git/editor slot switching, and current no-save truth.
- **CodeMirror adapter owns:** `EditorView` lifecycle, local resident document state, explicit extension configuration, theme bridge, local history for draft, update listener, editor-local Escape routing to parent dismiss, focus-at-start, listener cleanup, and destruction/remount cleanup.
- **Rust command owns current source read only:** `read_stack_basic_text_file` remains the read-only 1 MiB UTF-8 authority. Rust was intentionally unchanged by the CM6 adapter work.
- **Rust text-document session owns future authority:** source identity, file handles, decoding/encoding/BOM/EOL, full-document operations, conflict detection, save publication, recovery, privacy, and quotas remain future work.

Rules:

- Do not bind whole editor documents through Svelte reactivity.
- Do not move file content through global events or `localStorage`.
- Do not create save-capable UI until P03/P04 and later P05-P09 gates pass.
- Keep current 1 MiB resident draft route separate from future large-file projection.
- Small-file CodeMirror is allowed only as draft-only unless future gates change.

## 7. Folding strategy

Folding is not implemented.

Small resident document proposal:

- CodeMirror fold state may use normal document positions plus transaction mappings.
- `foldGutter` can provide visible fold affordances only after accessibility/runtime validation.
- Expand/collapse commands must update mapped ranges across edits.

Future bounded projection proposal:

- Fold state must use document anchors/mappings owned by projection/session, not stale viewport offsets.
- Fully expanded/collapse-all must not do whole-file sweeps before scale gates.
- Background parser/language loading must not gate first edit.
- No fake unloaded text and no folds spanning unknown content unless backend/projection can prove anchors.

## 8. UX and accessibility contract

Implemented current UX:

- CodeMirror renders the small draft editor with line numbers, active line, selection drawing, search keymap/extension, bracket matching, local history, and adapter-owned focus-at-start.
- Escape in the editor routes to parent dismiss/dirty guard rather than bypassing parent ownership.
- Status remains draft-only/no-save.

Deferred validation / no claim:

- Screen reader labels/state, gutter/fold controls, find/replace UI, high contrast, theme changes, DPI, zoom, narrow reflow, IME composition, and packaged WebView2 behavior are not proven.
- Required future validation includes packaged WebView2, keyboard-only operation, IME composition, NVDA/Narrator or agreed AT procedure, forced-colors/high contrast, DPI/reflow, and dirty close guard.

## 9. Save/recovery path and risk boundary

No frontend renderer feature may imply durable edit safety.

Save/recovery/conflict/encoding remain deferred backend-owned risks:

- Source-change detection and conflict classification.
- Encoding/BOM/EOL/final-newline preservation.
- Atomic publication and Windows race/failpoint handling.
- Recovery root privacy, retention, quota, restart classification.
- External mutation during edit/save.
- Privacy of recoverable content and diagnostics.

Any enabled save-capable product release still requires the appropriate P03/P04/P05-P09 gates and P10/P11 acceptance. P12/NFR-6 scale evidence remains unresolved for huge/enormous-file-readiness claims.

## 10. Deferred implementation plan: execution rules

All phases below are future work. Status starts **Not started**. This plan does not authorize implementation, report a passing result, or change P03/P04 or product-readiness status.

### 10.1 Fixed scope and invariants

Every phase MUST preserve these boundaries:

- Current route remains resident, UTF-8, read-only-from-Rust, memory-only draft editing with the existing 1 MiB backend cap.
- `StackPopupSurface.svelte` retains extension allowlist, surface switching, and the single parent-owned dirty-exit guard.
- `StackTextEditor.svelte` retains read/path staleness, chrome/status, and post-`tick`/`requestAnimationFrame` adapter mounting/focus ownership.
- `stackTextEditorAdapter.ts` remains the production `EditorView` owner. Language installation stays asynchronous and non-blocking; loader/parser failure or late completion cannot block typing, lose text, corrupt dirty state, or revive a destroyed view.
- Malformed JSON/XML remains editable. Structure features MUST disappear or safely reopen rather than invent structure.
- Fold persistence across edits MUST use CodeMirror transaction/change mappings accepted by M03-00, never stored absolute offsets from an earlier document revision.
- No save, write IPC, invoke, local/session storage, global content event, recovery, projection, hidden whole-file transport, or new Rust/capability surface.
- `Draft only — saving is not available yet` remains visible. No phase may claim durability, recovery, large-file readiness, P03 progress, or product readiness.
- This plan is only for the current resident route. `docs/stack-browser-quick-view-editor-implementation-plan.md` remains authoritative for blocked P03 projection and later save/recovery work.

### 10.2 Evidence classes

Evidence MUST be labelled; one class cannot substitute for another:

1. **Automated behavior evidence:** executable state/parser/editor tests with assertions on document, ranges, transactions, dirty callbacks, undo, and failure behavior. Source-string tests may protect wiring/exclusions but cannot prove UI, parser correctness, keyboard behavior, focus, WebView2, IME, or assistive technology.
2. **Test-only packaged feasibility evidence:** isolated synthetic fixture in actual packaged WebView2, unreachable from normal Stack Browser product routes. Passing this class permits a later product proposal only; it is not a product change or claim.
3. **Product packaged runtime evidence:** installed/release package using the normal `stack-popup` editor route and synthetic non-sensitive fixtures. Required before fold-control or tag-accessibility claims.

Each run record MUST include phase ID, date, owner/reviewer roles, git revision or patch hash, exact package versions, fixture hashes, commands with exit codes, expected/observed results, screenshots or AT notes where relevant, failures, and explicit `ACCEPT`, `REJECT`, or `BLOCKED`. Proposed evidence root: `test-results/stack-browser-modern-editor/<phase>/<run-id>/`; follow repository ignore/privacy rules and never copy user documents.

### 10.3 Documentation/API uncertainty gate

Context7 research is unavailable because monthly quota is exhausted. Therefore names, exports, keymaps, configuration, and package requirements not already proven in repository source are **unverified candidates**, not implementation facts. M03-00 is mandatory first work in any implementation wave. No package, lockfile, adapter, language registry, component, or test API change may precede its accepted record.

## 11. Dependency graph and feature trace

```text
M03-00 exact-version official documentation/API freeze
  +-> M03a JSON fold feasibility (test-only)
  \-> M03b XML parser/fold feasibility (test-only)
         \-> M05a XML matching/auto-close feasibility (test-only)
M03a + M03b -> M04a fold-control accessibility feasibility (test-only packaged WebView2)
M03a + M03b + M04a -> M04b product JSON/XML folds and controls
M03b + M05a + M04b -> M05b product XML matching/auto-close
M04b + M05b -> M06 combined packaged qualification and handoff
```

M03a and M03b may run in parallel after M03-00. M05a may run beside M04a after M03b. Product phases serialize adapter ownership. Failed, rejected, missing, browser-only, or source-string-only evidence blocks descendants.

| Requested feature row | Feasibility owner | Product owner | Final evidence gate |
|---|---|---|---|
| JSON folds | M03a | M04b | M06 valid/malformed/nested/edit-mapping matrix |
| XML folds and tags | M03b | M04b | M06 declaration/namespaces/CDATA/self-closing/malformed matrix |
| Fold controls | M04a | M04b | M06 keyboard/focus/name/state + packaged AT proof |
| Matching/auto-closing tags | M05a | M05b | M06 XML-specific parser, IME, undo, dirty, packaged AT proof |

| Phase | Status | Promotion dependency | Product behavior changed in phase? |
|---|---|---|---|
| M03-00 docs/API freeze | Not started | Approved implementation handoff | No |
| M03a JSON fold feasibility | Not started | M03-00 accepted | No; test-only |
| M03b XML parser/fold feasibility | Not started | M03-00 accepted | No; test-only |
| M04a fold-control a11y feasibility | Not started | M03a + M03b accepted | No; test-only packaged harness |
| M04b product folds/controls | Not started | M03a + M03b + M04a accepted | Yes, current draft route only |
| M05a XML tag feasibility | Not started | M03b accepted | No; test-only |
| M05b product XML tags | Not started | M03b + M05a + M04b accepted | Yes, XML current draft route only |
| M06 combined qualification | Not started | M04b + M05b implemented | No new behavior; qualification only |

## 12. Detailed gated phases

### M03-00 — Exact-version official documentation and API freeze

**Scope/context**

First implementation task. Convert current dependency facts into an exact, reviewable API/package decision before code or manifest changes. Current pinned facts include `@codemirror/lang-json` 6.0.2, `@codemirror/lang-xml` 6.1.0, `@codemirror/language` 6.12.4, `@codemirror/state` 6.7.4, and `@codemirror/view` 6.43.11; no explicit `@codemirror/autocomplete` dependency is present. This phase verifies, but does not change, them.

**Dependencies**

- Approved future implementation handoff.
- Current `package.json` and lockfile available.
- Access to official CodeMirror reference/manual, exact package release notes/source/type declarations, and installed package metadata. Context7 absence must be recorded, not bypassed with memory or third-party snippets.

**Implementation tasks**

1. Record resolved versions and integrity from lockfile/installed metadata.
2. Verify from official exact-version material which installed packages own: fold range discovery, fold state/commands/key bindings, fold-range mapping through transactions, gutter rendering, JSON parser support, XML parser support, matching-tag support, and XML auto-close support.
3. Record exact imports/exports/configuration only after verification. Mark unsupported, transitive-only, deprecated, or version-mismatched candidates rejected.
4. Determine whether each feature needs no package delta, an explicit existing transitive package declaration, or a new direct package. No package change occurs in this phase.
5. Freeze behavioral decisions: mapped fold range invalidation, malformed-document fallback, XML-only tag gate, keyboard command precedence, async reconfiguration, and no generic completion UI.
6. Define test-harness approach for parser/state behavior and isolated packaged WebView2. Inventory an existing repository packaged-probe pattern before proposing a new one.

**Test commands/procedures**

```powershell
npm ls @codemirror/commands @codemirror/lang-json @codemirror/lang-xml @codemirror/language @codemirror/state @codemirror/view --depth=0
npm ls @codemirror/autocomplete --depth=0
node --test tests/stackBasicTextEditorUi.test.mjs
```

Autocomplete inventory may exit nonzero because absence is expected; record that outcome explicitly. Reviewer compares every proposed import/configuration against official exact-version evidence and installed type declarations. No build is evidence of API correctness.

**Acceptance criteria**

- Version/API matrix names official URL or package source, exact version, exact export/config, owning package, intended phase, and fallback.
- Package delta is explicit and minimal; no tag feature silently enables broad autocompletion.
- Fold state design explicitly uses transaction mappings and defines safe reopening when mapped structure becomes invalid.
- Test-only packaged harness is isolated from product routing and uses synthetic documents.
- Reviewer records `ACCEPT`; all later phase test recipes can be implemented without guessing API names.

**No-go/reject criteria**

- Treating Context7 quota failure as evidence, applying latest-doc examples to pinned older versions, transitive package assumed stable, HTML behavior accepted as XML behavior, or unknown API names/config committed.
- Any product/package/lockfile change, `foldGutter` product proposal, or completion dependency addition before this gate accepts.

**File ownership / likely paths**

- Documentation/integration owner: this plan plus a focused evidence record under `docs/` or ignored test-results.
- Read-only inspection: `package.json`, lockfile, installed package declarations, `stackTextEditorAdapter.ts`, `stackTextEditorLanguages.ts`.
- No product, manifest, lockfile, Rust, capability, or test changes.

**Handoff/exit evidence**

Version/API matrix, official references, package-delta decision, harness decision, fixture list, command log, reviewer disposition, and exact RED test names for M03a/M03b/M04a/M05a. Any unresolved export/package question exits `BLOCKED`.

### M03a — JSON fold feasibility, test-only

**Scope/context**

Prove JSON structural folds against current resident-document assumptions without changing production adapter or exposing controls. Covers valid, malformed, nested JSON plus edit mapping and dirty behavior.

**Dependencies**

- M03-00 `ACCEPT`.
- Exact documented fold/parser symbols and test harness frozen.

**Implementation tasks**

1. Add meaningful RED behavioral tests before feasibility implementation. RED must fail on absent fold behavior, not missing imports or unconditional placeholders.
2. Build isolated test-only state/parser fixture using only M03-00-approved APIs.
3. Cover nested objects/arrays, empty objects/arrays, top-level scalar, escaped strings containing braces/brackets, edits before/inside/after folded regions, and malformed/truncated JSON.
4. Assert range mapping uses transaction mappings. When node identity survives, folded state follows mapped content; when parse structure becomes invalid/ambiguous, affected fold safely opens/disappears.
5. Assert fold/unfold itself does not change document or dirty state; text edits still call existing dirty path correctly.
6. Keep parser/language work optional and non-blocking; injected load/parse failure leaves typing and dirty tracking functional.

**Test commands/procedures**

Planned focused command after test file exists:

```powershell
node --test tests/stackTextEditorJsonFolding.test.mjs
node --test tests/stackBasicTextEditorUi.test.mjs
```

Record initial RED output, minimal GREEN output, and mutation traces for mapped ranges. Source-string assertions may verify production adapter remains unchanged but cannot satisfy this gate.

**Acceptance criteria**

- Valid nested object/array ranges fold and unfold to exact parser-derived boundaries.
- Mapped folds survive preceding and internal valid edits without stale offsets; invalidated ranges reopen safely.
- Malformed/truncated JSON remains editable; no false structure assertion, lost text, exception escape, typing block, or dirty-state regression.
- Fold operations are document-neutral; edits remain undoable and dirty tracking reflects content only.
- Production adapter, language registry, package manifest, and route remain unchanged.

**No-go/reject criteria**

- Regex/brace counting presented as JSON parser proof; stale numeric offsets; whole-document reparsing that blocks first edit; malformed JSON disables typing; feasibility code imported by product; save/persistence/projection addition.

**File ownership / likely paths**

- QA/editor feasibility owner: proposed `tests/stackTextEditorJsonFolding.test.mjs` and narrowly scoped helper/fixture under `tests/fixtures/stack-text-editor/`.
- Production paths are read-only in this phase.

**Handoff/exit evidence**

RED/GREEN logs, fixture hashes, fold-range/mapping traces, failure-injection results, explicit proof production bundle/routes did not change, and reviewer `ACCEPT`/`REJECT`/`BLOCKED`. Accepted output feeds M04a/M04b only.

### M03b — XML parser and fold feasibility, test-only

**Scope/context**

Prove XML-specific parse structure and fold semantics. "Tags" here means parser-recognized XML element boundaries/ranges, not matching or auto-close product behavior. HTML evidence is inadmissible.

**Dependencies**

- M03-00 `ACCEPT`.
- Exact `@codemirror/lang-xml` version/config and test APIs frozen from official evidence.

**Implementation tasks**

1. Add RED XML structure/fold behavioral tests before feasibility implementation.
2. Use XML-specific parser configuration only; record parser identity in evidence.
3. Required corpus: XML declaration; default and prefixed namespaces; namespaced element/attribute names; CDATA containing `<`, `>`, and tag-like text; comments; processing instructions; arbitrary named self-closing elements; nested same-name elements; entities in text/attributes; malformed crossed, missing, and partial tags.
4. Assert declaration/CDATA/comment/processing-instruction text does not fabricate element pairs or child folds.
5. Assert arbitrary self-closing elements need no closing partner and do not receive a body fold.
6. Assert malformed nested tags remain editable and ambiguous ranges are absent/safely removed.
7. Cover transaction-mapped edits before/inside/after XML ranges and async parser failure.

**Test commands/procedures**

```powershell
node --test tests/stackTextEditorXmlStructure.test.mjs
node --test tests/stackBasicTextEditorUi.test.mjs
```

Inspect parser/range traces for every required corpus row. Run negative controls through HTML configuration only to demonstrate that HTML results are not counted; those outputs are diagnostic, never acceptance evidence.

**Acceptance criteria**

- Every required XML corpus row has executable expected/observed parser and fold assertions.
- Declaration, namespaces, CDATA, arbitrary self-closing elements, and malformed nested tags behave as specified above.
- Fold ranges derive from unambiguous XML structure and map through edits; malformed ambiguity safely removes controls/ranges.
- Typing, undo, dirty callbacks, and async language-failure fallback remain functional.
- No product or package change.

**No-go/reject criteria**

- HTML parser/package result, syntax color, snapshot, regex, or source-string assertion presented as XML correctness.
- Parser repairs malformed nesting into fake pairs; CDATA content creates tags; only a fixed allowlist of self-closing names works; failure blocks input.

**File ownership / likely paths**

- XML feasibility/QA owner: proposed `tests/stackTextEditorXmlStructure.test.mjs`, synthetic XML fixtures, narrow test helper.
- `stackTextEditorLanguages.ts` and adapter remain read-only.

**Handoff/exit evidence**

Corpus matrix, parser identity/version, RED/GREEN logs, range/mapping traces, malformed/failure results, and reviewer disposition. M03b acceptance is mandatory for every later XML fold/tag claim.

### M04a — Fold-control accessibility feasibility, test-only packaged WebView2

**Scope/context**

Evaluate controls before any production `foldGutter` UI. Build smallest isolated test-only packaged WebView2 fixture using M03-00-approved APIs and synthetic JSON/XML. This phase selects or rejects control design; it does not wire normal Stack Browser.

**Dependencies**

- M03a and M03b `ACCEPT`.
- M03-00-approved packaged probe pattern.
- Accessibility owner and packaged-runtime operator assigned.

**Implementation tasks**

1. Add RED executable checks for control semantics where automation can inspect them: control count/range, accessible name, expanded/collapsed state, keyboard action equivalence, focus retention/restoration, and document neutrality.
2. Create isolated packaged fixture. It MUST be unreachable from ordinary `stack-popup` routes, contain no file IPC/content transport, and have explicit removal/retention decision.
3. Evaluate candidate gutter and non-gutter control designs. Product `foldGutter` is not proposed until one design passes.
4. Require each operable fold control to expose a unique range-aware name, current expanded/collapsed state, visible focus, and same action from keyboard and pointer. Define announcement behavior without flooding live regions.
5. Verify editor navigation, Tab/Shift+Tab policy, Escape-to-parent behavior, selection, undo/history, and dirty state remain intact.
6. Run actual packaged WebView2 with keyboard only and NVDA plus Narrator where available; record version, verbosity settings, spoken output, focus order, and state changes. Also run IME composition adjacent to fold boundaries.

**Test commands/procedures**

Planned harness commands, created by this phase and recorded exactly in evidence:

```powershell
node --test tests/stackTextEditorFoldControlsA11y.test.mjs
pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/stack-text-editor/launch-modern-editor-fold-a11y.ps1 -Configuration Release
```

Manual packaged procedure:

1. Open nested JSON, collapse/expand first and nested ranges with keyboard only, then pointer.
2. Repeat XML namespace/nested fixture; confirm no control for declaration, CDATA pseudo-tags, or self-closing elements.
3. Traverse controls/editor/chrome forward and backward; confirm visible focus and deterministic return to editor caret.
4. With each agreed screen reader, read control name/state, operate it, confirm changed state is announced once and nearby code remains navigable.
5. Start and commit non-Latin IME composition before, inside, and after a fold boundary; confirm one commit, no focus loss, no accidental toggle.
6. Edit around a folded range, undo/redo, trigger malformed structure, then Escape; confirm mapped/reopened state, dirty guard, and parent ownership.

**Acceptance criteria**

- Automated semantics pass, plus actual packaged WebView2 keyboard/focus/AT procedure passes with recorded observations.
- One control pattern has keyboard parity, visible focus, unique label, exposed state, deterministic focus behavior, and no dirty/document mutation.
- XML controls appear only for XML-parser-proven ranges.
- IME and parser/language failures do not block typing or steal focus.
- Accessibility owner and product reviewer record `ACCEPT` for design. Acceptance permits M04b proposal; it does not claim product support.

**No-go/reject criteria**

- Browser-only, DOM snapshot, source-string, screenshot, or ARIA-presence-only proof.
- Mouse-only gutter; unnamed controls; state unavailable to AT; fold action changes text/dirty state; Tab trap/explosion; focus loss; duplicate announcements; HTML evidence for XML.
- Test fixture reachable in normal product or requiring Rust/capability/save changes.

**File ownership / likely paths**

- Accessibility/QA owner: proposed test file, synthetic fixtures, manual evidence form.
- Integrator: proposed test-only launcher under `scripts/stack-text-editor/` and isolated packaged fixture path selected in M03-00.
- Production adapter/component/language registry remain unchanged.

**Handoff/exit evidence**

Packaged artifact hash, WebView2/Windows/AT/IME versions, automated logs, completed manual matrix, screenshots only as supplemental evidence, design decision, harness lifecycle decision, reviewer dispositions. Any inaccessible result keeps fold controls deferred and blocks M04b.

### M04b — Product JSON/XML folds and accessible fold controls

**Scope/context**

Integrate accepted JSON/XML fold behavior and accepted control design into current 1 MiB draft-only production route. This is first phase allowed to propose product fold UI. Scope excludes matching/auto-close tags.

**Dependencies**

- M03a, M03b, and M04a each `ACCEPT`.
- M03-00 package/API decision still matches lockfile and source.
- Explicit owner approval for any manifest/lockfile delta.

**Implementation tasks**

1. Update applicable existing negative source-contract assertions RED-first so they reject premature/incorrect wiring but no longer require folds to be absent. Add runtime behavioral tests before adapter changes.
2. Compose only M03-00-approved fold state/commands/control extensions in adapter. Do not add blind setup bundles or generic completion.
3. Enable folds only after matching async JSON/XML language install; plain text and other languages retain current behavior.
4. Preserve fold state via transaction mappings. Structural invalidation safely reopens/removes affected fold; never retain stale offsets.
5. Apply M04a-approved keyboard/focus/name/state design. Add theme rules only for accepted controls, using shell tokens and forced-colors-safe treatment.
6. Preserve mount/destroy/remount, font/language compartments, focus-at-start, Escape parent dirty guard, history/search keymaps, and loader rejection/destroy safety.
7. Keep fold/unfold document-neutral. Text edits inside/around folded ranges must update draft/dirty once and remain undoable.
8. If package delta was approved, change `package.json` and lockfile only after RED evidence; direct-declare required runtime package and record bundle delta. Otherwise do not touch manifest.

**Test commands/procedures**

```powershell
node --test tests/stackTextEditorJsonFolding.test.mjs tests/stackTextEditorXmlStructure.test.mjs tests/stackTextEditorFoldControlsA11y.test.mjs tests/stackBasicTextEditorUi.test.mjs
npm run check
npm run build
npm run tauri -- build
```

Run M04a manual procedure against normal packaged `stack-popup`, not feasibility fixture. Re-test loading rejection and completion-after-destroy. Record existing build warning separately; do not call build success runtime/a11y proof.

**Acceptance criteria**

- JSON and XML valid/nested folds work in normal route; all malformed/edit-mapping/failure cases pass.
- Controls satisfy accepted keyboard/focus/label/state contract in actual packaged WebView2 with agreed AT.
- No controls/folds leak to unsupported/plain languages or unproven XML ranges.
- Typing starts before async language install; failure cannot block typing. Dirty guard, Escape, undo/history, selection, font changes, and destruction remain correct.
- Draft-only notice, 1 MiB cap, no-save/no-persistence boundary, and P03 blocked status remain exact.

**No-go/reject criteria**

- Production `foldGutter` before M04a acceptance; source-string-only UI claim; stale fold offsets; parser work on first-edit critical path; broad setup/autocomplete; hidden content transport; package creep; save/projection claim.

**File ownership / likely paths**

- Frontend editor owner: `src/features/stack-browser/stackTextEditorAdapter.ts`; possibly `stackTextEditorLanguages.ts` only if M03-00 requires language-specific extension composition.
- Component/accessibility owner: `src/components/StackTextEditor.svelte` only if accepted external control/status semantics require it.
- QA owner: named fold tests and existing `tests/stackBasicTextEditorUi.test.mjs`.
- Integrator alone: `package.json` and lockfile if approved. No `StackPopupSurface.svelte`, Rust, capability, master-plan, or save-path change expected.

**Handoff/exit evidence**

Focused RED/GREEN logs, package/bundle delta, normal-route packaged manual matrix, AT/IME evidence, source diff ownership report, regression results, reviewer `ACCEPT`/`REJECT`/`BLOCKED`. Exit authorizes M05b work only; no release/readiness claim.

### M05a — XML matching and auto-close feasibility, test-only

**Scope/context**

Prove exact XML-specific matching and auto-close semantics before product wiring or dependency addition. HTML evidence cannot stand in. No generic autocomplete popup is in scope.

**Dependencies**

- M03b `ACCEPT`.
- M03-00 exact API/package matrix accepted.
- If official evidence says a missing direct package is required, approval to test that dependency in isolated feasibility only; product manifest remains unchanged.

**Implementation tasks**

1. Write RED behavioral tests for passive matching and auto-close transactions.
2. Define matching: when caret is on/inside an unambiguous XML opening or closing tag, expose exact parser-paired counterpart; namespace-qualified names must match exactly. Malformed/crossed/partial nesting yields no invented match.
3. Define auto-close: typing `>` for an unambiguous XML non-self-closing start tag inserts its exact closing tag with caret between. Do not auto-close declarations, processing instructions, comments, CDATA, closing tags, already self-closed arbitrary elements, or ambiguous malformed parser states.
4. Require inserted pair to be one undo group and one content/dirty update. Passive matching and navigation are document-neutral.
5. Cover namespace prefixes, nested same-name elements, attributes containing `>` or tag-like text, arbitrary self-closing elements, paste, selection replacement, backspace, undo/redo, and asynchronous parser arrival/failure.
6. Cover IME/composition event sequences so composition commit does not duplicate `>` or closing text. Synthetic tests are necessary but not sufficient; packaged proof follows.
7. Prove behavior is enabled only by XML parser identity/path, never filename alone after language load failure and never HTML configuration evidence.

**Test commands/procedures**

```powershell
node --test tests/stackTextEditorXmlTagEditing.test.mjs tests/stackTextEditorXmlStructure.test.mjs
pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/stack-text-editor/launch-modern-editor-xml-tags-a11y.ps1 -Configuration Release
```

Planned packaged feasibility launcher must use same isolated actual-WebView2 mechanism accepted in M04a. Manual procedure types every required construct rather than loading only completed markup; includes undo, IME, screen-reader reading/announcement behavior, and malformed transitions.

**Acceptance criteria**

- Exact XML matching and auto-close matrix passes for declaration, namespaces, CDATA, arbitrary self-closing elements, nested names, and malformed nesting.
- No HTML-derived result is counted.
- Auto-close is one undoable edit and dirty update; passive match never changes content/dirty.
- Parser unavailable/failing/ambiguous means safe plain typing, not blocked input or speculative repair.
- Test-only packaged WebView2 keyboard/IME/AT procedure passes; matching is not color-only, focus stays stable, and announcements do not duplicate typed/inserted text.
- Dependency need and bundle impact are recorded; product manifest remains unchanged.

**No-go/reject criteria**

- Generic HTML auto-close, regex matching, filename-only enabling, fixed void/self-close name list, duplicate IME commit, multi-step undo, broad completion UI, or package addition before RED and explicit approval.

**File ownership / likely paths**

- XML feasibility/QA owner: proposed `tests/stackTextEditorXmlTagEditing.test.mjs`, synthetic fixtures, isolated feasibility extension/helper.
- Integrator owns any temporary dependency experiment and proves it does not enter product manifest/route.
- Production adapter/component/language registry remain unchanged.

**Handoff/exit evidence**

RED/GREEN logs, typed-input transaction traces, XML parser-identity proof, IME/WebView2 manual record, dependency/bundle decision, malformed/failure matrix, reviewer disposition. Missing native composition proof exits `BLOCKED`.

### M05b — Product XML matching and auto-close tags

**Scope/context**

Integrate accepted XML-only matching and auto-close behavior into normal draft editor. No HTML/general completion expansion, lint, formatting, or correctness claim beyond tested structures.

**Dependencies**

- M03b, M05a, and M04b `ACCEPT`.
- Product fold/control regression baseline green.
- Integrator approval for exact package delta, if M03-00/M05a proved one necessary.

**Implementation tasks**

1. Add/adjust RED product integration tests before adapter/language changes.
2. Compose only accepted XML tag extensions when XML language installation succeeds. Keep JSON and all other languages unchanged.
3. Preserve async non-blocking creation and destroyed-view guard. Parser/dependency failure leaves plain editable text and correct dirty state.
4. Implement exact matching/auto-close semantics from M05a, including namespace-qualified names and all negative contexts.
5. Ensure auto-close is one transaction/undo group, one dirty update, and compatible with folded/mapped ranges. Passive matching cannot unfold, mutate, or mark dirty unless accepted design explicitly moves view state only.
6. Apply accepted theme/focus/AT semantics without introducing generic autocomplete popup or new chrome.
7. If approved, direct-declare exact dependency and update lockfile; verify no unrequested completion/lint features enter bundle.

**Test commands/procedures**

```powershell
node --test tests/stackTextEditorXmlTagEditing.test.mjs tests/stackTextEditorXmlStructure.test.mjs tests/stackTextEditorJsonFolding.test.mjs tests/stackTextEditorFoldControlsA11y.test.mjs tests/stackBasicTextEditorUi.test.mjs
npm run check
npm run build
npm run tauri -- build
```

Run typed XML matrix in normal packaged `stack-popup` with keyboard, pointer, IME, NVDA, and Narrator where available. Include parser-load rejection and close/switch during pending load.

**Acceptance criteria**

- Product behavior exactly matches accepted XML corpus and negative contexts.
- Matching is parser-specific and document-neutral; auto-close is one undoable dirty edit.
- Fold mappings/controls remain correct when paired tags are inserted, edited, malformed, undone, and redone.
- Packaged WebView2 keyboard/IME/AT evidence passes; typing remains available through all loader/parser failures.
- Current draft-only/no-save/no-transport/1 MiB/P03 boundaries remain unchanged.

**No-go/reject criteria**

- HTML proof substituted; behavior enabled without XML parser success; malformed document repaired speculatively; IME duplicate; inaccessible indication; broad completion; loader blocks editor; save/persistence/projection added.

**File ownership / likely paths**

- Frontend editor owner: `stackTextEditorAdapter.ts`, and `stackTextEditorLanguages.ts` only for M03-00-approved XML composition.
- QA/accessibility: XML tag tests plus existing focused contracts.
- Integrator: manifest/lockfile only if approved. `StackPopupSurface.svelte`, Rust, capabilities, and master Quick Edit plan remain outside expected scope.

**Handoff/exit evidence**

RED/GREEN logs, dependency/bundle delta, normal-route packaged typed-input/IME/AT matrix, failure/staleness evidence, fold regression, reviewer dispositions, and explicit statement that XML syntax/structured editing is not general XML validation.

### M06 — Combined packaged qualification and bounded handoff

**Scope/context**

Qualification-only gate for all four requested rows on current resident draft route. No new behavior should be added here; failures return to owning phase.

**Dependencies**

- M04b and M05b each have accepted complete evidence packets.
- Stable combined source revision and packaged release artifact.
- QA and accessibility reviewers assigned.

**Implementation tasks**

1. Freeze source/dependency/fixture hashes; rerun complete automated matrix without weakening assertions.
2. Package once from stable combined revision; run normal `stack-popup` manual matrix on that exact artifact.
3. Test files near current cap plus small files; valid/malformed transitions; repeated fold/tag operations; language load rejection/delay; rapid close/switch/destroy; font/theme changes; undo/redo; dirty close Cancel/Discard.
4. Run keyboard-only, focus, NVDA, Narrator, IME, forced-colors/high contrast, 100/150/200% DPI, zoom, and narrow layout procedures for fold controls and matching indication.
5. Confirm unsupported/plain languages have no new folds/tag behavior and no generic completion.
6. Audit diff for forbidden save/write/invoke/localStorage/global content transport/Rust/capability/P03 changes.
7. Record limited claim language. Passing permits only: JSON/XML folding, accessible fold controls, and XML-specific matching/auto-close on current 1 MiB draft-only route. It cannot claim XML validation, save safety, huge files, P03/P04 progress, or broad product readiness.

**Test commands/procedures**

```powershell
node --test tests/stackTextEditorJsonFolding.test.mjs tests/stackTextEditorXmlStructure.test.mjs tests/stackTextEditorFoldControlsA11y.test.mjs tests/stackTextEditorXmlTagEditing.test.mjs tests/stackBasicTextEditorUi.test.mjs tests/stackBasicTextFileContract.test.mjs
npm run check
npm run build
npm run tauri -- build
node --test tests/changelogPolicyHygiene.test.mjs
git diff --check
```

Manual matrix uses exact packaged artifact and synthetic fixtures. For each row record operator, OS/WebView2/AT/IME versions, steps, expected spoken/visual/document state, observed result, and artifact path. Source tests or successful packaging cannot replace this procedure.

**Acceptance criteria**

- All automated behavior and regression tests pass on frozen revision.
- All four requested feature rows pass normal-route packaged WebView2 procedures, including keyboard/focus/screen-reader label/state and XML-specific IME/tag behavior.
- Language/parser failure never blocks typing; malformed documents remain editable; dirty guard and draft notice remain correct.
- Fold state maps through edits, never uses stale offsets, and safely reopens on invalid structure.
- Diff audit confirms no save/persistence/global content transport/Rust/capability/projection scope leakage.
- QA, accessibility, and integrator reviewers each record `ACCEPT`; documentation updates report only bounded claim.

**No-go/reject criteria**

- Any missing/manual-not-run row, source-string-only claim, ordinary browser standing in for packaged WebView2, unexplained failure, accessibility/IME defect, stale fold state, parser-blocked typing, scope leakage, or attempted P03/P04/readiness promotion.

**File ownership / likely paths**

- QA owner: focused test suite and evidence packet.
- Accessibility owner: manual packaged matrix.
- Integrator: stable package, diff audit, final bounded docs/changelog update.
- Product code changes are rejected in this phase and return to M04b or M05b.

**Handoff/exit evidence**

One combined evidence index linking every phase packet, exact artifact/source/package hashes, command outputs, packaged manual results, reviewers, unresolved risks, and bounded claim text. Any failed or unavailable required proof exits `BLOCKED`, with current product truth unchanged.

## 13. Global reject and rollback rules

- Preserve failing fixtures/traces. Never make a gate pass by deleting malformed cases, weakening label/state checks, raising document cap, switching XML tests to HTML, or converting runtime assertions into source-string checks.
- A phase failure disables only descendant work. Existing draft editor and syntax highlighting remain current baseline.
- Product integration regression: remove/revert only new fold/tag composition and retain async syntax, editor availability, dirty guard, and no-save notice.
- Package/API mismatch: return to M03-00; do not guess replacement imports or float versions.
- Accessibility failure: fold/tag UI remains deferred. Do not ship pointer-only controls or hide inaccessible controls behind undocumented shortcuts.
- Performance or parser stall: keep editor usable without feature; no synchronous parser gate before first edit.
- P03 remains blocked regardless of these results. No evidence from resident <=1 MiB documents transfers to bounded projection or large-file claims.

## 14. Current-document validation and source traceability

This planning change itself requires no build. Required checks:

```powershell
node --test tests/changelogPolicyHygiene.test.mjs
node --test tests/stackBasicTextEditorUi.test.mjs
git diff --check
```

Focused editor test is applicable only as confirmation that current source still excludes production folds/tags; future M04b/M05b must deliberately replace obsolete exclusion assertions RED-first.

Source anchors:

- Current product boundary: `master_spec.md` Stack Quick View/Edit boundary.
- Current shell/editor lifecycle: `src/components/StackTextEditor.svelte`.
- Current production `EditorView`: `src/features/stack-browser/stackTextEditorAdapter.ts`.
- Current async language registry: `src/features/stack-browser/stackTextEditorLanguages.ts`.
- Current extension allowlist/dirty guard: `src/components/StackPopupSurface.svelte`.
- Current dependency pins: `package.json` and lockfile.
- Current source contracts: `tests/stackBasicTextEditorUi.test.mjs`, `tests/stackBasicTextFileContract.test.mjs`.
- Projection/save authority and blocked gates: `docs/stack-browser-quick-view-editor-implementation-plan.md`.
- Changelog protocol: `CHANGELOG_POLICY.md`.
