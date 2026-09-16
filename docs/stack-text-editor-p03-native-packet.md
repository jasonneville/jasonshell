# P03 native evidence packet template

Packet status: **BLOCKED — unsigned template**

| Field | Recorded value |
|---|---|
| Run ID / absolute evidence directory | BLOCK — required |
| Git commit + dirty diff disposition | BLOCK — required |
| UTC start/end | BLOCK — required |
| Windows edition/build | BLOCK — required |
| WebView2 runtime version | BLOCK — required |
| Host binary/build profile | `stack_text_p03` debug test-only example; confirm |
| IME name/version/language/profile | BLOCK — required |
| NVDA version/settings | BLOCK — required |
| Narrator version/settings | BLOCK — required |
| Screen recording operator | BLOCK — required |

## T03-04 observation

- Candidate UI visible while focused-editor **Ctrl+Shift+L** triggers adjacent lease arrival: BLOCK
- `compositionStart` event line: BLOCK
- `adjacentLeaseArrived` line with `composing:true`: BLOCK
- `compositionEndPendingInput` event line / redacted committed-text metadata: BLOCK
- Exactly one following trusted `inputCommit`: BLOCK
- Focused-editor Ctrl+Shift+L `beforeFocused:true`, `afterFocused:true`, `beforeViewComposing:true`, and `afterViewComposing:true`: BLOCK
- Cancel line / zero commit delta: BLOCK
- Blur-cancel line / zero commit delta: BLOCK
- Ctrl+Z/Ctrl+Y one-unit round trip: BLOCK
- Caret/candidate stability: BLOCK
- Result: **BLOCK**
- Input/engine reviewer name, signature, UTC: BLOCK
- Privacy reviewer confirms synthetic non-sensitive input, redacted event text, cropped recording; name, signature, UTC: BLOCK

## T03-07 observation

- Mounted units / newline state: BLOCK
- Combining-grapheme physical caret observations: BLOCK
- RTL visual/logical caret observations: BLOCK
- Selection bounds / copied UTF-16 units: BLOCK
- `t03-07-copy.txt` SHA-256 / exact oracle comparison: BLOCK
- Result: **BLOCK**
- Engine reviewer name, signature, UTC: BLOCK

## T03-08 observation

- NVDA announced role/name/editability: BLOCK
- NVDA exact cross-lease and giant-line speech/copy observations: BLOCK
- Narrator announced role/name/editability: BLOCK
- Narrator exact cross-lease and giant-line speech/copy observations: BLOCK
- No skip/duplicate/newline/focus/selection defect: BLOCK
- Result: **BLOCK**
- Named listening accessibility reviewer, signature, UTC: BLOCK

## Artifact manifest

List every file, byte size, SHA-256, producer, and relation to test. Required: `p03-events.ndjson`, `p03-summary.json`, three test recordings/captures, speech transcripts, copy oracles, and this signed review.

Recording operator and privacy reviewer must inspect event log plus every recording before signature. Record crop/notification controls, redaction findings, rejected/deleted captures, final artifact hashes, reviewer names, signatures, and UTC times. Raw sensitive text is forbidden.

## Final disposition

- P03 result: **BLOCK** until all three rows PASS and both named reviewer roles sign.
- Manual execution performed: **No** (template default; operator must change truthfully).
- Exceptions/residual risks: BLOCK — required.
