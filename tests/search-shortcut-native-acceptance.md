# Search shortcut: external-process acceptance

Status: **NOT RUN — explicit live Windows consent required.** This checklist does
not authorize starting JasonShell/Brave, installing native hooks, generating
desktop input, or recording input. No temporary keyboard logger is retained.

## Master merge verification

Merge validation targets `C:/dev/jasonshell`, retaining master's eight-action
registry, capture leases, expiry timer, native fences, and routed synchronous
Speech dispatch. The feature-era direct-request source assertion initially failed
(55/56 Node pass), because master executes opaque native request closures. Its
adapter now requires acquisition pause around request execution and unconditional
verified-state refresh before publishing the closure response, including failure.
Merged focused Node passes 57/57; `npm run check` reports zero errors/warnings,
`npm run build` passes, and `npm run cargo:check` passes.

Safe merged Rust module gate passes **53/53**, zero failures or ignored cases,
957 filtered out: 25 Search fixtures, 22 master capture/registry fixtures, and
six existing module cases. Command:

```powershell
npm run cargo:test -- --bin jason-shell windows_key_hook -- --skip task_windows::native_hooks::tests::real_worker_stops_and_clears_native_resources
```

This is a filtered module run, not a full-suite rerun. Existing Cargo warnings
remain; the prohibited real native-worker smoke was not run during merge validation.

Four added deterministic production-state integration cases cover:

- capture pauses new Search acquisition while preowned repeats/releases remain
  swallowed, with registered Search routed only to the active capture lease;
- capture end discards old queued Search and permits only a fresh post-release
  trigger cycle, never a preheld repeat;
- native capture fences preserve prior emission permits without letting permit
  Drop reopen capture-paused or permanently retired admission;
- capture expiry resumes only verified active Search, preserving all seven other
  slots even when Search is unavailable after partial rollback.

The fixtures use in-process thread messages and observed synthetic state, not
OS hook registration, real key injection, windows, or Tauri events. Resumption
tests preseed the observed pressed trigger to short-circuit `GetAsyncKeyState`;
they do not inspect live desktop input. The merged Speech source assertion keeps
both active-thread/`!expired` guarding and master's routed dispatch, plus the
existing synchronous original-target preparation before UI emission.

No full Rust/default gate or native smoke is authorized by this merge check.
The native Search/Brave E2E and original-event/root-cause determination remain
pending consent. Earlier feature-branch counts below are historical snapshots,
not claims of a new merged full-suite run.

## Feature-branch reviewer-blocker regression gate — resolved by automated evidence

Test-first evidence against the first implementation was focused Node 53 total,
51 pass and two expected failures, plus Rust compile RED with nine missing-seam
errors. After the owner implemented both fixes, focused Node passes 53/53 and
Rust `windows_key_hook` passes 27/27, including all 21 Search fixture cases.
No source/behavior assertions were removed to obtain GREEN; constructor adapters
only gained the new production fields.

Four added behavior fixtures now exercise these production seams:

- `reconcile_search_startup_keys([bool; 256], [Option<bool>; 256]) -> [bool; 256]`:
  accept an authoritative post-install sample, with callback-observed per-key
  transitions winning over a raced sample. Source contracts verify hook-first
  sampling, callback observation recording, and reconciliation before readiness;
  source inspection confirms pressed/held are seeded from that merged sample. Tests
  cover Ctrl released in the pre-install gap, observed release/press races, and
  a preheld Search trigger that remains unowned until a fresh released cycle.
- `SearchDispatch::try_admit_emission() -> Option<permit>` and `retire()`:
  serialize admission with retirement, retain the permit through the emission,
  and do not block retirement on a previously admitted IPC side effect. Channel
  barriers force stale-live-read → retirement → admission-rejection, and
  admission → retirement → allowed completion with later admission rejected.
  The fixtures use mock emission counts, not Tauri or desktop input.

The retirement test checks rejection both while prior admitted work is in flight
and after permit release with `pending` deliberately re-enabled, so cleared
pending state cannot hide a gate that accidentally reopens. The startup tests
exercise the production merge helper and ownership state; actual hook installation
timing is covered by source ordering, not live callback delivery.

