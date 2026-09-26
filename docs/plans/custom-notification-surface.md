# JasonShell Custom Notification Surface Implementation Plan

**Status:** Draft — implementation plan only; no behavior described here is shipped
**Date:** 2026-09-26
**Owner:** JasonShell shell/platform work
**Reviewers:** Windows/native, frontend/accessibility, QA/release owners (to be assigned)
**Decision state:** Phase 0 evidence and the unresolved decision gates below must be reviewed before implementation begins

## 1. Summary

Build a JasonShell-owned notification surface: a bounded stack of themed, rounded rectangular cards in the bottom-right of the **primary monitor**, positioned above the JasonShell bottom AppBar. First-party JasonShell producers own the complete lifecycle, including delivery, rendering, timeout, dismissal, review, and action execution. The surface is a new hidden, non-activating Tauri/WebView window backed by a dedicated Win32 style helper and a single bounded coordinator queue.

The supported product promise is narrow and honest:

> When JasonShell itself creates a notification, JasonShell can render and control that notification in its own themed card stack without moving focus, while the user can review it and invoke its explicitly declared actions.

This plan does **not** promise that JasonShell can intercept every Windows toast before Windows displays it, replace Explorer's notification UI, disable Windows banners programmatically, read a universal Explorer badge/unread count, or provide multi-monitor placement. A third-party mirror is a separate, opt-in, post-delivery lane and cannot be treated as an equivalent replacement.

## 2. Corrected Windows-notification onboarding

“Turn off Windows notifications so only ours show” must be implemented as a manual, user-confirmed onboarding flow, not as a settings mutation:

1. JasonShell explains that the user must open Windows notification settings for a selected sender.
2. The user manually turns that sender's **banner notifications OFF** while leaving **Notification Center/history ON**.
3. The user returns to JasonShell and explicitly confirms the setting.
4. JasonShell records only the user's consent/onboarding state and sender selection; it does not write registry values, policy, notification settings, or other Windows configuration.
5. JasonShell shows status as `user-confirmed`, `not-confirmed`, `unknown`, `drift-suspected`, or `listener-unavailable`. It must never label banner suppression as machine-verified unless a future supported Windows API proves it.

Windows provides no documented supported generic pre-display interception point, no generic programmatic per-sender banner-disable operation, and no cross-app Explorer badge-read contract. A manual confirmation is therefore a user instruction, not proof. JasonShell must explain that another Windows surface, a sender setting change, focus mode, policy, or an OS update may still produce or suppress a banner. JasonShell must never claim transparent replacement or guarantee that “only ours” will show.

The onboarding copy must also state that the original third-party notification remains in Windows Notification Center. A JasonShell card dismissal only dismisses the JasonShell copy; it does not dismiss, delete, or alter the original Windows notification.

## 3. Scope, lanes, and non-goals

### 3.1 Delivery lanes

| Lane | Initial status | Ownership | Required behavior |
|---|---|---|---|
| First-party JasonShell notifications | Required first | JasonShell producer and coordinator | Full lifecycle and action control; works without the Windows listener |
| Third-party post-delivery mirror | Optional later | Windows listener adapter plus JasonShell coordinator | Only after Phase 0 proof; selected/allowlisted senders; no historical replay; original remains in Notification Center |

Phase 0 must permit the first-party lane to proceed even if the third-party listener proof fails. Third-party support is unavailable until package identity, manifest capability/configuration, UI-thread access request, and visible-content observation are all proven in a packaged Windows build.

### 3.2 Explicit non-goals

JasonShell MUST NOT:

- mutate Windows notification settings, policy, registry, focus mode, or per-sender banner state;
- intercept or suppress a third-party toast before Windows displays it;
- use private Explorer scraping, undocumented hooks, window injection, accessibility scraping, or process injection to imitate interception or read badges;
- claim cross-app Explorer badge/unread parity;
- replay all existing Notification Center history when the listener starts or a sender is enabled;
- treat a third-party mirror as a replacement for the original notification;
- add multi-monitor placement in this effort;
- persist notification body text, icons/media, action payloads, or arbitrary third-party content;
- accept remote HTML, script, remote media, or unbounded markup in a card;
- reuse the current shell style helper for the notification stack, because that helper removes `WS_EX_NOACTIVATE`.

## 4. Current evidence and implementation boundaries

- The runtime target is Windows with primary-monitor top and bottom AppBars; multi-monitor behavior remains planning-only.
- Surface routing is currently `src/lib/shellSurface.ts`, `src/lib/surfaceLoader.ts`, and `src/App.svelte`. A notification surface must be added deliberately rather than inferred from an existing popup.
- Native window construction is in `src-tauri/src/shell_windows.rs`. Existing popups are always-on-top, unfocused, undecorated, skip-taskbar, hidden initially. Their shared shell style path is not suitable for this stack because `desired_shell_ex_style` removes `WS_EX_NOACTIVATE`.
- The current listener in `src-tauri/src/task_windows/notifications.rs` polls once per second, requests access from a spawned worker thread, counts toast IDs by AUMID since focus, and does not yet establish the package/capability/UI-thread/content-visibility proof required for mirroring.
- Themes are defined in `src/lib/themes.ts` and shared `--js-*` CSS tokens/theme synchronization must be reused rather than duplicated.
- The exact Tauri configuration/capability/manifest path for a notification window, event permissions, and Windows package identity is **to validate in Phase 0**. This plan does not assert that a particular current config path already exists or is sufficient.

## 5. Proposed architecture

### 5.1 Ownership flow

```text
First-party producer ─┐
                      ├─> typed ingress ─> one NotificationCoordinator ─> bounded state
Listener adapter ─────┘                                      │
                                                             ├─> event/snapshot IPC bridge
                                                             └─> action result / diagnostics
                                                                    │
                                         hidden notification-stack WebView window
                                         (themed, non-activating, primary monitor)
```

