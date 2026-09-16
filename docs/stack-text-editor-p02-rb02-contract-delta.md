# P02 RB-02 v1 → Canonical v2 Contract-Delta Packet

**Prepared:** 2026-09-15  
**Owner role:** P02 contract/storage implementation owner  
**Status:** **P02 Accepted — canonical-v2 non-scale storage exit only.** Coordinator accepted RB-02/P02 after technical review PASS, named independent storage/safety ACCEPT, and named independent security/privacy ACCEPT against fresh packet `P02/20260916-p02-rb02-nonscale-05`. This authorizes P04 entry only; P04 recovery-root/saving/recovery security and P12/NFR-6 populated scale debt remain open. No P03, product-integration, recovery, huge-file-readiness, or overall product PASS follows.

## Boundary and decision rule

Canonical future contract is `stack-text-editor.v2` in
`src-tauri/src/stack_popup/text_document/protocol.rs` and
`src/features/stack-browser/textEditorProtocol.ts`. P02 remains isolated,
test/debug-only `stack-text-editor.v1` plus
`stack-text-editor.p02-feasibility.v2` in
`src-tauri/src/stack_popup/text_document/feasibility/contract.rs`,
`scripts/stack-text-editor/p02-run.mjs`, and
`tests/fixtures/stack-text-editor-protocol.json`. No adapter, schema relabel, or
production route exists or is authorized.

Disposition meanings match RB-02. `RERUN AFFECTED` requires a new run directory
and manifest. `BLOCK` means required provenance, artifacts, or an executable
compatibility check is unavailable. No row below qualifies for `REUSE ISOLATED`.

## Provenance

### Original recorded runs

| Run ID | Role | Recorded result | Provenance limit |
|---|---|---|---|
| `20260909-validation-parent-01` | Historical mixed-size P02 lineage run cited by prior records | Prior record says 20 PASS; multi-GiB and over-RAM BLOCK; matrix status BLOCK | `test-results/stack-text-editor/P02/20260909-validation-parent-01` is unavailable in this checkout. Manifest and outputs cannot be independently inspected; no reuse. |
| `20260909-actual-auth` | Historical actual-window authorization run cited by prior records | Prior record says actual window PASS; six unrequested scale BLOCK rows; matrix status BLOCK | `test-results/stack-text-editor/P02/20260909-actual-auth` is unavailable in this checkout. Manifest and outputs cannot be independently inspected; no reuse. |
| `20260908-storage-populated-02` | Historical populated scale run cited by plan | Prior record says 23/23 | `test-results/stack-text-editor/P02/20260908-storage-populated-02` is unavailable in this checkout. No current validation or reusable evidence. |
| `20260908-native-caller-repair-1788862340945` | Historical caller repair cited by plan | Prior record describes native authorization follow-up | `test-results/stack-text-editor/P02/20260908-native-caller-repair-1788862340945` is unavailable in this checkout. Core storage was not rerun after repair; no reusable evidence or current approval. |

Run IDs above come from prior documentation. No cited directory, manifest, or
output is accessible in this checkout, so original source/fixture hashes and any
embedded immutable run-ID field cannot be verified. This prevents every
historical reuse and end-to-end fixture/consumer provenance claim.

### Hash anchors

SHA-256 values below are directly computed from current checkout files. Historical
manifest values are unavailable and are not asserted as independently verified.

| Path | Historical artifact hash | Current checkout SHA-256 |
|---|---|---|
| `feasibility/contract.rs` | unavailable | `444b3b799d73127b27deaba257fad30e90afd23f6edf9cb74b2b3819afdf2938` |
| `feasibility/lease.rs` | unavailable | `82eb25bdef1d35cdd0a503c4550cb104a8b1d262f3e566f91b312e2cbd6ccb82` |
| `feasibility/decode.rs` | unavailable | `9e60610bec0312b1aaaca2841a9e35004600e060eb32f9c16ca91b9770062836` |
| `feasibility/session.rs` | unavailable | `7c0b15b7d2f8f0c20cd75cc61f28f35c8445a4899f38c6998b68db71a1e0df04` |
| `feasibility/source.rs` | unavailable | `a47693d76989be0036e52f4694e97574fed740e6e852cfe4f3079b5e827920a7` |
| `feasibility/scheduler.rs` | unavailable | `2e24f0ba9b9e18dadbea4ad2201725585f490b513a957b47be9a003be0fddb93` |
| `feasibility/backing.rs` | unavailable | `13bc9e7ce48af692befc5b37ed551d376a07ca7877e0d4ee73aa9aa7c9ab4e5d` |
| `protocol.rs` | unavailable | `5deabdcc0c2790884979cc09689f2685661bdfc0963cd26d5aea827554f36551` |
| `p02-run.mjs` | unavailable | `d0a2eae19cae5a1e9cf675ae7f5c9d8b0ce7a10646fce9ff565eebb4d521a0d6` |
| `textEditorProtocol.ts` | unavailable | `11dfe031ba773e4c63e003c0d7064eceee57bb601ef60ac541212c6693566f77` |
| `stack-text-editor-protocol.json` | unavailable | `390de25940400a4b1880396eca1383b47bdbd5b5da5ddb8e76b481e8c1c1f254` |

