# Speech EOU Phase 0 Evidence

## Status

**EOU rejected; bounded local TDT fallback selected. Production capture remains
subject to the limitations below.**

Phase 0 candidate failed licensed-fixture transcript quality. It also lacks a
verified full immutable source revision and downstream redistribution notice
review.

## Candidate artifact

| Field | Value |
| --- | --- |
| Publisher | `altunenes` |
| Source | `https://huggingface.co/altunenes/parakeet-rs/tree/main/realtime_eou_120m-v1-onnx` |
| Requested ref | `0bd721c` (short ref only; full commit ID not verified) |
| Required layout | `encoder.onnx`, `decoder_joint.onnx`, `tokenizer.json` |
| Encoder SHA-256 | `D472887CC38A784A5BFC21C2DBE247639EDC3B3F9992388D8CECEAEC07256B5B` |
| Decoder SHA-256 | `9D2553AC043C2FC5F69E970769B0FB8AB9103FBFDEB7D26A1EA9729D4BD2DDDD` |
| Tokenizer SHA-256 | `F6B0AD8690559351FA478116FE0985A203B76F7C040F3A9381F485C99C0325F8` |

Artifact stayed outside repository and runtime resources in owner-approved
temporary storage. Runtime made no network request and no audio/transcript was
persisted by application code.

## License and redistribution route

Upstream model page: `https://huggingface.co/nvidia/parakeet_realtime_eou_120m-v1`.
It identifies NVIDIA Open Model License. That license permits commercial use and
derivative-model distribution, with redistribution requiring the agreement and a
Notice saying: `Licensed by NVIDIA Corporation under the NVIDIA Open Model License`.

This third-party packaging record does not establish the full immutable upstream
revision, complete downstream provenance, or required bundled license/notice
contents. Legal/provenance gate remains open.

## Offline compatibility result

`src-tauri/tests/eou_artifact_spike.rs` loads the local artifact using
`ParakeetEOUHandle::from_pretrained`, sends 16 kHz mono 2,560-sample chunks, and
uses three zero chunks for documented flush. Silent-stream compatibility passed.

Actual crate API is `transcribe(chunk, reset_on_eou)`: `true` resets on detected
EOU; it is not an input-final flag. Finalization must use a padded tail plus zero
flush if a later approved artifact passes all gates.

## Licensed fixture quality result

Fixture source: [LibriSpeech SLR12](https://www.openslr.org/12), `test-clean`,
CC BY 4.0; attribution: Panayotov et al., 2015. Archive source:
`https://www.openslr.org/resources/12/test-clean.tar.gz`.

| Field | Value |
| --- | --- |
| Exact sample | `61-70968-0002.flac` |
| Source FLAC SHA-256 | `8676316E531D8C0E496E3677811ED2BF63E5AAF0A12446CDE70416FD490B1792` |
| Test input | 16 kHz mono signed-16-bit little-endian PCM via `ffmpeg -ac 1 -ar 16000 -f s16le` |
| Test PCM SHA-256 | `102032BF20B34EED7261198031DAA2AB06FF2CA3AE92B346B1346F91922C802D` |

Test uses this public English sample, normalized expected words, and never
prints transcript content. Candidate returned a non-matching normalized result
under documented chunk and flush sequence.

Command:

```powershell
$env:JASONSHELL_EOU_MODEL_DIR = '<approved-local-artifact>'
$env:JASONSHELL_EOU_FIXTURE_PCM = '<approved-16khz-mono-s16le-fixture>'
cargo test --test eou_artifact_spike -- --ignored
```

Result: 1 compatibility test passed; 1 fixture-quality/no-duplicate-tail test
failed. This triggers Phase 0 blocking gate 2 in
`docs/speech-streaming-implementation-plan.md`.

## Approved route and remaining limitations

Owner approved bounded local TDT fallback after EOU rejection. Selected route:
finite overlapping inference, not stateful streaming. Approved implementation
parameters are fixed 2,560-sample chunks, queue capacity 32, a 300-second cap
from successful play, and a 45-second finalization deadline after close.

EOU remains rejected because candidate transcript quality failed and artifact
provenance/legal review is incomplete. This record does not establish artifact
legal clearance, true/stateful streaming behavior, live microphone or clipboard
end-to-end validation, or full release validation.

Focused fallback tests and checks pass. Consent-gated Windows microphone and
clipboard end-to-end validation remains pending. Broad `npm run test:node`
baseline remains 43 failures unrelated to speech.