There is one coordinator/queue for ordering, deduplication, capacity, expiry, pause/review state, action claims, and visibility decisions. Producers and adapters must not independently manipulate the WebView or create competing queues. Native polling, if ultimately supported, is an input adapter only.

### 5.2 Native window

Add a dedicated hidden notification-stack window builder, likely alongside the existing builders but with its own helper and tests. It should be always-on-top, undecorated, skip-taskbar, initially hidden, and non-activating. The Win32 helper must explicitly preserve/add `WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE` and must not call the existing `apply_no_alt_tab_shell_style` behavior that removes `WS_EX_NOACTIVATE`. Validate hit-testing, pointer delivery, focus behavior, z-order, and show/hide transitions on the packaged app.

The host should avoid a transparent WebView configuration unless a concrete clipping and pointer model is proven. Rounded cards can be rendered inside an opaque or controlled host; if transparency is required, prove that the WebView background, clipping, shadow, and pointer hit regions do not produce a rectangular click-capture surface.

### 5.3 Coordinator state

The coordinator owns:

- pending and visible collections;
- stable notification identity and deduplication;
- source/lane policy and selected-sender allowlist;
- age/expiry and pause-on-hover/review behavior;
- action ownership and exactly-once claims;
- bounded overflow behavior and content-free diagnostics;
- snapshots for renderer recovery and event sequence/versioning;
- explicit unavailable/error states for the listener adapter.

Suggested Phase 0 defaults, subject to approval in the Phase 0 decision gate:

- maximum visible cards: **4**;
- maximum pending entries: **50**;
- informational auto-dismiss: **8 seconds**;
- first-party actionable auto-dismiss: **15 seconds**;
- hover pauses countdown; review mode pauses countdown for deliberate keyboard/pointer review;
- newest accepted notification enters the queue; overflow drops the oldest pending item, never a visible card or an already-claimed action;
- all limits are configuration constants, tested as policy, and not silently changed by the renderer.

### 5.4 IPC event/snapshot bridge

The bridge must support:

- initial `snapshot` after window load or renderer reconnect;
- ordered `notification-added`, `notification-updated`, `notification-removed`, `queue-status`, `theme-updated`, and `listener-status` events;
- a monotonic coordinator revision or event sequence so stale events are ignored;
- renderer-to-coordinator `dismiss`, `invoke-action`, `enter-review`, `leave-review`, `pause`, and `resume` requests;
- explicit success/failure responses for action invocation;
- idempotent replay-safe action request IDs;
- a full snapshot fallback after a missed event, restart, or version mismatch.

No renderer-generated notification is authoritative. The coordinator validates identity, source, action, expiry, and current state before applying a request.

## 6. Contracts and data model

The eventual TypeScript/Rust representation may differ in syntax, but the following constraints are mandatory and must be converted into shared/static contracts before implementation.

### 6.1 Notification contract

```ts
type NotificationLane = 'first-party' | 'third-party-mirror';
type NotificationSeverity = 'info' | 'success' | 'warning' | 'error';
type NotificationState = 'pending' | 'visible' | 'paused' | 'dismissed' | 'expired' | 'acted' | 'rejected';

interface NotificationAction {
  id: string;                 // bounded identifier, not executable code
  label: string;              // sanitized, bounded text
  kind: 'command' | 'open-surface' | 'open-url' | 'dismiss';
  target: string;             // validated against an allowlist for the kind
}

interface NotificationInput {
  id: string;                 // producer-stable, unique within source
  lane: NotificationLane;
  senderId: string;           // first-party namespace or selected AUMID
  senderLabel: string;
  title: string;
  body: string;
  severity: NotificationSeverity;
  icon?: 'info' | 'success' | 'warning' | 'error' | 'sender';
  actions: NotificationAction[];
  createdAt: number;
  expiresAt?: number;
  sourceEventId?: string;     // listener correlation only; not a replay cursor
}

interface NotificationCard extends NotificationInput {
  state: NotificationState;
  receivedAt: number;
  visibleAt?: number;
  pausedAt?: number;
  revision: number;
}
```

Constraints:

- IDs, sender labels, titles, body, action labels, and targets have explicit byte/length limits and Unicode/control-character sanitization.
- Actions are declarative, typed, allowlisted, and validated again at execution; no arbitrary command or script text crosses the renderer boundary.
- Third-party data is treated as untrusted input and is reduced to a safe text/icon/action subset.
- `createdAt` and listener IDs cannot cause historical replay: only events observed after an enabled baseline are eligible for mirroring.
- Card content, media, and action payloads are memory-only. Only consent, selected senders, safe preferences, and required version/status metadata may persist.

### 6.2 Settings and listener status

Persist only a versioned settings shape similar to:

```ts
interface NotificationSettings {
  enabled: boolean;
  firstPartyEnabled: boolean;
  thirdPartyMirroringEnabled: boolean;
  selectedSenderIds: string[];       // allowlisted AUMIDs, normalized
  manualBannerOnboarding: Record<string, 'confirmed' | 'not-confirmed'>;
  reviewModeShortcutEnabled: boolean;
}

type ListenerStatus =
  | 'not-requested'
  | 'proof-pending'
  | 'unavailable'
  | 'access-denied'
  | 'allowed'
  | 'polling'
  | 'revoked'
  | 'error';
```

Do not persist card bodies, action payloads, notification history, historical listener IDs, or a badge count. A settings migration must default safely to first-party enabled, third-party disabled, and no selected senders.

## 7. Geometry and lifecycle rules

The stack is placed at the bottom-right of the primary monitor's current **work area**, above the JasonShell bottom AppBar. Geometry must be derived from the actual work area and AppBar reservation, not from a hard-coded screen height.

Use exactly one coordinate authority for the bottom edge. The preferred authority is the primary monitor's current work area **after** the AppBar reservation. In that case, the stack bottom MUST be `workArea.bottom - approvedSafeGap`; the AppBar height MUST NOT be subtracted again. Only when the platform reports bounds that do not include the AppBar reservation may the implementation derive an equivalent work area as `monitorBounds.bottom - bottomAppBarHeight`, and that fallback MUST be identified in diagnostics/tests so both forms cannot be applied together.