Historical/current equality cannot be established from accessible artifacts.
Old results cannot establish current-source behavior without reconstruction or
rerun, even where prior documentation recorded a matching hash.

## Contract delta and claim disposition

| Area | v1 / P02 old fields and semantics | Canonical v2 fields and semantics | Actual producer → consumer | Affected claims | Row provenance: relevant current sources/fixture; historical run IDs | Compatibility check | Disposition and rationale |
|---|---|---|---|---|---|---|---|
| Lease mapping and bounds | `ViewLease.sourceByteStart/End`, `localUtf16Length`; segments carry explicit scalar `boundaries`, byte ranges, local ranges, line-break spans; 1 MiB lease-byte cap plus 131072 UTF-16/2048 segments/4096 rows | `Lease.segments[].byteStart/text/encoding/newlines`; validators reconstruct byte boundaries; same UTF-16/segment/row ceilings, but no 1 MiB lease-byte field/cap and different wire shape | P02 `lease.rs`, `decode.rs`, `session.rs` → feasibility tests/fixture. v2 `protocol.rs` → Rust tests; TS file → isolated Node tests only | T02-02/03/04; T03-01/02/06/07 | `lease.rs` `82eb25bd…ccb82`; `decode.rs` `9e60610b…2836`; `session.rs` `7c0b15b7…df04`; `contract.rs` `444b3b79…938`; `protocol.rs` `5deabdcc…551`; TS `11dfe031…f77`; fixture `390de259…254`. Fresh run: `20260916-p02-rb02-nonscale-05`; historical artifacts remain unavailable. | Fresh canonical-v2 lease/context plus manifested T02-02/03/04 storage/oracle checks cover scalar/CRLF splits, boundary reconstruction, and limit rejection | **ACCEPT — non-scale test-only.** Technical PASS plus named storage/safety and security/privacy ACCEPT reviews support coordinator acceptance. T03 and P04 remain separate. |
| `sourceState` / `invalidAt` | Feasibility-only `ViewLease.sourceState` default `Ready`; `invalidAt` only with `DecisionRequired`, at/after readable lease end. Open/read results also expose source state | No lease `sourceState` or `invalidAt`. Failures use `ErrorCode`; session has `readOnlyReason`; incomplete text context is separate | P02 `source.rs`/`decode.rs`/`session.rs` → `ViewLease`/result consumers in feasibility tests. v2 has no equivalent producer | T02-02/04/05; T03-02/07 | `source.rs` `a47693d7…20a7`; `decode.rs` `9e60610b…2836`; `session.rs` `7c0b15b7…df04`; `contract.rs` `444b3b79…938`; `protocol.rs` `5deabdcc…551`; fixture `390de259…254`. Fresh run: `20260916-p02-rb02-nonscale-05`; historical artifacts remain unavailable. | Test-only seam maps only legacy outcomes with exact canonical codes and refuses semantically absent outcomes while preserving `invalidAt`; fresh manifested source-state checks pass | **ACCEPT — non-scale test-only.** No production semantic destination is claimed; P04 recovery remains open. |
| Selections, ownership, expiry | `DocumentSelection` owns session/source/revision, endpoint byte + `anchorId`, direction, expiry; creation request is single-lease start/end; remap status exact/deleted/expired | `Selection` adds input sequence and active/invalidated state; endpoints retain affinity; creation accepts two authoritative `WirePoint`s from different leases and validates owner/source/revision; expiry horizon 256 | P02 contract request/result types → feasibility tests; no mounted projection consumer. v2 Rust/TS helpers → contract tests only | T02-03; T03-06/09 | `contract.rs` `444b3b79…938`; `session.rs` `7c0b15b7…df04`; `protocol.rs` `5deabdcc…551`; TS `11dfe031…f77`; fixture `390de259…254`. Fresh run: `20260916-p02-rb02-nonscale-05`; historical artifacts remain unavailable. | Canonical-v2 boundary and fresh manifested P02 storage/history checks cover cross-lease ownership, forged/stale/released/expired rejection, affinity/direction, and barrier/replay behavior | **ACCEPT — non-scale test-only.** P03 projection and P04 recovery remain outside P02. |
| Incomplete grapheme context | Lease has no before/after context or continuation ID; local P02 state can expose invalid bytes but cannot encode incomplete grapheme boundary context | `Lease.context.before/after` = complete/continued plus required nullable continuation ID; `ContextRequired` rejects certification on incomplete context | P02 lease producer → feasibility tests. v2 validators/helpers → contract tests only | T02-04; T03-01/07/08 | `lease.rs` `82eb25bd…ccb82`; `decode.rs` `9e60610b…2836`; `protocol.rs` `5deabdcc…551`; TS `11dfe031…f77`; fixture `390de259…254`. Fresh run: `20260916-p02-rb02-nonscale-05`; historical artifacts remain unavailable. | Fresh canonical-v2 lease/context conversion and manifested T02-04 oracle checks exercise continuation context and split CRLF behavior | **ACCEPT — non-scale test-only.** No production continuation producer or P03 projection/AT claim follows; P04 recovery remains open. |
| Input barriers, revisions, replay | `InputBarrier.acceptedRevision/localInputSequence/operationIds`; requests also carry expected revision and caller-provided request digest; mutation receipt accepted/duplicate with revision state | `Barrier.documentRevision/inputSequence/operationIds`; canonical recipient-computed request bytes; bounded ledger pending/accepted/rejected, durable retirement, replay refusal after retirement | P02 session/contract → feasibility mutations/tests. v2 Rust/TS ledger/validators → contract tests only | T02-02/03/05; T03-03/05/06/09 | `session.rs` `7c0b15b7…df04`; `contract.rs` `444b3b79…938`; `protocol.rs` `5deabdcc…551`; TS `11dfe031…f77`; fixture `390de259…254`. Fresh run: `20260916-p02-rb02-nonscale-05`; historical artifacts remain unavailable. | Fresh canonical-v2 and manifested mutation/history/scheduler checks cover recipient canonicalization, stale/missing input rejection, duplicate immutability, durable retirement, and delayed/ABA replay | **ACCEPT — non-scale test-only.** Production authority is not claimed; P04 recovery remains open. |
| Errors, cancellation, publication | Rich `ProtocolError` message/retryable/field/expected+actual revisions; separate job state/phase/publication and cancellation receipt outcomes | Closed error code + retry/disposition; job phase and cancellation jointly constrain pre-publication, already-published, and ambiguous outcomes; ambiguous is never retryable | P02 source/scheduler/session → feasibility jobs/tests/native runner. v2 result validators → contract tests only | T02-01/02/05; T03-03/05/09; P04 remains separate | `source.rs` `a47693d7…20a7`; `scheduler.rs` `2e24f0ba…b93`; `session.rs` `7c0b15b7…df04`; `protocol.rs` `5deabdcc…551`; runner hash recorded above; fixture `390de259…254`. Fresh run: `20260916-p02-rb02-nonscale-05`; historical artifacts remain unavailable. | Fresh run occupies demand workers, proves supersession/removal/cancelled results, maps unknown completion to non-retryable `PublicationAmbiguous`, and records native command `025` | **ACCEPT — non-scale test-only.** Production publication and P04 recovery remain unproved. |
| Transport credits and resource limits | Constants include 4 range payloads, 1 MiB pending edits, 64 speculative ops, 8 leases, 64 selections, 32 jobs, 256 events/results, 1024 active/retired operations, spool quotas; scheduler owns priority/latest seek | TS `LIMITS` retains these policies and adds 2 reads/1 pending seek/hot/index memory; Rust v2 validates only subset. Contract docs say owner-specific bounded transport, no broadcast | P02 scheduler/session/backing → feasibility tests/runner. v2 TS constants and Rust partial validators → isolated tests; no actor/transport consumer | T02-03/05; T03-02/03/05; NFR-3/5/6 | `scheduler.rs` `2e24f0ba…b93`; `session.rs` `7c0b15b7…df04`; `backing.rs` `13bc9e7c…e5d`; `protocol.rs` `5deabdcc…551`; TS `11dfe031…f77`; runner `d0a2eae1…a0d6`; fixture `390de259…254`. Fresh run: `20260916-p02-rb02-nonscale-05`; historical artifacts remain unavailable. | Test-only v2 seam plus fresh manifested run check credit accounting, quota refusal, dirty-data retention, latest seek, cancellation, and no actor-lock I/O | **ACCEPT — non-scale test-only.** No production actor/IPC exists; P12 scale and P04 recovery remain separate open obligations. |

