# P03 native manual procedure

Status: **BLOCKED — runnable procedure; no named native/human session has run.** Automated checks cannot PASS T03-04, mounted T03-07, or T03-08.

## Launch and evidence root

From repository root on Windows with WebView2 and Rust MSVC available:

```powershell
npm run p03:experiment -- 20260916-p03-native-01
```

This creates a new, non-overwriting `test-results/stack-text-editor/P03/20260916-p03-native-01/`, builds only `p03-experiment.html`, then runs isolated Rust example `stack_text_p03` with mandatory `--p03-experiment` guard. Product routes, IPC registry, providers, actors, capabilities, and UI remain untouched. Keep host open through all tests. Start screen recording separately and save it in run directory.

Host writes `p03-events.ndjson`; **Save evidence summary** creates `p03-summary.json` once. Save recordings, AT speech viewer captures, screenshots, copied oracle text, and completed `review.md` beside them. Never replace existing run directory/files.

## T03-04 — genuine WebView2 IME during adjacent arrival

1. Select **Mount IME boundary fixture**. Editor receives focus at final grapheme `境`.
2. Activate named non-Latin Windows IME (Microsoft Japanese IME recommended). Type genuine phonetic input and keep candidate window open. Do not use DevTools or synthetic events.
3. While candidate window remains open and editor retains focus, press **Ctrl+Shift+L**. Do not click controls. Event trace must contain `compositionStart`, then `adjacentLeaseArrived` with `"composing":true`, `"beforeFocused":true`, `"afterFocused":true`, `"beforeViewComposing":true`, and `"afterViewComposing":true`. Any blur or false composition field = BLOCK.
4. Select candidate and commit normally. Verify trace contains one `compositionEndPendingInput` followed by one trusted `inputCommit`, candidate appears once, caret remains sensible, and counter increases by exactly one only after that trusted input commit. Use native Ctrl+Z/Ctrl+Y; verify one-step round trip.
5. Repeat, cancel candidate with Escape, then repeat and blur without commit. Trace must distinguish `compositionCancel` and `compositionBlurCancel`; input commit counter must remain unchanged for both.
6. Activate **Save evidence summary** only after all scenarios. Record committed text only in separately encrypted/reviewer-controlled notes when required; harness event log records only bounded unit counts/redacted markers. Failure/cancellation, Latin fallback, synthetic input, more than one candidate commit, or absent candidate UI = BLOCK.

Expected evidence: `p03-events.ndjson`, `p03-summary.json`, `t03-04-ime.mp4`, `t03-04-final.png`, completed signed T03-04 fields in `review.md`.

## Privacy and recording procedure

- Use only explicit synthetic, non-sensitive fixture/input text created for this run. Never type credentials, personal data, source text, clipboard contents, or customer data.
- Harness event metadata redacts composition/input text to bounded unit counts. Reviewer must inspect `p03-events.ndjson` before signing and confirm no entered text appears.
- Recording operator must crop to harness window, disable notifications/overlays, review recording frame-by-frame for unrelated/sensitive content, then record SHA-256 in manifest.
- Named privacy reviewer must sign UTC attestation in `review.md` only after log/recording redaction review. If redaction cannot be verified, delete unsafe capture, record deletion, rerun with new non-overwriting run ID; never include unsafe artifact in packet.

## T03-07 — mounted 20,001-unit combining grapheme and RTL caret/copy

1. Select **Mount 20,001 combining + RTL fixture**. Status must report `20013` UTF-16 units and no newline; first grapheme is `a` plus exactly 20,000 U+0301 marks.
2. Use physical Left/Right, Ctrl+Left/Right, Shift variants, Home/End through combining and `אבג XYZ גבא`. Record caret/selection behavior and bidi visual/logical transitions; do not infer success from unit tests.
3. Select first grapheme using native pointer/keyboard behavior, activate **Copy editor selection**, and save clipboard as UTF-8 `t03-07-copy.txt`. Expected UTF-16 selection length: 20,001; no newline. Record SHA-256 and observed caret behavior.
4. Any split/corrupt copy, invented newline, unreachable caret position, crash/hang, or reviewer-determined bidi mismatch = BLOCK.

Expected evidence: event `fixtureMounted` with `units:20013`, `selectionCopied` with actual bounds/units, `t03-07.mp4`, `t03-07-copy.txt`, signed T03-07 fields.

## T03-08 — named NVDA and Narrator sessions

Run both ATs separately with speech viewer/transcript capture. For cross-lease test, mount IME fixture, keep editor focused, then press **Ctrl+Shift+L**; no composition is needed. Verify the `adjacentLeaseArrived` event records both `beforeViewComposing` and `afterViewComposing` as false. For continued giant-line test mount giant fixture.

1. Focus editor by Tab. Record announced role/name: `P03 bounded projected lease editor` and editable semantics.
2. With NVDA, navigate character/word/line across `境界` lease boundary; extend selection across boundary, activate **Copy editor selection**, page/focus away and back, then continue. Repeat over combining/RTL fixture.
3. Repeat exact workflow with Narrator. Compare speech/copy to visible bounded source. No hidden full-document accessibility mirror exists.
4. Record spoken sequence and copied text. Skip, duplicate, synthetic newline, selection loss, focus reset, unannounced editability, or inability to traverse boundary = BLOCK. DOM inspection alone cannot PASS.

Expected evidence: `t03-08-nvda.mp4`, `t03-08-nvda-speech.txt`, `t03-08-narrator.mp4`, `t03-08-narrator-speech.txt`, copy files, AT versions/settings, and named accessibility reviewer signature.

## Required `review.md`

Copy [`stack-text-editor-p03-native-packet.md`](stack-text-editor-p03-native-packet.md) into run directory as `review.md`, fill every field, list artifact hashes, and sign. Keep result BLOCK until all observations and named signatures exist. Automated PASS is not manual PASS.