The implementation must:

1. obtain primary-monitor position, physical work-area bounds, scale factor, and current bottom AppBar height;
2. convert logical card/spacing dimensions to physical window coordinates exactly once;
3. support negative monitor origins and non-zero work-area origins;
4. keep the full stack inside the right and bottom edges of the chosen coordinate authority, using `workArea.bottom - approvedSafeGap` when AppBar reservation is already included and never double-subtracting the AppBar height;
5. recompute on DPI, monitor/work-area, AppBar-height, display, and relevant window-size changes;
6. clamp if the work area is shorter or narrower than the preferred stack;
7. preserve card order and coordinator state across a reposition/reload;
8. handle lock, unlock, sleep/wake, fullscreen, shell restart, and hidden-window recovery without replaying stale cards;
9. prove pointer hit-testing and clipping at 100%, 150%, and 200% DPI.

The visible stack is a single primary-monitor surface. Do not silently move cards to another monitor or claim multi-monitor support. Fullscreen behavior must have a specified policy (default: hide/pause display without discarding current live state, then reconcile by expiry on return); this policy is a Phase 0 decision gate and must prevent stale replay after lock/fullscreen.

## 8. Visual, interaction, accessibility, and privacy design

### 8.1 Card design

Cards are rectangular with rounded edges, theme-aware background/border/shadow, readable typography, and controlled spacing. They must use shared `--js-*` tokens and the existing theme broadcast/synchronization path rather than hard-coded per-surface colors. A card contains:

- sender icon or safe fallback;
- sender label;
- title and sanitized body;
- relative age or time indicator;
- dismiss control;
- zero or more validated actions.

No remote image, HTML, CSS, script, or arbitrary markup is accepted. Text is rendered as text nodes. Long content is bounded and visibly truncated with an accessible expansion/review route only if explicitly approved.

### 8.2 Interaction

- Arrival MUST NOT focus, activate, steal foreground, or move keyboard focus.
- Pointer interaction MUST work on the card and controls without activating the window.
- Review mode is explicitly entered from a JasonShell control/shortcut or an accessible notification affordance; it may focus a card only after user intent.
- Review mode pauses expiry and supports keyboard navigation, dismiss, action invocation, and escape/close.
- Action execution is exactly once per action request/claim; repeated clicks, duplicate IPC messages, reconnects, and stale cards cannot execute twice.
- Hover pauses timers; leaving hover resumes from the remaining duration. Review mode supersedes timeout until it exits.

### 8.3 Accessibility

Use semantic roles and names for cards, sender/title/body, dismiss, and action buttons. Provide a non-focus-stealing live-region strategy for new first-party cards, with coalescing/rate limits to avoid an announcement storm. Meet tested contrast for every theme, visible keyboard focus in review mode, screen-reader names for icon-only controls, reduced-motion behavior, and pointer/keyboard parity. The card must remain useful when color and animation are unavailable.

### 8.4 Privacy and diagnostics

Card content and action payloads exist in memory only and are cleared on dismissal/expiry/restart. Persisted settings contain consent and sender selection only. Diagnostics may report counts, queue lengths, state transitions, status codes, normalized sender IDs where explicitly approved, and failure categories, but MUST redact title/body/action text and media. Debug logging must have an explicit safe mode and tests that reject content leakage.

## 9. Delivery phases

Every phase is a gate. “Done” means its artifacts and evidence exist; later phases must not smuggle unapproved behavior backward into an earlier gate.

### Phase 0 — Product contract and Windows listener proof

**Objective:** Freeze the supported first-party promise, validate the feasibility boundary for optional third-party mirroring, and approve defaults without modifying Windows settings.

**Tasks**

- Record the first-party and third-party lanes, non-goals, manual onboarding wording, privacy rules, primary-monitor boundary, and Phase 0 defaults.
- Confirm current Tauri 2 config/capability/permission files, Windows package identity, manifest capability declarations, signing/packaging path, and runtime initialization needed by `UserNotificationListener`; label every result as observed, missing, or to validate rather than assuming a config path.
- Build a disposable/package-based proof harness or narrowly scoped diagnostic path that requests listener access from the required UI thread/dispatcher, not the current spawned-worker request path.
- Prove package identity and manifest capability in the exact packaged artifact intended for testing.
- With explicit human consent and a disposable test sender, manually set a sender to banner OFF and Notification Center ON, confirm the user onboarding step, generate a **new** retained toast, and observe whether content and AUMID are actually visible to the listener.
- Test access denied, unavailable API, revocation, package mismatch, missing capability, UI-thread failure, poll failure, and empty/unsupported content.
- Define the event baseline: enabling a sender records observed IDs at the enable point; no pre-baseline IDs are replayed.

**Artifacts**

- reviewed requirements/decision record;
- packaged proof log with package identity, manifest/capability evidence, request-thread evidence, AUMID, content visibility, and timestamps;
- listener status matrix and failure taxonomy;
- approved or rejected Phase 0 defaults;
- manual onboarding copy and status UI contract.

**Tests/evidence**

- Static checks for identified Tauri config/capability/manifest locations and a test that fails closed when proof fields are absent.
- Rust/native unit tests for status mapping, baseline IDs, and redacted diagnostics.
- Packaged Windows proof for a new retained toast after manual banner-off/center-on onboarding.
- E2E: **required for the packaged listener proof only**, because package identity, OS permissions, UI-thread access, and content visibility cannot be established by unit tests. It is skipped for ordinary repository CI and must be recorded as a consent-gated manual evidence item.

**Exit gate**