## Automated evidence boundary

Before the change, the three focused Node files passed 29/29. With test-first
acceptance contracts and unchanged production code, they passed 26/29: RED for
missing Search native ownership, missing key transition handling, and the old
Search `WM_HOTKEY` emission (duplicate delivery risk).

Feature-branch focused Search/standard/speech/close contracts passed 53/53; Rust
`windows_key_hook` passes 27/27, including all 21 Search fixture cases.
Prior `npm run check`, `npm run build`, and `npm run cargo:check` passed; frontend
inputs are unchanged by the reviewer fixes, and the owner reran Cargo check.
The last broader Node run, before the two reviewer source contracts were added,
had 1,395 tests: 1,382 pass, seven fail, three skipped,
three TODO. The seven failures were also observed before production edits and
their relevant inputs have no diff from HEAD
`0e2643f67f726f64cf6ab6110b2a14450d5b897b`:

- `centeredSearchSurface.test.mjs:84,104`: existing combobox/option source shape.
- `changelogPolicyHygiene.test.mjs:32`: missing expected AGENTS wording.
- `searchClearButtons.test.mjs:44`: existing clear-button source shape.
- `searchPanelState.test.mjs:536,545`: existing scoped visible-row source shape.
- `sourceContractIntentRegistry.test.mjs:152`: existing missing registry entry
  for `searchPanelPlacementContract.test.mjs`.

The newly exposed speech source-shape assertion was updated to require the
shutdown admission guard while retaining synchronous prepare-before-emission.
Full Node remains non-green; these results do not turn `npm run validate` green.

The prior broader Rust binary gate with the explicit existing native-smoke
exclusion passes: 934 passed, zero failed, seven ignored, one filtered out
(181.10 seconds). It includes all 17 Search fixture cases. The three new adapter
cases also passed five fresh-process repetitions (15/15 executions). Since the
reviewer fixes touch only the hotkey module, its current 27/27 focused gate updates
that prior broader snapshot; the 934 count is not a claim of a new full-suite run.
This is bounded in-process evidence, not cross-process Search/Brave acceptance.

Run from the repository root:

```powershell
node --test tests/windowsKeyOverride.test.mjs tests/hotkeySystem.test.mjs tests/searchCloseReset.test.mjs tests/speechHotkeyPasteTarget.test.mjs
npm run cargo:test -- --bin jason-shell windows_key_hook -- --skip task_windows::native_hooks::tests::real_worker_stops_and_clears_native_resources
```

For a broader binary-unit gate without the existing unrelated taskbar-hook smoke:

```powershell
npm run cargo:test -- --bin jason-shell -- --skip task_windows::native_hooks::tests::real_worker_stops_and_clears_native_resources
```

Safety finding during this task: an initial unfiltered binary-unit gate ran the
existing `real_worker_stops_and_clears_native_resources` test before its live
WinEvent/shell-notification hook behavior was noticed. That test starts then stops
the real taskbar worker and passed its resource-clear assertions. This was not a
consented native smoke and must not be described as a no-hooks run. No Search
keyboard hook, shell application, Brave journey, or synthetic desktop input was
started. Subsequent broad gates exclude that exact existing test. Its live
operations require separate consent; this checklist does not grant it.

`src-tauri/tests/fixtures/search_hotkey_ownership.rs` is included by the
production module under `cfg(test)`. Its original 14 cases drive production state,
never an independent model. They make queue admission observable without OS hooks:
initial down queues immediately, down/repeats/up are consumed, toggle count is
one; focus/modifier changes cannot surrender an owned release; rebind, disable,
preheld input, negative codes, unrelated chords, and queue rejection are bounded
deterministic cases. Three adapter cases additionally exercise the actual
production queue against an isolated test-thread message queue, the consumed
callback branch with stack-owned callback data and seeded left/right Ctrl/Alt,
and lifecycle admission/state retention/reaping with an ordinary Rust worker.
They never install a hook, generate desktop input, create windows, start the
dispatcher/Tauri runtime, or execute `CallNextHookEx`. This establishes the
consumed callback mapping and one-message queue admission, not hook delivery.
Node assertions check source contracts, not OS delivery.