## Claim-level decision matrix

| Claim | Coordinator disposition | Remaining boundary |
|---|---|---|
| T02-01 authoritative target identity/path races independent of editor payload shape | **ACCEPT — non-scale test-only** | Fresh packet includes exact manifest plus actual-window command `025`; named security/privacy review accepted this scope. No production route follows. |
| T02-02 prefix edit/protected copy/source-state/cancellation | **ACCEPT — non-scale test-only** | Named storage/safety review accepted this scope. Populated multi-GiB proof remains separate P12 debt; P04 owns recovery. |
| T02-03 paged backing/history/quota/resource scaling | **ACCEPT — non-scale test-only** | Non-scale quota/resource evidence accepted. Populated scale acceptance belongs to P12 and remains unproved. |
| T02-04 decode/CRLF/invalid-tail/lease mapping | **ACCEPT — non-scale test-only** | Manifested oracle, distant-invalid-data, and canonical mapping accepted for P02 only. P04 recovery byte-integrity remains unproved. |
| T02-05 scheduler priority/cancellation/credits | **ACCEPT — non-scale test-only** | Scheduler evidence accepted; production actor/publication remains absent and P04-owned save/recovery proof remains open. |
| T03 cross-lease selection, grapheme continuation, projection/input/AT claims | **BLOCK** | P03 harness absent. Reconstruct under RB-03; do not treat v2 contract tests or P02 storage as projection proof. |

