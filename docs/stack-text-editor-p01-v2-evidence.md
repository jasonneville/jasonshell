# P01 v2 Contracts and Evidence

## Scope

Current checkout implements `stack-text-editor.v2`, approved by the user in this
session. Earlier P01/P02 PASS narratives in the implementation plan describe
historical work, not artifacts present at starting commit `322418f`.
This record governs the current P01 implementation. No P02/P03 promotion follows.

The Rust module is test/debug-only; TypeScript contracts have no production
imports. No editor IPC commands, document actor, file provider, production UI,
save engine, recovery store, or unrestricted test-provider selector is registered.
The standalone Tauri example does not load shell hooks, AppBars, or user files.

## Contract

- Rust `src-tauri/src/stack_popup/text_document/protocol.rs` and TypeScript
  `src/features/stack-browser/textEditorProtocol.ts` define exact request/result
  variants, errors, session revisions, leases, selections, jobs and barriers.
- All global byte positions, revisions and sizes use canonical decimal u64
  strings. Local positions are bounded UTF-16 code-unit integers. Surrogate and
  original CRLF interiors are not editable boundaries. Original newline widths
  remain explicit while the projection uses one LF unit. BOM bytes are preserved
  outside the small oracle's editable range.
- Lease endpoints resolve through authoritative lease ID and view generation.
  Combined selections may span leases but must share session, source generation
  and document revision. Both endpoints retain affinity and direction; expired
  or invalidated handles cannot silently revive. A handle expires after its
  recorded revision horizon (initially 256 revisions, saturating at u64 maximum).
- Arithmetic remapping is not input validation. The future document owner must
  validate edit/endpoints against authoritative encoding/CRLF boundaries first.
  Continued grapheme context is explicit and cannot certify a complete boundary;
  this does not prove bounded renderer continuation or IME feasibility.
- Snapshot barriers name accepted revision, visible input sequence and operation
  IDs. The future actor must compare against its authoritative input stream,
  reject omissions, and freeze the snapshot only after preceding input drains.
  Later input belongs to the next snapshot, never to a partially completed save.
- Replay compares recipient-computed SHA-256 of canonical ASCII JSON: lexical
  object keys, preserved array order, lowercase UTF-16 escapes for non-ASCII.
  Shared canonical bytes include a non-BMP insertion and values above 2^53.
  Caller-provided digests are never authority. Duplicate IDs preserve immutable
  pending/accepted/rejected receipts; changed payloads reject. Retirement requires
  durable acceptance, and retired IDs cannot reapply. Bounded tombstone exhaustion
  refuses admission; production persistence/retirement reconciliation is later work.
- Job and error schemas distinguish cancellation before publication, already
  published state and ambiguous publication. `PublicationAmbiguous` and ambiguous
  disposition imply each other and never permit automatic replay.
- Clipboard denial cannot cut. Intervening edit or undo invalidates an awaited
  clipboard cut, including an edit/undo ABA cycle. Bulk insertion uses an explicit
  spool variant, not truncation at the inline-input cap.
- Future command handlers must derive caller identity from Tauri's window, permit
  only the authorized Stack Browser owner, and resolve handles in that owner and
  source generation. Payload fields never confer authority. Close/reload must
  invalidate outstanding generations, handles, jobs and response credits.

## Frozen Budgets

These are admission hypotheses, not measured production guarantees. Do not raise
them to make a later experiment pass.

| Resource | Initial limit / policy |
| --- | --- |
| First source read / subsequent block | 64 KiB / 256 KiB |
| Projection | 131072 UTF-16 units; 2048 segments; 4096 total rows |
| Boundary entries | At most projection units plus segments |
| Reads and returned payloads | 2 active reads; 4 combined active/held credits |
| Pending seek | One latest demand, replacing obsolete queued demand |
| Local speculative input | 1 MiB UTF-16 storage; 64 operations; one in flight |
| Oversized input | Explicit spool path; refuse admission without dropping text |
| Handles | 8 leases; 64 selections; explicit release/expiry |
| Jobs / events / result page | 32 / 256 / 256 |
| Operation receipts / tombstones | 1024 / 1024; refuse when full |
| Hot source / index memory | 32 MiB / 16 MiB |
| Spool | 64 MiB per session; 256 MiB globally; no dirty-session eviction |
| Scheduling | Interactive edits/visible reads, then demand, then background FIFO |
| Disk admission | Checked source + add-store + staging + backup + overhead sum |

Transport is bounded request/response or owner-specific channels, never broadcast
source content. Cancellation/release return credits; close invalidates queued work.
The harness models these boundaries; it does not implement disk reservations,
preemption of blocking OS operations, or the production scheduler.

## Dependencies

Exact development pins in `package-lock.json`: `codemirror` 6.0.2,
`@codemirror/state` 6.7.4, `view` 6.43.11, `commands` 6.11.0,
`language` 6.12.4 and `search` 6.7.2. These are MIT-licensed packages.
Only the isolated experiment imports them. Production frontend build remains
free of CodeMirror imports. Rust uses existing locked serde/Tauri dependencies;
no crypto implementation or new Rust dependency was introduced.
Later recovery dependency candidates remain the plan's pins, not P01-proven crypto.

