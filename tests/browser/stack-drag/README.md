# Stack Browser native-drop routing fixture

## Pointer-owned regression

The updated runner no longer injects an OLE local receipt. At 150% scale it sends trusted CDP mouse moves and release onto the live folder row. The mock contract is `prepare_stack_local_drop({ paths }) -> { requestId }`, then `commit_stack_local_drop({ requestId, destination })` after a valid release. Staying inside must not invoke `start_stack_file_drag`; leaving the viewport hands off once, and a later release over the folder cannot move locally.

Historical pointer RED: `node tests/browser/stack-drag/run.mjs` returned 2 passes and 9 failures with the earlier CDP driver. Both incoming copy controls passed. The old renderer invoked OLE before exit and made no commit or clipboard mutation without a local receipt. The old renderer was not rerun with the corrected driver, so this is not a matched RED/GREEN comparison or proof of a Windows COM failure.

GREEN: `node tests/browser/stack-drag/run.mjs` passed 11/11 with held CDP moves reporting `button: 'left'` and idle moves reporting `'none'`. Using `'none'` for held moves produced a misleading 6/11 result with lost pointer capture. With the correction, marquee release issued one commit, no OLE or clipboard calls, and left both files exactly once in the in-memory destination, absent from the source. Native shell behavior was not exercised.

The notes below describe the previous, receipt-injected fixture and its historical evidence.

Run from the repository root with Node 24 and Chromium:

```powershell
node tests/browser/stack-drag/run.mjs
```

Set `CHROME_PATH` if neither the default Windows Chrome path nor `/usr/bin/chromium` exists. The runner owns a loopback Vite server and an isolated headless profile. It mounts the production Stack Browser and CSS. CDP sends trusted mouse input for pointer marquee and drag initiation. The mocked native response supplies a request ID and local receipt position at 1.5× physical client coordinates. A separate untagged event models external delivery; the mock intentionally does not emit that event for a native-authenticated local receipt. File state is in-memory. The runner never invokes Tauri, touches real files, or changes Windows shell state.

The move case expects exactly one `commit_stack_local_drop` to `Dest` with the issued request ID and no clipboard commands. Both selected files disappear from the source listing and appear exactly once in the in-memory destination. Other cases verify that background, file-row and self targets and cancellation do not mutate; an untagged incoming drop copies even if its paths match the active drag; and a native outbound copy receipt without a local position leaves local state unchanged. This does not certify COM identity, WebView2 delivery, actual filesystem renames, or outbound copy effects.

Pre-fix RED: the runner exited 1. The folder case copied and pasted twice into the source folder. The three invalid own-drop cases each copied and pasted once into the source folder. Incoming copy and outbound-only controls passed.