Historical observations remain non-reusable because cited archived artifacts are
inaccessible. Fresh run `20260916-p02-rb02-nonscale-05` supersedes that missing
provenance for current non-scale execution only; it does not relabel historical
evidence or prove production integration, recovery, or P12 scale behavior.

## Executable checks and blockers

Executed current lineage command:

```powershell
node scripts/stack-text-editor/p02-run.mjs --serial-authorized --actual-window-probe --output test-results/stack-text-editor/P02/20260916-p02-rb02-nonscale-05
```

Runner exit was 0 with `IN_REVIEW`: 25/25 executed checks passed, discovery was
26 total / 4 canonical-v2 / zero missing descriptors, and six omitted populated
scale rows were retained as `P12/NFR-6` BLOCK debt rather than P02 non-scale
failures. `native-caller-result.json` records actual `stack-popup` acceptance,
actual `top-bar` rejection for existing and missing targets, zero rejected-open
content disclosure, `snapshotComplete=true`, and `recoveryComplete=false`.
`review-packet.json` explicitly records storage and security/privacy review as
pending, coordinator disposition `PENDING`, and gate disposition
`BLOCKED-PENDING-REVIEWS-COORDINATOR-AND-P04`.

Those values preserve packet generation-time state. Subsequent disposition is:
technical review **PASS**, named independent storage/safety **ACCEPT**, named
independent security/privacy **ACCEPT**, coordinator **ACCEPT** for canonical-v2
non-scale P02/RB-02 storage exit only. This authorizes P04 entry; it does not
retroactively alter the packet or establish P04 recovery or P12 scale.

The broad run still includes v1/local-v2 lineage and cannot alone close RB-02.
The isolated test-only boundary now makes these canonical-v2 checks runnable:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml stack_popup::text_document::feasibility::tests::rb02_v2_lease_mapping_and_context -- --exact --nocapture
cargo test --manifest-path src-tauri/Cargo.toml stack_popup::text_document::feasibility::tests::rb02_v2_source_outcomes_and_errors -- --exact --nocapture
cargo test --manifest-path src-tauri/Cargo.toml stack_popup::text_document::feasibility::tests::rb02_v2_selection_barrier_replay -- --exact --nocapture
cargo test --manifest-path src-tauri/Cargo.toml stack_popup::text_document::feasibility::tests::rb02_v2_transport_credits_and_cancellation -- --exact --nocapture
```

The four checks validate lease/CRLF/context conversion, explicit refusal of
non-equivalent legacy source outcomes, cross-lease ownership plus barrier/replay,
and scheduler credit/latest-seek/cancellation outcomes through canonical v2
validators. `p02-run.mjs` describes them as `RB-02-v2`, rejects zero discovery,
and snapshots both canonical-v2 files, canonical fixtures, legacy fixture, and P02
sources. Fresh run `20260916-p02-rb02-nonscale-05` performed the authorized
manifested non-scale runner and actual native-window probe. Populated scale was
not run and remains explicit non-gate P12/NFR-6 debt.

## Ordered next action

1. Coordinator disposition complete: P02/RB-02 canonical-v2 non-scale storage exit accepted after technical PASS and both named independent ACCEPT reviews.
2. P04 may enter and must independently establish recovery root, saving/publication behavior, retention, and recovery security; `snapshotComplete=true` remains distinct from `recoveryComplete=false`.
3. P12 runs the separate populated scale campaign and retains failures; this disposition records no NFR-6 scale acceptance.
