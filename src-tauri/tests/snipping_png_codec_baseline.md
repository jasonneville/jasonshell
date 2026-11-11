# Synthetic PNG codec baseline — 2026-11-09

Test/measurement artifact only. No native runtime launched for this assignment.
No desktop/clipboard data, AppBars, foreground changes or new dependencies.

This artifact's historical measurements and goldens cover the stored `png_rgba`
serializer, which remains the Save/clipboard path and the compact transport's
fallback; they do not measure the newer `png_rgba_transport` branch or current
native capture. Retained executable hashes are measurement-time historical
snapshots, not claims about the current checkout or final release.

## Preserved prior native evidence (not rerun)

Approved numeric-only CDP attempt 2 observed secondary DPR **3**, viewport
**1268×714**, overlay DOM rectangle **(0,0,1268,714)**. Original physical
gesture `(3000,400)→(3240,580)` produced actual DOM pointer client coordinates
`(140.66667175293,133.333343505859)→(220.66667175293,193.333343505859)`.
Unchanged outward floor/ceil predicts **241×181** from those observed floats.
Native client screen origin was `(2578,0)`, client size `3804×2142`.

Aligned physical endpoints `(3001,402)→(3241,582)` were chosen from that measured
mapping: **computed**, not DOM-observed, `(141,134)→(221,194)` at DPR3.
`(147,134)→(227,194)` would instead be **computed** using screen origin `(2560,0)`;
these are not recorded actual DOM samples. No aligned DOM samples are claimed.

Primary initial CDP metadata: DPR1.25, viewport2034×1146, overlay DOM rectangle
`(0,0,2034.40002441406,1145.59997558594)`.

Prior successful no-debug production release hash:
`4BD94FFBDCD59CE3FCD2D814F9FB5084FDF733700BC8BB25EC560DEB880A70D9`.
Button-down clock: up116ms, first sampled new visible1538ms, exact pair1555ms,
selection1561–1937ms, valid preview2416ms, dismiss2581ms. DIB240×180;
sequence143→152. All owned runtime cleanup completed. This is one success, not
repeatability, immediate-feedback, direct arming, or bounds-drift certification.

## Baseline setup

- Production `clipboard::png_rgba`, directly included with production geometry.
- Clipboard source SHA256:
  `BA58AC2F0E9E0267854426168E5EBA4D3155E71BD94CEB8F9A851A55F4481BBE`.
- rustc1.97.1 (`8bab26f4f`, 2026-07-14), release profile, existing project config.
- Synthetic RGBA: R=x mod256, G=y mod256, B=(x XOR y) mod256, A=255.
- Inputs preallocated outside timing. Both retained buffers total47,923,200 bytes.
- Two warmups per monitor, five samples per monitor, fixed primary→secondary order.
- `Instant` measures only `png_rgba` including its output allocation/encoding;
  generation, decoding, checksums in test oracles, hashing, logging and output
  destruction excluded. Pair numbers sum the two encode spans, not wall time.
- No cargo/rustc processes active when measured. Ambient OS load uncontrolled.

## Measured release results

Primary2560×1440: input14,745,600 bytes; PNG14,748,233 bytes.
Sorted encode milliseconds: **121.2143,121.3120,121.5764,121.9841,123.7601**.
Median **121.5764ms**, range **121.2143–123.7601ms**.
PNG SHA256 `0226e03af2d0658b2b91bfd955939e79893bdb1009887cf975709110bad277cd`.

Secondary3840×2160: input33,177,600 bytes; PNG33,182,358 bytes.
Sorted encode milliseconds: **270.4496,271.4920,272.1139,273.9163,281.5995**.
Median **272.1139ms**, range **270.4496–281.5995ms**.
PNG SHA256 `e95f10d85bdf7b17b7debc5caf4d24b038162bf5a78806d40f00507dd61421ea`.

Sequential pair sorted sums: **391.6639,393.4259,393.4761,397.6764,403.1759ms**.
Median **393.4761ms**, range **391.6639–403.1759ms**.
This does not attribute the prior native1555ms overlay latency to encoding.

## Correctness and comparison contract

Normal tests independently parse exact IHDR/IDAT/IEND framing, filter0, zlib
stored-block header/final-bit/LEN/NLEN boundaries, and trailing-data absence.
Table-based CRC and weighted-sum Adler oracles have known-answer vectors and
check every payload. Existing `png` decoder explicitly enables checksums and
compares every decoded RGBA byte. Real-resolution golden SHA256 values preserve
byte-for-byte output. Fixtures include black/white RGB0/255, patterned opaque
pixels, filtered lengths5549/5553/5557 and65533/65535/65537, plus multirow data.
Invalid dimensions/lengths/retained counts, integer overflow and retained-plus-
actual-capacity512MiB budget boundary remain enforced without huge allocations.
Allocation failure injection and isolated33.6M-pixel-limit acceptance are not
claimed: the existing public API supplies no allocator failpoint, and testing
oversized valid slices would exceed this assignment's bounded fixtures.

Before/after: run normal correctness tests unchanged, then the identical release
benchmark serially on the same machine/compiler/profile and predetermined buffers.
Repeat both versions if ambient noise makes ranges overlap; compare median/range
and exact PNG bytes, not a flaky universal time assertion. No established project
codec latency budget exists here, so this baseline is not fabricated performance
RED. Full actual production-release native E2E remains required after optimization.
Synthetic assignment E2E skipped: pure codec test artifacts, no integrated change.

## Commands

Standard focused checks:

```powershell
npm run cargo:test -- --release --test snipping_png_codec -- --test-threads=1 --nocapture
npm run cargo:test -- --release --test snipping_png_codec warmed_real_resolution_encode_baseline -- --ignored --exact --test-threads=1 --nocapture
```