The first-party lane may proceed regardless of the Phase 0 listener-proof outcome. The third-party mirroring lane is `verified` only when **every mandatory row** in the Phase 0 decision matrix is verified: package identity, manifest/capability, UI-thread access, user access, new content visibility, baseline correctness, settings-truth handling, and revoke/recover behavior. If any row is `failed` or `not-tested`, third-party mirroring remains `unavailable`; the selected-sender banner-off/Notification-Center-on proof is one required evidence element, not a sufficient outcome by itself.

**Unresolved decision gate**

Approve: (a) 4/50/8s/15s defaults, (b) hide-and-pause policy during fullscreen/lock, (c) whether an icon-only listener status should be visible to users, and (d) whether the optional mirror offers any action at all. No implementation of third-party mirroring starts without a `verified` result in the decision matrix below.

### Phase 1 — Contracts, bounded queue, and policy (RED-first)

**Objective:** Implement the source-independent notification domain and prove deterministic lifecycle behavior before opening a window.

**Tasks**

- Add typed contracts, normalization/sanitization, lane policy, sender allowlist policy, settings defaults/migration, and listener status model.
- Implement one coordinator with bounded visible/pending queues, deterministic ordering, deduplication, expiry, pause/resume, review mode, stale-event rejection, and snapshot generation.
- Define first-party producer API and adapter interface; keep third-party listener behind a capability/status gate.
- Implement action registry/validation and idempotent exactly-once claims.
- Implement content-free diagnostics and memory-only content ownership.

**Artifacts**

- shared contract types and versioning note;
- coordinator state machine and invariants;
- test fixtures for first-party, untrusted third-party, duplicate, stale, over-capacity, and expired inputs;
- settings schema/default/migration contract.

**Tests/evidence**

- RED-first unit tests for all coordinator transitions, queue bounds, expiry, pause, review, dedupe, sender policy, action claims, sanitization, persistence boundary, and listener unavailable behavior; then implementation to green.
- Property/table tests for 0, 1, 4, 5, 50, and 51 entries; duplicate event IDs; clock movement; malformed text and actions.
- E2E: skipped; no window or OS listener is needed. A deterministic coordinator trace is stronger and safer at this gate.

**Exit gate**

All domain tests pass; queue never exceeds 4 visible/50 pending; first-party inputs work with listener `unavailable`; every accepted action has one claim/result; no card content reaches persistence or diagnostics.

**Unresolved decision gate**

Approve overflow policy, time source/clock-jump handling, action failure presentation, and whether `acted` cards remain briefly visible or leave immediately.

### Phase 2 — Native stack window and geometry

**Objective:** Create a dedicated non-activating notification window and make placement safe across DPI/work-area/AppBar changes.

**Tasks**

- Add a distinct notification-stack label, builder, lifecycle registration, and initial hidden state in the native window setup.
- Validate the exact Tauri builder options and capability/permission requirements for this new window; do not treat existing popup configuration as proof.
- Add a dedicated Win32 style helper that preserves/adds `WS_EX_TOOLWINDOW` and `WS_EX_NOACTIVATE`; test it separately from the existing shell Alt+Tab helper.
- Implement primary work-area/AppBar geometry, negative-origin handling, logical/physical conversion, clamping, repositioning, and show/hide transitions.
- Prove pointer routing, no activation, z-order, taskbar exclusion, and controlled transparent/opaque host behavior.

**Artifacts**

- native stack window builder/style helper;
- pure geometry module and diagrams/examples for 100/150/200% DPI and negative origins;
- window lifecycle/recovery contract;
- packaged geometry evidence template.

**Tests/evidence**

- Rust unit tests for style bits and geometry with negative origins, reduced work areas, AppBar heights, DPI values, and clamping.
- Node/source contract tests for window label, hidden/default flags, and dedicated helper usage.
- E2E: **required for consent-gated Windows smoke**, because `WS_EX_NOACTIVATE`, focus, z-order, pointer hit-testing, and actual AppBar work area are OS/window-manager facts. Repository CI skips it.

**Exit gate**

Packaged smoke shows cards can be shown above the bottom AppBar without foreground activation at approved DPI levels; geometry remains in the primary work area after resize/DPI/AppBar changes; no existing popup behavior regresses.

**Unresolved decision gate**

Approve host transparency strategy, safe gap/card dimensions, and whether a native window resize is driven by exact stack bounds or a fixed maximum host.

### Phase 3 — Themed accessible renderer

**Objective:** Render safe cards that look native to JasonShell and remain usable without unsolicited focus.

**Tasks**

- Add notification surface routing/component loading deliberately through `shellSurface.ts`, `surfaceLoader.ts`, and `App.svelte`, or define an explicitly separate composition if the window does not use the normal route.
- Build card stack, sender/icon fallback, age, dismiss, actions, pending/review UI, empty/error/listener status states.
- Consume shared `--js-*` tokens and `installShellThemeSync`; verify theme changes propagate to the hidden stack without persistence of card content.
- Implement sanitized text-only rendering, bounded layout, keyboard review mode, focus indicators, reduced motion, live-region behavior, contrast, and screen-reader names.
- Implement IPC snapshot/event version checks and renderer recovery.

**Artifacts**

- Svelte notification stack/card components and styles;
- accessibility interaction contract and keyboard map;
- theme token mapping;
- renderer IPC adapter and fixture story/test data.

**Tests/evidence**

- Component/DOM tests for content bounds, escaping, card layout, theme sync, focus behavior, actions, review mode, reduced motion, and status states.
- Accessibility checks for semantic names, keyboard traversal, contrast tokens, and no automatic focus.
- E2E: **required for one packaged pointer/keyboard/theme smoke**; skipped in ordinary CI if the repository has no safe packaged WebView harness, with deterministic component tests retained.

**Exit gate**

The renderer can recover from a snapshot, never renders raw markup, never steals focus on arrival, responds to pointer and review keyboard controls, and visibly follows every supported theme.

**Unresolved decision gate**

Approve truncation/expansion rules, live-region announcement policy, card action visual order, and review-mode entry affordance.