Remaining automated gaps: negative/injected/unowned callback forwarding is
source-inspected rather than invoked against a live hook chain; OS posting
failure, actual Tauri emission/teardown, timer failure, installation/unhook
failure, and the bounded 250ms shutdown-drain behavior lack injected runtime
seams. The emission-admission/retirement ordering itself now has deterministic
production-permit barrier coverage, but does not mount the Tauri dispatcher.
Pure queue-failure/ownership cases do not establish those OS lifecycles.
Adding a narrow injectable native-operation seam would enable deterministic
adapter fault tests, but is outside this test-only implementation scope.

Neither suite proves external-process input suppression, Brave media state,
native hook timing, or real physical-key delivery. No percentage hardware
coverage is claimed.

## Consent-gated initial trace gate

1. Obtain separate explicit consent for the live shell/native-hook run and any
   temporary diagnostics. Record executable/build identity, Windows version,
   keyboard layout, configured Search chord, and Brave version. Do not retain
   browsing history, typed text, arbitrary keys, URLs, or input logs.
2. Use a disposable browser tab playing public/non-sensitive media. Record only
   initial/final playing-or-paused state and observed Search open/closed state.
3. First reproduce the report using the unfixed build: with Brave playing,
   open Search using Ctrl+Space, then close using Ctrl+Space. Test release orders
   Space-before-Ctrl and Ctrl-before-Space. Record whether media pauses and
   whether opening versus closing differs. If not reproducible, report that
   limitation; do not assert that a hook patch resolves the original cause.
4. If a scoped trace is necessary, request authorization first; restrict it to
   Search trigger transition counts/consumption and dispatch counts, do not log
   arbitrary keyboard activity, and remove temporary instrumentation afterward.
   The trace gate is currently blocked, not passed by the pure fixture.

## Fixed-build external-process journey

After consent and build identity verification, use real physical input. Start
JasonShell through its documented entrypoint only when authorized. Do not invoke
native input injection merely to make this checklist automatic.

- Brave foreground, media playing: tap Ctrl+Space to open, release, then tap to
  close. Search opens/closes once each; media remains playing throughout.
- Repeat with Space released before Ctrl, then Ctrl released before Space.
- Hold the Search trigger long enough for autorepeat during closing, releasing
  modifiers first. Search toggles once; media remains playing; no reopen.
- While the trigger is held, allow Search close/focus return to Brave; release
  there. No leaked trigger release/repeat and no second toggle.
- In Brave, bare Space still pauses/resumes; Ctrl+Shift+Space and unrelated
  shortcuts remain unowned. Terminal, Stack Browser, speech, and snipping retain
  existing registrations; avoid actually starting capture/recording without
  additional consent. Never claim those actions passed from static tests alone.
- Remap Search to an allowed alternative such as Alt+K. Verify the new chord
  opens/closes once, old Ctrl+Space is no longer Search-owned, and Alt release
  does not become a new blanket suppression path.
- Hold the configured trigger before startup/rebind; adding its modifier or
  autorepeat must not activate Search. Release fully, then a fresh chord works.
- Rebind while an owned old trigger is held; original repeats/up stay owned
  until release. Verify later old-chord input passes and new-chord input works.
- Exercise registration conflict/write-failure rollback only in a separately
  authorized disposable setup. The unavailable Search chord must not be owned;
  verified restored Search must work. Pure fixtures establish only the state
  invariant, not actual conflict/recovery timing with another process.
- Quit/reinstall while a trigger is held only with explicit approval. Record
  bounded shutdown behavior, owned-release handling, no stale registrations,
  and no preheld activation after reinstall. Abnormal process termination cannot
  guarantee suppression after the process is gone; do not claim otherwise.

For each executed case record: build ID, chord, release order, foreground
process, Search toggles, media before/after, PASS/FAIL/NOT RUN, and cleanup.
Restore original shortcuts, close only test-owned tabs/processes, and verify
shell work-area cleanup. Keep a native failure as unresolved evidence rather
than replacing it with browser-mocked IPC results.
