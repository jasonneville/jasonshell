# Stack text editor M03–M06 evidence

Date: 2026-09-16. Scope: resident <=1 MiB UTF-8 memory-only draft route.

## M03-00 API freeze — ACCEPT

Installed/lock-resolved versions: `@codemirror/language@6.12.4`, `@codemirror/lang-json@6.0.2`, `@codemirror/lang-xml@6.1.0`, `@codemirror/state@6.7.4`, `@codemirror/view@6.43.11`, `@codemirror/commands@6.11.0`. `@codemirror/autocomplete` is transitive-only and is not imported. No manifest/lock delta.

Exact installed declarations establish `foldService`, `foldNodeProp`, `foldInside`, `foldable`, `foldEffect`, `unfoldEffect`, `foldState`, `foldedRanges`, `foldCode`, `unfoldCode`, `toggleFold`, `foldKeymap`, `codeFolding`, `foldGutter`, `syntaxTree`, and `ensureSyntaxTree` in `@codemirror/language`. `@codemirror/lang-xml` exports `xml`, `xmlLanguage`, and `autoCloseTags`; `xml()` defaults `autoCloseTags` true. Installed XML source configures parser-derived element folds and tag-name bracket matching. Context7 quota failure is not evidence.

Decision: use native fold state/effects so CodeMirror maps ranges through transactions. Install `codeFolding`, gutter, and keymap only after successful async JSON/XML language resolution. Rejection/pending/destroy leaves no fold controls. Keep XML package default autoclose and existing parser bracket matching; add no completion UI/import. Parser ambiguity means no fabricated app-owned structure.

Harness: Node state/parser behavior tests plus isolated synthetic fixture `scripts/stack-text-editor/modern-editor-fixture.{html,js}`. Node may import or inspect fixture source, but `typeof document !== 'undefined'` guards its DOM path, so Node does not execute browser behavior. Fixture has no IPC and is unreachable from product routing. Existing `src-tauri/examples/stack_text_probe.rs` is packaged-probe precedent; no Rust/capability mutation authorized.

## M03–M05 automated evidence

- `tests/stackTextEditorJsonFolding.test.mjs`: nested/empty/scalar/escaped/malformed JSON, mapped ranges, document-neutral folds, editable fallback.
- `tests/stackTextEditorXmlStructure.test.mjs`: declaration, namespaces/attrs, CDATA, comments, PI, arbitrary self-close, same-name nesting, entities, malformed XML, mapped edits.
- `tests/stackTextEditorFoldControlsA11y.test.mjs`: source contracts require click-time focus scheduling, same-range replacement matching, and declarations in the isolated no-IPC browser fixture. Test inspects source only; it does not execute fixture DOM, keyboard, fold, or focus behavior.
- `tests/stackTextEditorXmlTagEditing.test.mjs`: parser/package behavior covers namespaces/attributes, closing-tag completion, and exclusion contexts. It also exposes the pinned package's two-step undo defect; the production adapter uses an XML-only atomic input handler, while the browser fixture must prove its real `onChange` and undo behavior. No generic completion.

## Required packaged procedures — BLOCKED / unperformed

Build one Release artifact from frozen revision. In isolated fixture, then normal `stack-popup`: use keyboard only to collapse/expand nested JSON/XML; verify each control name/state and focus with NVDA and Narrator; pointer parity; Tab/Shift+Tab and Escape ownership; compose non-Latin IME before/inside/after fold boundaries. Type XML start tags with namespace/attrs and verify one closing insertion, one undo, one dirty transition; verify no insertion for declaration, PI, comment, CDATA, closing/self-closing/malformed tags; exercise paste, selection, backspace, undo/redo. Record OS, WebView2, AT/IME versions, artifact SHA-256, spoken output, screenshots, operator, and result.

M06 matrix on same artifact: small and near-cap files; valid/malformed transitions; language load reject/delay; rapid close/switch/destroy; fonts/theme; undo/redo/dirty Cancel/Discard; keyboard/focus/NVDA/Narrator/IME; high contrast; 100/150/200% DPI; zoom; narrow layout; unsupported TXT/CSV/LOG absence. Audit no save/write IPC, persistence/storage, global content events, recovery/projection, Rust/capability, generic completion, or P03/P04 claim.

**BLOCKED:** ordinary-browser fixture mount was attempted but timed out, leaving DOM-path behavior unproved. Packaged WebView2/NVDA/Narrator/IME runs remain unperformed. Node source/state tests do not replace browser or packaged/manual evidence. Procedures remain required; no M04a/M04b/M05a/M05b/M06 runtime accessibility acceptance or readiness claim.