### Phase 4 — First-party producers and settings

**Objective:** Ship the useful lane that does not depend on Windows notification APIs.

**Tasks**

- Add first-party producer entry points for initial JasonShell domains, each with stable IDs and declarative actions.
- Wire producer ingress through the coordinator and event/snapshot bridge; never let a producer write directly to the renderer.
- Add settings for master enable, first-party enable, review mode shortcut, timing/accessibility preferences if approved, and clear status explanation.
- Add manual onboarding UX as explanatory copy only; do not present first-party operation as dependent on Windows banner state.
- Handle app restart, window reload, action failure, fullscreen/lock transitions, and shutdown without persisting content.

**Artifacts**

- producer adapter list and ownership table;
- settings surface changes and migration tests;
- first-party notification catalog with action safety review;
- user-facing help/status copy.

**Tests/evidence**

- Unit/integration tests for each producer, stable identity, action authorization, lifecycle, and restart behavior.
- Node contract tests for settings persistence boundary and theme/IPC wiring; Rust tests for command/event validation.
- E2E: **required for first-party arrival-to-action smoke** once human consent is granted; otherwise skipped and clearly reported because live native window behavior cannot be inferred from unit tests.

**Exit gate**

A first-party producer delivers a themed card with no focus theft, bounded lifecycle, exactly-once action, review mode, and no listener permission. Settings drift or listener failure cannot disable the first-party lane.

**Unresolved decision gate**

Approve the initial producer list, action allowlist, default notification severity/timing, and user control for muting individual first-party producer categories.

### Phase 5 — Opt-in third-party mirroring

**Objective:** Add narrowly scoped post-delivery mirroring only when Phase 0 proof supports it.

**Tasks**

- Keep mirroring disabled unless package identity, manifest capability, UI-thread access, and visible content are all `verified` for the packaged build.
- Request access on the required UI thread/dispatcher and map denied, revoked, unavailable, and poll-error states without blocking first-party notifications.
- On sender enable, establish a baseline of observed notification IDs. Process only newly observed notifications after that baseline; never replay historical Notification Center entries.
- Require explicit sender selection and normalized allowlisted AUMIDs; do not mirror every sender by default.
- Ingest event plus reconciliation polling only after support proof. Deduplicate `(AUMID, notification ID)` and bound caches.
- Sanitize content to text/icon/action-safe fields; reject unsupported or invisible content and record only redacted failure diagnostics.
- Present the manual onboarding/status matrix: user-confirmed banner OFF/center ON, not confirmed, unknown/drift-suspected, listener unavailable, access denied, revoked, and recovered.
- Keep the original Windows notification in Notification Center. Local card dismissal changes only the custom card.

**Artifacts**

- verified listener adapter and packaged evidence;
- sender selection/settings UX;
- baseline/reconciliation/deduplication design;
- sanitizer tests and status/onboarding matrix;
- explicit third-party support limitations in user docs.

**Tests/evidence**

- Rust adapter tests with fake listener sequences: baseline, new item, duplicate, missing ID/AUMID, content unavailable, revocation, recovery, and poll failure.
- Integration tests for allowlist and local-dismissal separation.
- E2E: **required and consent-gated** for a packaged manually onboarded sender with banner OFF/center ON, new retained toast, no historical replay, sender filtering, revocation/recovery, and original Notification Center preservation. If any Phase 0 proof is absent, this E2E is skipped and the lane remains unavailable.

**Exit gate**

Only verified packaged support enables the lane; selected senders mirror new post-baseline observations; no historical replay occurs; content is sanitized; local dismissal does not claim OS dismissal; first-party behavior is unaffected by listener failure.

**Unresolved decision gate**

Approve the initial allowlisted AUMIDs, whether sender labels/icons are trusted or normalized, the reconciliation interval, and the user-facing response when Windows settings drift cannot be machine-verified.

### Phase 6 — Hardening and release documentation

**Objective:** Make the feature safe to release, diagnosable, reversible, and accurately documented.

**Tasks**

- Run focused Node, TypeScript/Svelte, Rust, and source-contract gates for changed surfaces.
- Run security/privacy review of IPC, action registry, sanitization, persistence, logs, package/capability configuration, and listener data.
- Run adversarial queue/window/accessibility tests: burst arrivals, malformed content, clock jumps, lost events, renderer restart, DPI change, AppBar change, lock/fullscreen, revocation, and repeated actions.
- Add rollback/disable behavior for the stack window and third-party lane; default third-party mirroring off if evidence or configuration is invalid.
- Update only owned user/release docs after behavior is actually shipped; plans and research must not be promoted to current behavior.
- Produce release notes that clearly separate first-party supported behavior from optional third-party limitations and manual onboarding.

**Artifacts**

- test and evidence report;
- security/privacy/accessibility review records;
- packaged artifact identity and configuration evidence;
- release checklist, rollback instructions, and user help copy;
- known-limitations entry covering primary monitor, no interception, no badge parity, and settings non-verification.

**Tests/evidence**

- Full applicable automated validation from `package.json` for touched surfaces, plus focused docs/link/size checks.
- E2E: required only for the approved Windows smoke matrix and skipped when explicit consent is absent; no live shell smoke, package run, or Windows setting mutation is implied by this plan.

**Exit gate**

All release evidence is attached, privacy/accessibility/security findings are resolved or accepted, third-party defaults are safe, rollback is demonstrated, and user-facing docs make no unsupported claims.

**Unresolved decision gate**

Release owner approves the evidence package, supported Windows/package matrix, telemetry/logging policy, and whether third-party mirroring ships at all or remains an experimental disabled capability.

## 10. Phase 0 decision matrix

