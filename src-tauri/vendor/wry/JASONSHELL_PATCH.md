# JasonShell Wry source provenance

## Base

- Published crate: `wry 0.54.4`, copied from the installed crates.io package.
- Registry archive SHA-256 from the pre-patch application lockfile:
  `e5a8135d8676225e5744de000d4dff5a082501bf7db6a1c1495034f8c314edbc`.
- Source: https://crates.io/crates/wry/0.54.4
- Original source, build script, manifests, examples, documentation and license
  files are retained. Registry-local `.cargo`, `.cargo-ok` and the dependency's
  own `Cargo.lock` are omitted; the application's lockfile remains authoritative.
- SPDX: `Apache-2.0 OR MIT`. Preserve `LICENSE-APACHE`, `LICENSE-MIT`,
  `LICENSE.spdx`, upstream copyright and source notices when redistributing.

## Applied bounded backport

Applied after the runner-owned actual-source facade reproduced both teardown
cases reentering the still-registered callback during controller release.

- Official upstream fix: PR 1795, first released in Wry 0.56.1.
- Commit: `3fbf592feab29ffb269778a6f5573746a2f25a4a`.
- Patch: https://github.com/tauri-apps/wry/commit/3fbf592feab29ffb269778a6f5573746a2f25a4a.patch
- Changelog: https://raw.githubusercontent.com/tauri-apps/wry/dev/CHANGELOG.md
- Target: `src/webview2/mod.rs`, `InnerWebView::parent_subclass_proc` teardown arm.
- Applied diff: replace the non-null check / boxed controller drop / subclass
  reset-to-null sequence with `RemoveWindowSubclass` followed by the boxed
  controller drop. Removing dispatch authority before COM release prevents
  synchronous release reentry through stale or cleared controller data.
- Behavioral deviations from the supplied official upstream patch: none. This
  backports only the teardown arm to 0.54.4; it adds no broad null guards, changes
  no other callback arms, and does not upgrade Wry/Tauri/runtime versions.
- This teardown backport modifies `src/webview2/mod.rs`. Original licenses and
  copyright notices are intact.

The application uses `tauri-runtime-wry 2.10.1`, whose Wry requirement is
`^0.54.0`; Wry 0.56.1 is outside that range. This local patch avoids a broad
Tauri/runtime upgrade.

The observed native crash address reaches a shared COM-call wrapper; matching
binary/PDB evidence supports this teardown defect but does not independently
establish the crash-time caller/register values. Safe regression evidence does
not replace consent-gated native capture/cancel/reentry E2E.

## JasonShell local file-drop observer

`src/lib.rs`, `src/webview2/mod.rs`, and `src/webview2/drag_drop.rs` expose a Windows-only scoped observer for the existing OLE drop target. A target descended from the owner HWND can negotiate COPY for the exact canonical COM identity registered for the synchronous native drag on that STA, without requiring `CF_HDROP`. Drop repeats the identity check and requires both a source-allowed COPY effect and valid physical child-client coordinates before recording one point. Local drops do not emit generic file-drop events. External file drops still require `CF_HDROP` and negotiate only source-allowed COPY; outbound OLE remains copy-only. This observer authenticates the native OLE boundary; it is not a claim that the Chromium local-pointer fixture proves COM identity, WebView2 delivery, or filesystem behavior. End-to-end same-STA delivery still needs consent-gated Windows acceptance.