Baseline execution used `cargo rustc --manifest-path src-tauri/Cargo.toml --release
--test snipping_png_codec --message-format=json`, then its reported test executable
directly, first normal tests and then the exact ignored benchmark. Initial build
command timed out after producing the test executable; no compilers remained when
tests/benchmark ran. Production release executable hash was still the prior
certified hash at measurement time.

A subsequent full build after adding golden-byte assertions completed successfully
in3m25s; correctness5/5 and rustfmt passed afterward. Cargo also relinked the app
binary (despite selecting the test target): current release SHA256 is
`19CF33CDF64A450B341BB0BEEAF1C2FCDF1EB735E68980BC5E5309A091A9EEAD`.
Production clipboard source hash stayed unchanged. No native runtime was launched;
the new release hash has no native E2E certification. Do not reuse the old hash's
native success as evidence for this newly linked executable.

## Independent optimized measurement — 2026-11-10

Original baseline above preserved. Optimized production clipboard source SHA256:
`5F70A8E8CA0D017A7374C391F9417C37E622F153AB13981865BE2FDFD2C68311`.
Fixture unchanged before/after: SHA256
`21DD30DC354666D9033B69C33F046F6AF627FF9DB5D60437A0666064A22762BB`.
Same compiler/profile, inputs, retained47,923,200, two warmups and five samples.
Compilation completed before correctness/timing. Cargo/rustc and known shell/native
fixture process counts were0 before and after timing. Ambient OS load, power state,
temperature and other applications were not controlled; this is not a universal SLA.

Commands executed serially:

```powershell
node scripts/prepare-rustc-wrapper.mjs
cargo rustc --manifest-path src-tauri/Cargo.toml --release --test snipping_png_codec --message-format=json
& src-tauri/target/release/deps/snipping_png_codec-335229b6309585b0.exe --test-threads=1 --nocapture
& src-tauri/target/release/deps/snipping_png_codec-335229b6309585b0.exe warmed_real_resolution_encode_baseline --ignored --exact --nocapture --test-threads=1
```

Release build completed3m26s. Normal correctness **5 passed,0 failed,1 ignored**;
exact opt-in benchmark **1 passed,0 failed,5 filtered**. Both exact output lengths
and golden hashes printed above were unchanged, alongside independent pixel,
CRC/Adler, stored-block/boundary and retained-capacity budget checks.

Primary sorted milliseconds: **41.0113,41.3786,41.6557,41.6921,41.9248**.
Median **41.6557ms**, range **41.0113–41.9248ms**, baseline median121.5764ms:
**2.918602×** median speedup.

Secondary sorted milliseconds: **93.1295,93.5361,93.6050,93.6083,95.0605**.
Median **93.6050ms**, range **93.1295–95.0605ms**, baseline median272.1139ms:
**2.907044×** median speedup.

Pair sorted sums: **134.6163,134.7852,135.3004,135.4609,136.4391ms**.
Median **135.3004ms**, range **134.6163–136.4391ms**, baseline median393.4761ms:
**2.908167×** median speedup (**65.614074%** lower encode time).
Timed spans still include output allocation but exclude generation/validation/
destruction. No new clock threshold or oracle relaxation. This is codec-only;
prior native1555ms observation is not an optimized native result or instant snip.

Measured test executable SHA256:
`6AA7A52E5685A1696C5173700034E7AAF1A7BCF308E3C60328868853F203C5BC`.
Cargo also relinked the app; current release app SHA256 at measurement completion:
`88E0F5273AAF4A4C19EB04F14FA8AB04E88CD57BB7D27E80EEEAFD525CA2512E`.
No native certification transfers from old4BD94FF...70D9 to this hash. No native
runtime launched during this assignment; E2E skipped for pure codec measurement.
Fresh clean release and actual single-display journey remain required. User
explicitly excluded two-display/mixed-DPI/post-show-dual validation from current
completion scope; its retained visible fixture was compiled but visible stages
were not executed (earlier inventory gate found one display). No geometry cause
was established or claimed fixed.

## Single-display profile preparation (no actual journey)

After the quiet codec measurement, metadata-only command:

```powershell
pwsh -NoProfile -File tests/snipping-release-native-e2e.ps1 -AcceptanceProfile SingleDisplay -ProfileMetadataOnly
pwsh -NoProfile -File tests/snipping-release-profile.test.ps1
```

Observed PMv2 monitor `(0,0,3840,2160)`, work area `(0,0,3840,2160)`, effective
DPI288. This read native monitor metadata only: no application/overlay HWND was
created or client rectangle observed. Profile's required overlay client target is
the full monitor. Predicted physical gesture `(288,288)→(576,576)` corresponds to
CSS `(96,96)→(192,192)` **if** exact client coverage and scale288/96 hold;
expected clipboard is288×288, not a measured DOM sample. Preview prediction from
current work area and source320×264 logical sizing is `(2880,0,960,792)`.
Actual run recalculates work-area expectation after shell bar startup and before
input, then rejects work-area/topology changes; this metadata-only preview value
must not be reused blindly after AppBars reserve space.

New profile preserves owner/PID/epoch/hit-test checks, exact full client coverage
before and during selection, capture exclusion, external foreground/nonactivation,
exact selected image dimensions/changed sequence and dismissal. LegacyDual keeps
its original bounds,240×180 crop and preview oracles. Pure preparation tests passed
4 valid geometries and17 invalid cases, parser and C# compilation with0 native
methods executed. First pure run caught PowerShell comma/arithmetic precedence in
new array construction; explicit grouping repaired it before any metadata/native
journey. Subsequent metadata-only read passed. Full single-display capture journey
and responsiveness measurement remain unrun pending Leader's fresh-build/hash
handoff. Two-display coverage excluded by user, not a current completion blocker.