| Proof item | Required observation | If absent | First-party lane |
|---|---|---|---|
| Package identity | Packaged app has stable identity matching the tested artifact | Listener `unavailable`; record exact packaging gap | Unaffected |
| Manifest/capability | Required notification listener declaration is present and effective | Do not request/claim access; listener disabled | Unaffected |
| UI-thread access | Request is made through the required UI dispatcher/thread and result is observable | Treat current worker-thread approach as unverified; listener disabled | Unaffected |
| User access | User grants access in the tested packaged artifact | `access-denied` or `not-requested` | Unaffected |
| New content visibility | A newly generated retained toast yields usable AUMID and permitted visible content | No mirror; no badge/count inference | Unaffected |
| Baseline correctness | Existing IDs are excluded and post-enable new IDs are accepted | No third-party release until fixed | Unaffected |
| Settings truth | Only manual confirmation is available; no machine verification | Show `unknown`/`drift-suspected`, never claim guaranteed banner-off | Unaffected |
| Revoke/recover | Revocation disables mirroring and a later approved grant recovers safely | Keep disabled and explain status | Unaffected |

Decision values should be `verified`, `failed`, or `not-tested`, with evidence link, artifact hash/version, timestamp, and operator consent. Third-party lane may be `verified` only when all required rows are verified. A failure must not block Phase 1–4 first-party work.

## 11. Requirement traceability

