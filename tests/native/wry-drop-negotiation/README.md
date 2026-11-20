# Wry OLE drop negotiation fixture

On Windows, from the repository root:

```powershell
cargo test --offline --manifest-path tests/native/wry-drop-negotiation/Cargo.toml --target-dir node_modules/.cache/wry-drop-negotiation
node tests/browser/stack-drag/run.mjs
```

The Rust fixture calls the production Wry `IDropTarget` via COM, using hidden test HWNDs and real Shell data objects. It does not launch Tauri, call `SHDoDragDrop`, register a live target, or move the cursor. The renderer E2E separately exercises production UI with mocked native IPC.

Initial RED: 1 passed, 3 failed. For an exact local object without `CF_HDROP`, DragOver reports NONE despite COPY being offered. An unsupported external object leaves COPY untouched at DragEnter instead of reporting NONE. A manually invoked local Drop captures a position but leaves a synthetic COPY|MOVE offer unchanged. Production `SHDoDragDrop` offers COPY only, so the mixed-mask case tests callback output explicitly; it does not establish the live cursor cause.

Additional checks verify that MOVE-only offers do not capture local receipts; cached COPY clears after Leave and Drop; and a distinct Shell COM object representing the exact same files still emits an ordinary incoming COPY event and no local receipt while the source observation is active.

The same-parent control mirrors production's absolute parent and relative child PIDLs for two temporary files. `GetData(CF_HDROP)` succeeds on this machine. Preserve external incoming `CF_HDROP` copy and outgoing `SHDoDragDrop` COPY-only behavior; only the exact local COM identity may yield a local receipt. Full WebView2 delivery, actual drag-loop negotiation, and cursor behavior remain unverified.