Fresh npm audit reports four high and one moderate existing findings in devalue,
nanoid, postcss, svelte and vite; no CodeMirror advisory was reported. This is not
a clean repository security audit or an acceptance of those unrelated risks.
The packaged probe embeds local lazy chunks and a local module worker, with CSP
byte-equal to production. No remote script, CDN import or CSP relaxation is used.

## Reproduction

Run commands from repository root. Evidence output directories must be new and
under `test-results`; generated content is synthetic. Never publish earlier
diagnostic screenshots, which may contain unrelated desktop pixels.

```powershell
npx tsc -p tsconfig.test.json
node --experimental-test-coverage --test tests/stackTextEditorProtocol.test.mjs tests/stackTextEditorResults.test.mjs tests/stackTextEditorHarness.test.mjs tests/stackTextEditorAdmission.test.mjs tests/stackTextEditorFingerprint.test.mjs
cargo test --manifest-path src-tauri/Cargo.toml text_document
npx vite build --config scripts/stack-text-editor/vite.config.mjs
cargo test --manifest-path src-tauri/Cargo.toml --example stack_text_probe --features tauri/custom-protocol
cargo build --manifest-path src-tauri/Cargo.toml --example stack_text_probe --features tauri/custom-protocol
node scripts/stack-text-editor/run-corpus.mjs test-results/stack-text-editor/P01/NEW-corpus
pwsh -NoProfile -File scripts/stack-text-editor/run-native.ps1 -OutputDirectory C:\dev\jasonshell\test-results\stack-text-editor\P01\NEW-native
```

The probe config lives in its own directory because Tauri config discovery uses
the directory, not an arbitrary alternative filename. Windows examples need the
Common Controls v6 manifest in `build.rs`; these linker arguments affect examples
only, not the production binary. The current repository has one such example.

## Verified Evidence

- Focused Node: 29/29 pass. Executed Node coverage: protocol 93.84% lines,
  80.42% branches; harness 97.14% lines, 87.38% branches. This is not Rust or
  whole-application coverage. Several review regressions have observed RED/GREEN;
  no claim that every initial test was run RED before implementation.
- Rust focused filter: 13 pass (12 protocol tests plus one existing name match).
  Full Rust run before the last canonical-vector addition: 649 pass, 3 existing
  ignored. Native metadata validation test: 1 pass.
- Production `npm run build` and `npm run check` pass; three existing unused
  StackGitPanel CSS warnings remain.
- Full Node baseline independently reproduced from clean archived HEAD
  `322418fed4655bf039ae0b37ebdf1e270cc08ba7`: 875 tests, 854 pass, 18 fail,
  3 todo. All 18 failing names match the current full-suite run. No unrelated
  test/source fixes were folded into P01.
- `test-results/stack-text-editor/P01/v2-corpus-02/manifest.json`: eight seeded
  fixtures; 128 MiB giant fixture; maximum chunk 65536 bytes; independent streamed
  byte-count/SHA-256 checks agree. Per-chunk memory peaks are observed samples,
  explicitly not transient-peak or production-memory proof.
- `test-results/stack-text-editor/P01/v2-native-09/`: packaged native editor PID
  5340 and control PID 18860 exited zero. DPI-aware foreground client captures
  contain only synthetic probe content. The editor visibly contains inserted
  `x` while ACK is pending and EOF withheld; control reports deliberate 80ms
  stall. Five artifact hashes independently recomputed and matched.
- Native process sampling checks PID plus creation identity and descendant
  ancestry, accounting for CIM microsecond precision versus FILETIME 100ns.
  Renderer and native-host samples remain separate; synthetic prefix is 128 bytes.
- Independent code and artifact QA found no remaining P01 blockers after BOM,
  clipboard ABA, ambiguity, metadata, PID attribution and capture privacy fixes.
- `test-results/stack-text-editor/P01/v2-native-10/`: editor PID 5860 and control
  PID 37252 exited zero with child-only WebView2 arguments
  `--proxy-server=http://127.0.0.1:9 --proxy-bypass-list=<-loopback>` observed in
  descendant command lines. Local worker startup, edited content and all five
  artifact hashes passed independent review. A separate connection check to the
  proxy endpoint returned `ConnectionRefused`. This is controlled dead-proxy
  evidence, not a physically disconnected-machine or OS-firewall test.

## Evidence Limits

RAF marks are callback timestamps, not compositor presentation measurements.
Screenshots prove visible state, not precise native input-to-pixel latency. The
native edit is programmatic, not IME/keyboard/assistive-technology acceptance.
ACK is deliberately delayed eight seconds; durability, EOF and index completion
are deliberately unobserved, never assigned fabricated completion timestamps.
Failure injection is a test model, not a real disk-full or crash-recovery proof.
Native process observations do not establish production memory or file-size
scaling budgets. The worker and chunks are packaged local assets. Their startup
passed under the documented child-only dead-proxy configuration. The special
`<-loopback>` token removes Chromium's implicit loopback proxy bypass; it does
not disable OS loopback networking. No claim is made about every possible network
operation, physical disconnection, or OS-firewall enforcement. P01 local-asset
startup and independent documentation/evidence review are accepted; later-phase
gates remain closed pending their own implementation and evidence.