The IDs referenced by the acceptance criteria are defined here. Each requirement is atomic, testable, and uses **MUST**/**MUST NOT** as the conformance rule. The phase list is the delivery owner; the AC list is the minimum evidence map.

### Functional requirements

| ID | Requirement | Delivery phases | Acceptance criteria |
|---|---|---|---|
| **FR-01** | JasonShell MUST accept valid first-party notification inputs and render them through the coordinator-owned stack without activating or focusing another application. | 1, 3, 4 | AC-01, AC-02 |
| **FR-02** | The native stack MUST use a dedicated hidden non-activating window and MUST place/reposition it within the primary-monitor work area above the bottom AppBar without double reservation. | 2, 3 | AC-03, AC-04, AC-18 |
| **FR-03** | The coordinator MUST enforce maximum visible/pending bounds, deterministic ordering, expiry, and the approved overflow policy. | 1, 4 | AC-05, AC-06, AC-16 |
| **FR-04** | The stack MUST provide explicit review mode with intentional focus, keyboard navigation, dismissal, and action invocation; arrival MUST NOT steal focus. | 1, 3, 4 | AC-01, AC-07 |
| **FR-05** | Notification content MUST be sanitized, bounded, text-rendered, and themed through shared JasonShell tokens; unsafe markup/media/actions MUST NOT be accepted. | 1, 3, 4, 5 | AC-08, AC-09 |
| **FR-06** | Card content and action payloads MUST remain memory-only, and persisted settings/diagnostics MUST contain only approved consent, sender-selection, preference, and redacted status data. | 1, 4, 6 | AC-10 |
| **FR-07** | Third-party mirroring MUST remain unavailable unless every mandatory Phase 0 proof-matrix row is verified; when enabled it MUST use selected allowlisted senders, a post-baseline stream, safe reconciliation, and explicit unavailable/revoked states. | 0, 1, 5, 6 | AC-11, AC-12, AC-13, AC-14, AC-15 |
| **FR-08** | Actions and dismissals MUST be coordinator-authorized and idempotent: a custom card dismissal MUST NOT claim to dismiss the original Windows notification. | 1, 4, 5 | AC-02, AC-14 |
| **FR-09** | Lock, fullscreen, sleep/wake, restart, and renderer recovery MUST reconcile live state without stale historical replay or duplicate visible cards. | 1, 2, 4, 5, 6 | AC-04, AC-15, AC-16, AC-17 |

### Non-functional requirements

| ID | Requirement | Delivery phases | Acceptance criteria |
|---|---|---|---|
| **NFR-01** | The notification surface MUST be non-activating and MUST preserve foreground focus on arrival, while its explicit review mode MAY focus only after user intent. | 2, 3, 4 | AC-01 |
| **NFR-02** | Geometry MUST be deterministic across negative origins, supported DPI values, work-area changes, and AppBar changes, with one coordinate authority and no double subtraction. | 2, 6 | AC-03, AC-04, AC-18 |
| **NFR-03** | Queue and lifecycle resource use MUST remain bounded by the approved 4-visible/50-pending limits and MUST NOT replay stale entries after environmental transitions. | 1, 2, 4 | AC-05, AC-06, AC-16 |
| **NFR-04** | Every supported JasonShell theme MUST update the stack through shared tokens without a restart and MUST retain readable contrast and interaction states. | 3, 4, 6 | AC-08 |
| **NFR-05** | The feature MUST fail closed for unsupported listener/content/action conditions and MUST NOT leak notification content through persistence, diagnostics, markup, remote media, or arbitrary execution. | 0, 1, 3, 5, 6 | AC-09, AC-10, AC-11, AC-15 |

## 12. Acceptance criteria

Each criterion is a future implementation test, not a claim about the current prototype.

### First-party and focus behavior

**AC-01 (FR-01, NFR-01)**  
**Given** the first-party lane is enabled and the shell has a foreground application, **when** a producer submits a valid notification, **then** a card appears in the JasonShell stack without changing foreground window, keyboard focus, activation state, or taskbar selection.

**AC-02 (FR-01, FR-08)**  
**Given** a valid first-party notification with a declared action, **when** the action is activated twice by rapid pointer/IPC duplication, **then** the action executes at most once and the second request receives an idempotent already-claimed/result response.

### Geometry and native window

**AC-03 (FR-02, NFR-02)**  
**Given** the primary monitor is at a negative origin and the bottom AppBar reserves its configured height, **when** the stack is shown at 200% DPI, **then** the right and bottom card bounds remain inside the physical primary work area and the lowest card is above the AppBar by the approved gap.

**AC-04 (FR-02)**  
**Given** the work area, DPI, or AppBar height changes, **when** the native layout event is processed, **then** the stack is repositioned/clamped without changing coordinator order or replaying expired cards.

**AC-18 (FR-02, NFR-02)**  
**Given** the reported primary work area already includes the bottom-AppBar reservation, **when** the stack anchors, **then** its bottom edge is exactly `workArea.bottom - approvedSafeGap` and the AppBar height is not subtracted again; a geometry test MUST fail if both reservations are applied.

### Bounded queue and lifecycle

**AC-05 (FR-03, NFR-03)**  
**Given** four visible cards and fifty pending cards, **when** another notification arrives, **then** visible count remains at most four, pending count remains at most fifty, and the documented overflow policy is applied deterministically.

**AC-06 (FR-03)**  
**Given** an informational card with no hover or review pause, **when** eight seconds elapse, **then** it expires once; **given** an actionable card, **when** fifteen seconds elapse, **then** it expires once unless hover/review pause is active.

**AC-07 (FR-04)**  
**Given** the user enters review mode, **when** the user navigates, dismisses, or invokes an action with the keyboard, **then** expiry is paused, focus is visible and intentional, and the same controls work without pointer input.

### Theme, content, and privacy

**AC-08 (FR-05, NFR-04)**  
**Given** any supported JasonShell theme is active, **when** the theme changes in another shell window, **then** the notification cards update their shared tokens without requiring a restart and without changing notification content.

**AC-09 (FR-05, NFR-05)**  
**Given** a body contains HTML-like text, control characters, oversized text, or an untrusted URL/action target, **when** it enters the coordinator, **then** it is bounded/sanitized or rejected and no markup/script/unsafe action is rendered or executed.

**AC-10 (FR-06, NFR-05)**  
**Given** a card is dismissed, expired, or the app restarts, **when** settings and diagnostics are inspected, **then** body text, media, and action payloads are absent; only approved consent, sender selection, safe preferences, and redacted diagnostics remain.

### Listener availability and manual onboarding

**AC-11 (FR-07)**  
**Given** package identity, capability, UI-thread access, or content visibility proof is absent, **when** the app starts, **then** the listener is reported unavailable/unsupported, no third-party mirror is enabled, and first-party notifications still work.

**AC-12 (FR-07)**  
**Given** a user selects a sender, manually turns its Windows banners OFF while retaining Notification Center, and confirms the onboarding step, **when** JasonShell records settings, **then** it records consent/selection only and does not mutate or claim machine verification of Windows settings.

**AC-13 (FR-07)**  
**Given** third-party mirroring is enabled for a sender, **when** the listener baseline is established, **then** notifications observed before that baseline are not mirrored and a newly observed post-baseline notification is eligible only if its AUMID is selected/allowlisted.

**AC-14 (FR-07, FR-08)**  
**Given** a mirrored card exists while its original remains in Notification Center, **when** the user dismisses the JasonShell card, **then** only the custom card is removed and JasonShell makes no claim that the original OS notification was dismissed.

**AC-15 (FR-07)**  
**Given** listener access is revoked or polling fails, **when** status changes, **then** mirroring pauses with an explicit status, no stale historical replay occurs on recovery, and first-party delivery remains available.

### Environment and recovery

**AC-16 (FR-09, NFR-03)**  
**Given** the shell enters lock/fullscreen and later returns, **when** the stack is reconciled, **then** expired cards are not replayed, paused/live state follows the approved policy, and no stale duplicate is shown.

**AC-17 (FR-09)**  
**Given** the notification WebView reloads or misses an IPC event, **when** it requests a snapshot, **then** the coordinator returns the current bounded state with revisions and the renderer converges without duplicate actions or cards.

## 13. Test strategy and code-path map

### Existing paths to extend or verify

- `src/lib/shellSurface.ts`, `src/lib/surfaceLoader.ts`, `src/App.svelte`: deliberate surface label, metadata, loader, and theme/preferences lifecycle integration.
- `src/lib/themes.ts`: shared theme IDs, token application, and broadcast synchronization.
- `src-tauri/src/shell_windows.rs`: dedicated window builder, label, hidden lifecycle, and separate `WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE` helper; do not alter the semantics of the current shell Alt+Tab helper without a separate decision.
- `src-tauri/src/task_windows/notifications.rs`: retain as a legacy/counting reference only until a new listener adapter is proven; do not infer mirror support from its current polling behavior.
- `src-tauri/src/main.rs` and Tauri configuration/capability/manifest files: validate exact command/event/window permissions and package identity paths in Phase 0.

### Likely new or touched files (implementation planning only)

- `src/lib/notifications/notificationContracts.ts` — shared types, limits, sanitization boundary.
- `src/lib/notifications/notificationCoordinator.ts` — pure bounded state machine/queue.
- `src/lib/notifications/notificationIpc.ts` — typed event/snapshot/action bridge.
- `src/lib/notifications/notificationSettings.ts` — versioned safe settings only.
- `src/components/NotificationStackSurface.svelte` and `src/components/NotificationCard.svelte` — themed accessible renderer.
- `src-tauri/src/notifications/` — coordinator bridge, first-party ingress, listener adapter, diagnostics, and tests, if module boundaries are approved.
- `src-tauri/src/shell_windows.rs` — notification window construction and geometry hooks.
- `tests/notificationContracts.test.mjs`, `tests/notificationCoordinator.test.mjs`, `tests/notificationSurface.test.mjs`, `tests/notificationWindowContract.test.mjs` — Node/source/component contracts as appropriate.
- Rust unit/integration tests adjacent to the new native modules.
- Exact Tauri capability/manifest/config file(s) — **path and changes to validate in Phase 0; not asserted as existing here**.

### Evidence map

| Claim | Best evidence | Limitation |
|---|---|---|
| Queue is bounded and deterministic | Pure coordinator unit/property tests | Does not prove WebView timing |
| Actions execute once | Coordinator/IPC idempotency tests plus packaged smoke | OS target side effects need safe test targets |
| No focus theft and correct z-order | Packaged Win32 smoke with focus/window inspection | Requires consent and Windows |
| Bottom-AppBar/DPI geometry | Pure geometry vectors plus 100/150/200% packaged smoke | Multi-monitor excluded |
| Theme/accessibility | Component tests, contrast checks, keyboard/screen-reader review | Automated checks do not replace user review |
| No content persistence | Serialization/log redaction tests and filesystem inspection of approved settings | Must inspect release logging configuration |
| Listener support | Packaged proof with identity/capability/thread/content evidence | OS/package/version dependent |
| No historical replay | Baseline fixture tests plus fresh packaged sender run | Requires a controlled test sender |

## 14. Phased implementation checklist

- [ ] Phase 0 requirements and defaults reviewed; status remains Draft until approved.
- [ ] Phase 0 packaged listener proof complete or third-party lane explicitly marked unavailable.
- [ ] Exact Tauri capability/manifest/package paths identified and evidence attached.
- [ ] Phase 1 RED tests written before coordinator implementation.
- [ ] Contract, sanitizer, settings, queue, expiry, pause, review, and exactly-once tests green.
- [ ] Phase 2 dedicated window/style helper and geometry vectors complete.
- [ ] Phase 2 consent-gated no-focus/DPI/AppBar smoke complete or recorded as pending.
- [ ] Phase 3 themed renderer, IPC recovery, accessibility, and content safety complete.
- [ ] Phase 4 first-party producers and settings complete without listener dependency.
- [ ] Phase 5 remains disabled unless all listener proof rows pass; if enabled, baseline/allowlist/revocation/recovery evidence complete.
- [ ] Phase 6 security, privacy, accessibility, adversarial, release, and rollback reviews complete.
- [ ] User docs describe manual onboarding and limitations without claiming shipped behavior before release.

## 15. Risk table

| Risk | Impact | Mitigation | Stop condition |
|---|---|---|---|
| Windows listener requires package/capability conditions not met | False mirror promise or silent data loss | Fail closed; Phase 0 proof; first-party independent | Any required proof row absent |
| Current worker-thread access request is invalid | Listener appears to work inconsistently | UI-thread/dispatcher proof and explicit status mapping | No repeatable packaged access result |
| Windows banner setting cannot be verified | User believes only JasonShell cards show | Manual onboarding, honest status matrix, no machine-verification claim | Product copy implies guarantee |
| Window activates or steals focus | Disruptive shell behavior | Dedicated `WS_EX_NOACTIVATE` helper and packaged focus tests | Any unsolicited activation/focus |
| Transparent host captures/clips incorrectly | Invisible click blockers or broken rounded cards | Prefer controlled host; test pointer/clipping before release | Pointer/clip behavior differs by DPI/WebView |
| Burst notifications overwhelm user | Missing or unusable cards | 4/50 bounds, deterministic overflow, review mode | Queue grows unbounded or drops visible/actionable state |
| Duplicate IPC/action messages | Repeated destructive action | Coordinator claims and request IDs | Action cannot be proven exactly once |
| Untrusted third-party content reaches WebView/action executor | XSS/unsafe execution/privacy exposure | Text-only sanitizer, typed allowlist, no remote media/HTML | Any raw markup or arbitrary target accepted |
| Card content leaks to disk/logs | Privacy breach | Memory-only cards, redacted diagnostics, persistence tests | Body/action payload found outside memory |
| Fullscreen/lock/restart replays stale content | User confusion/privacy issue | Reconcile expiry, baseline and revision rules | Stale card shown after return |
| Theme tokens drift between surfaces | Inconsistent visual product | Shared tokens/broadcast and theme contract tests | A supported theme leaves notification card unreadable |
| Scope expands to badges/multi-monitor/interception | Unbounded project and unsupported claim | Explicit non-goals and review gates | Requirement depends on private Explorer behavior |

## 16. Manual test consent boundary

Do not run live Tauri/Windows shell smoke, packaged listener proof, fullscreen/lock/DPI/AppBar tests, or any test that changes Windows notification settings without explicit human consent for that run. Do not mutate Windows settings as part of automated validation. Docs-only work for this plan must not start the shell, build/package the app, request listener permission, or claim runtime evidence.

When consent is later granted, use a disposable sender/test account, record the exact artifact/version and Windows environment, provide rollback instructions, restore manually changed notification settings, and capture only redacted evidence. A passed static or unit test is not evidence that Windows is displaying, suppressing, or exposing a notification as expected.

## 17. Release evidence and definition of done

### Required release evidence

- approved product contract and Phase 0 decision matrix;
- packaged artifact identity, manifest/capability/configuration evidence, and listener status result;
- automated contract/coordinator/native/style/geometry/component/accessibility/privacy tests;
- focused packaged smoke evidence for no focus theft, AppBar/DPI placement, review/action behavior, theme sync, and recovery, where consented;
- first-party producer catalog and action safety review;
- if shipped, third-party sender allowlist, baseline/no-replay evidence, sanitizer tests, revocation/recovery evidence, and manual onboarding/status screenshots or logs with content redacted;
- security/privacy/accessibility review and accepted residual-risk list;
- rollback/disable procedure and user-facing limitations.

### Definition of done

The feature is done only when:

1. First-party notifications work end to end on the primary monitor without Windows listener support, focus theft, unbounded queues, duplicate actions, unsafe content, or notification-content persistence.
2. The stack is a dedicated themed, accessible, rounded-card surface above the bottom AppBar and follows approved DPI/work-area/recovery rules.
3. Settings store consent/sender selection only; JasonShell never changes Windows notification policy/settings.
4. All required automated tests and focused evidence pass, with live/manual gaps explicitly reported.
5. Third-party mirroring is either proven and released exactly within the Phase 5 limits or remains disabled and documented as unavailable; it is never implied by first-party completion.
6. Release docs distinguish supported product behavior from non-goals: no pre-display interception, no guaranteed banner suppression, no machine-verifiable setting state, no Explorer badge parity, and no multi-monitor support.
