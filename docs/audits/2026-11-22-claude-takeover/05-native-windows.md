## Native Windows, robustness & security audit

Scope: JasonShell `src-tauri` native layer (Tauri 2.10.3 / tauri-runtime-wry 2.10.1 / vendored wry 0.54.4 / windows-rs 0.61). Read-only review. No build, run or smoke executed. Lane: Win32 correctness, robustness, shell-replacement completeness, unsafe and security. Rust thread/perf and IPC ACL layers are covered by other audits and only cross-referenced here.

Severity scale: Critical / High / Medium / Low. Effort: S (< 1 day), M (1-5 days), L (> 1 week).

### Executive summary

JasonShell is not currently a shell replacement. It is a pair of WebView2 AppBars layered on top of a still-running `explorer.exe`, with Explorer's own taskbar hidden through `ShowWindow(SW_HIDE)`. Most "shell" features are delegated to Explorer, either explicitly (pinned-app folder, tray toolbar scraping, Start menu, Win+ shortcuts, ITaskbarList, toasts) or implicitly (`SHAppBarMessage` only works while Explorer's `Shell_TrayWnd` is alive). The AppBar protocol itself is implemented only half way: a callback message is registered but never received, so every `ABN_*` notification is lost, and display, DPI, Explorer-restart and session changes are not handled at all. Geometry is captured once at startup.

The most important robustness gaps are crash recovery (no panic hook, no persistent "taskbar hidden" marker, console-subsystem binary whose console can be closed) and several Win32 threading hazards. These are a lock held across cross-thread `SetWindowPos`, blocking Win32 calls (UAC waits, `AttachThreadInput`, `SendMessageTimeout`) inside synchronous Tauri commands that run on the UI thread, and infinite `Unhook*` retry loops.

The most important security issues are:
- **SEC-01.** A command-injection path in "Open terminal here" (PowerShell `-Command` with appended directory text).
- **SEC-02.** Everything SDK DLL loading from the current working directory and its ancestors (and from an env var).
- **SEC-03.** Unhardened `git` invocation on every browsed folder, which allows repo-config code execution through `core.fsmonitor` and filters.
- **SEC-04.** A taskbar "Close" that silently escalates to `TerminateProcess`, which loses unsaved work.

Good practice exists and should be preserved:
- Handle-based, reparse-aware local-drop rename (`stack_popup/local_drop.rs`).
- BatBadBut-safe quick-command launching (`quick_commands/windows_launch.rs`).
- Identity-revalidated process termination (`process_manager.rs:826-877`).
- RAII DWM thumbnail handles (`task_preview.rs:90`, `task_gallery/thumbnails.rs:96`).
- An LL keyboard hook that does minimal work in the callback (`windows_key_hook.rs:205-237`).

### Unsafe code inventory

| Metric | Value | Evidence |
|---|---|---|
| `unsafe {` blocks | 840 | `grep "unsafe {"` over `src-tauri/src` |
| `unsafe fn` / `unsafe extern "system" fn` | 45 | same |
| `unsafe impl Send/Sync` | 5 | `quick_commands.rs:312,314`, `snipping/native.rs:203`, `text_document/feasibility/native.rs:447-448` |
| Files containing `unsafe` | 66 | |
| `SAFETY` comments | 107 total (about 1 per 8 blocks) | |
| Files with zero SAFETY comments despite heavy unsafe | `appbar.rs` (20), `explorer.rs` (12), `windows_key_hook.rs` (28), `task_windows/actions.rs` (37), `task_windows/native_hooks.rs` (27), `launchers.rs` (30), `process_manager.rs` (37), `audio.rs` (55) | per-file grep counts |
| Hand-declared externs | `CryptProtectData`/`BCryptGenRandom` (`text_document/feasibility/native.rs:205,226`), `GetDpiForMonitor` (`snipping/native.rs:19`), `GetDpiForWindow` (`snipping/windows.rs:9`), `DeregisterShellHookWindow` (`native_hooks.rs:671-675`), `NtQueryInformationProcess` (`process_manager.rs:1237`), hand-built `IPolicyConfig` vtable (`audio.rs:196-280`) | |

`#![deny(clashing_extern_declarations)]` is set (`main.rs:2`), which is good. However, `clippy::undocumented_unsafe_blocks` and `unsafe_op_in_unsafe_fn` are not enforced.

---

### Findings: Win32 correctness and robustness (WIN-xx)

- **WIN-01: AppBar callback message registered but never received, so all ABN_* notifications are lost** (High, M)
  - Evidence: `appbar.rs:974-983`. `new_appbar_data` sets `uCallbackMessage = RegisterWindowMessageW("JasonShell.AppBar")`. No `SetWindowSubclass`, `GWLP_WNDPROC` or tao message hook exists anywhere in `src` (grep for `SetWindowSubclass|GWLP_WNDPROC|DefSubclassProc` finds nothing). The bar HWNDs are plain Tauri/tao windows whose WndProc discards unknown registered messages. In addition, `ABM_ACTIVATE` and `ABM_WINDOWPOSCHANGED` are never sent (only `ABM_NEW/QUERYPOS/SETPOS/REMOVE` are imported at `appbar.rs:24`).
  - Problem: Explorer communicates through this message:
    - `ABN_POSCHANGED`: another appbar, taskbar or resolution change.
    - `ABN_FULLSCREENAPP`: a rude/fullscreen app opened or closed.
    - `ABN_STATECHANGE`: auto-hide or always-on-top changed.
    - `ABN_WINDOWARRANGE`: cascade/tile.

    None of these reach JasonShell. That forces the 250 ms fullscreen polling thread (WIN-08) and leaves the reserved rectangles stale after any external change. Per the AppBar contract, the appbar must also forward `WM_ACTIVATE` to `ABM_ACTIVATE` and `WM_WINDOWPOSCHANGED` to `ABM_WINDOWPOSCHANGED` so Explorer keeps z-order right.
  - Recommendation:
    1. Subclass both bar HWNDs with `SetWindowSubclass` (comctl32 v6 is already in the manifest), or use a dedicated hidden owner/message HWND per bar created on the main thread.
    2. Handle `uCallbackMessage` to drive re-reservation: `ABN_POSCHANGED` → `ABM_QUERYPOS`/`ABM_SETPOS` + `SetWindowPos`; `ABN_FULLSCREENAPP` (lParam TRUE/FALSE) → hide/restore.
    3. Forward `WM_ACTIVATE` → `ABM_ACTIVATE` and `WM_WINDOWPOSCHANGED` → `ABM_WINDOWPOSCHANGED`.
    4. Call `ABM_REMOVE` from `WM_DESTROY`.

- **WIN-02: No TaskbarCreated handling, so an Explorer restart drops AppBar reservations permanently** (High, S)
  - Evidence: grep for `TaskbarCreated` returns nothing. The only native message window (`task_windows/native_hooks.rs:276-311,600-623`) handles `WM_DISPLAYCHANGE` (env-gated) and `SHELLHOOK` only.
  - Problem:
    - The AppBar list lives inside `explorer.exe`. When Explorer crashes or restarts (common: Explorer updates, "Restart Explorer", shell extension crashes), the new instance knows nothing about JasonShell's bars. The work area resets, maximized windows cover the top and bottom bars, and the bars become plain topmost overlays.
    - The legacy taskbar guard re-hides the new `Shell_TrayWnd` by polling (`appbar.rs:2066-2086`), but nothing re-runs `ABM_NEW`/`ABM_SETPOS`.
    - `state.registered_appbars` still lists the old HWNDs, so later `ABM_REMOVE` calls fail and block re-registration (`appbar.rs:903-937`).
  - Recommendation:
    - `RegisterWindowMessageW(L"TaskbarCreated")` in a top-level (not `HWND_MESSAGE`) window.
    - On receipt: clear tracked registrations without `ABM_REMOVE`, re-run `register_appbar` + `reserve_appbar` + `move_window_to_rect`, re-snapshot and re-hide `Shell_TrayWnd`, re-discover tray toolbars, and call `RegisterShellHookWindow` again.
    - Because medium-IL processes need it after elevation boundaries, also call `ChangeWindowMessageFilterEx(hwnd, taskbarCreated, MSGFLT_ALLOW)`.

- **WIN-03: Monitor geometry, DPI and work area captured once at startup; no WM_DISPLAYCHANGE / WM_DPICHANGED / WM_SETTINGCHANGE handling** (High, M)
  - Evidence:
    - `appbar.rs:260-276`: `app.primary_monitor()` rect and `scale_factor()` are read once into `ShellSurfaceLayout.monitor_rect`. It is never refreshed (the only writes are `commit_activation_plan` and `restore_shell_after_fullscreen`/`resize_shell_bar_runtime`, which keep `..layout`).
    - `WM_DISPLAYCHANGE` is only handled when `JASONSHELL_EXPLORER_SUPPRESSION_V2=1`, and only to reconcile the taskbar (`native_hooks.rs:601-604`).
    - `WM_DPICHANGED` appears only in a snipping comment (`snipping/native.rs:117`).
    - `shell_windows.rs:110-139,162-212` likewise compute everything from the startup primary monitor.
  - Problem: Changing resolution, scaling, orientation or the primary monitor, docking/undocking a laptop, connecting a projector, or RDP reconnecting at a different size all leave the bars at stale physical coordinates, with a stale AppBar reservation and stale `SPI_SETWORKAREA`. When the user changes DPI, tao applies its own `WM_DPICHANGED` suggested rect to the bar windows, so the HWND size diverges from the AppBar reservation. Bars can end up off-screen or mid-screen.
  - Recommendation:
    - Handle `WM_DISPLAYCHANGE`, `WM_DPICHANGED` (use the suggested RECT only as a hint; recompute physical thickness from logical height × `GetDpiForMonitor(MDT_EFFECTIVE_DPI)/96`), and `WM_SETTINGCHANGE(SPI_SETWORKAREA)`.
    - Debounce them (displays fire bursts) and re-run a single idempotent `relayout()`: `MonitorFromWindow(MONITOR_DEFAULTTOPRIMARY)` → `GetMonitorInfoW` → `ABM_QUERYPOS/ABM_SETPOS` → `SetWindowPos`.
    - Store the `HMONITOR` and device name (`MONITORINFOEXW.szDevice`) instead of a bare RECT.

- **WIN-04: Crash leaves Explorer taskbar hidden, and the next clean run never restores it** (Critical, S)
  - Evidence:
    - `explorer.rs:197-205`: `hide_primary_taskbar_if_needed` returns the snapshot unchanged if `!snapshot.originally_visible`.
    - `explorer.rs:471-474`: `restore_taskbar` returns `Ok(true)` without showing when `!snapshot.originally_visible`.
    - `explorer.rs:445-447,456-458`: v2 also skips "originally hidden" taskbars.
    - No panic hook (`grep set_hook` → none), no minidump/WER registration, and no persisted "hidden-by-JasonShell" marker exist.
  - Problem: If JasonShell dies by panic-abort, access violation in WebView2/COM, Task Manager kill, logoff race, or the console-close case in WIN-05, `Shell_TrayWnd` stays `SW_HIDE`. On the next launch it is classified as "originally hidden", so JasonShell deliberately never shows it again, even on a clean exit. The user is left with no taskbar until Explorer restarts or they log off. Unit test names confirm the baseline-work-area recovery path exists (`appbar.rs:2472`), but visibility is not recovered.
  - Recommendation:
    1. Persist a marker under `%LOCALAPPDATA%\JasonShell\state\taskbar-hidden.json` (HWND class, pid, process start time) before hiding, and delete it after restore. On startup, if the marker exists, treat the taskbar as originally visible.
    2. Install `std::panic::set_hook` that synchronously does `ShowWindow(FindWindowW("Shell_TrayWnd"), SW_SHOW)` + `ABM_REMOVE` for both bars + restores the baseline work area, then aborts.
    3. Add `SetUnhandledExceptionFilter`/`AddVectoredExceptionHandler` for SEH crashes with the same best-effort restore.
    4. Add an out-of-process watchdog (see Big bets): a tiny helper that waits on the shell process handle and restores Explorer on abnormal exit.

- **WIN-05: Release binary is a console-subsystem executable** (High, S)
  - Evidence: `main.rs:1-2` has no `#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]` (grep `windows_subsystem` → none). Many `println!`/`eprintln!` diagnostics are on production paths (`native_hooks.rs:386-390,569`; `appbar.rs:1624-1628`; 56 non-test sites in total).
  - Problem: Launched outside a terminal, Windows allocates a console window next to the shell. Closing it delivers `CTRL_CLOSE_EVENT`, and the process is killed about 5 s later with no cleanup (no `SetConsoleCtrlHandler`). That triggers WIN-04 (taskbar stuck hidden, work area stuck). It also looks broken to users and leaks diagnostics to anyone who can see the console.
  - Recommendation: Add `windows_subsystem = "windows"` for release builds. Route logs to a rotating file under `%LOCALAPPDATA%\JasonShell\logs` (`tracing` + `tracing-appender`). If a console is ever attached, install `SetConsoleCtrlHandler` that triggers the graceful exit path.

- **WIN-06: Fullscreen guard holds the shell state Mutex across cross-thread SetWindowPos, which can deadlock the UI thread** (High, S)
  - Evidence:
    - `appbar.rs:1741-1781`: `hide_shell_for_fullscreen` runs on the `fullscreen_guard` worker thread (`appbar.rs:1602`). It keeps the `MutexGuard` `state` alive while calling `unregister_appbar` (a SendMessage to Explorer), `sync_work_area_best_effort_with` (`SPI_SETWORKAREA` + `SPIF_SENDCHANGE` broadcast) and `park_shell_surface_windows` → `SetWindowPos` on HWNDs owned by the main thread (`appbar.rs:1211-1233`).
    - `restore_shell_after_fullscreen` similarly holds the lock across `register_appbar`/`reserve_appbar` (`appbar.rs:1799-1878`).
    - Meanwhile `resize_shell_bar` is a synchronous `#[tauri::command]` (`appbar.rs:439-445`). Tauri 2 runs non-async commands on the main thread, and it calls `state.lock()` (blocking) at `appbar.rs:479`.
  - Problem: Cross-thread `SetWindowPos` without `SWP_ASYNCWINDOWPOS` synchronously sends `WM_WINDOWPOSCHANGING/CHANGED` to the owning (main) thread and waits. If the main thread is blocked in `state.lock()` because the user is dragging the bar resize handle while a fullscreen transition fires, neither side can progress and the entire shell UI hangs. `SPIF_SENDCHANGE` broadcasting `WM_SETTINGCHANGE` to our own top-level windows from a lock-holding thread has the same hazard.
  - Recommendation:
    - Never hold `ShellRuntimeState` across any call that can send messages to another thread or process. Copy out the needed state, drop the guard, perform the side effects, then re-lock and validate a generation counter.
    - Use `SWP_ASYNCWINDOWPOS` for all SetWindowPos calls on shell HWNDs from worker threads, or marshal geometry changes onto the main thread with `AppHandle::run_on_main_thread`.
    - Make `resize_shell_bar` `async` / `spawn_blocking` so it does not block the event loop.

- **WIN-07: Blocking Win32 work inside synchronous Tauri commands on the UI thread (UAC waits, AttachThreadInput, SendMessageTimeout, sleeps)** (High, M)
  - Evidence: There are 230 synchronous `#[tauri::command]` functions and 0 `#[tauri::command(async)]` (grep). Examples:
    - `task_windows/mod.rs:225-231` `close_task_window` → `actions.rs:283-332`: `SendMessageTimeoutW(WM_CLOSE, 750 ms)`, then `wait_for_window_close` (10 × 50 ms), then `PostMessage` + wait, then `TerminateProcess` + wait, or `elevate_close_target` → `shell_execute_runas_wait` + `wait_for_process_exit(.., 8 s)` (`task_windows/windows.rs:445-470`). The UI thread is blocked while the UAC consent prompt is open.
    - `task_windows/mod.rs:193-194` `activate_task_window` → `actions.rs:90-173`: `wait_for_windows_restore` (up to 500 ms of `thread::sleep`), plus `AttachThreadInput(current_thread, foreground_thread, true)` and the target thread, called from the main thread.
    - `system_tray.rs:108-153`: synchronous `list_system_tray_icons`/`invoke_system_tray_icon` with cross-process `VirtualAllocEx`/`SendMessageTimeoutW` into Explorer.
    - `shell_paths.rs:3-23,276-300`: `ShellExecuteW` synchronously, which can block for seconds on UNC paths, offline network drives or slow shell extensions.
    - `resize_shell_bar`: `set_work_area_with_retry` sleeps up to 20 × 50 ms (`appbar.rs:1046-1073`).
  - Problem: The UI thread is the tao event loop for every bar and panel. While blocked, no surface repaints or receives input. `AttachThreadInput` with a hung app's thread can wedge the shell's input queue, the classic shell hang. Explorer avoids this by doing these operations on worker threads.
  - Recommendation:
    - Mark these commands `async` and use `tauri::async_runtime::spawn_blocking`. Run COM work on a dedicated STA worker (one long-lived thread, not a thread per call).
    - Replace the `AttachThreadInput` dance with `AllowSetForegroundWindow`/`SetForegroundWindow`. The shell is usually the foreground process when the user clicks the bar, so it holds foreground rights. Fall back to `SwitchToThisWindow` only, and never attach to the foreground thread from the UI thread.
    - Use `ShellExecuteExW` with `SEE_MASK_ASYNCOK` and `SEE_MASK_FLAG_NO_UI` on a worker.

- **WIN-08: Fullscreen detection is a 250 ms polling heuristic instead of ABN_FULLSCREENAPP / SHQueryUserNotificationState** (Medium, M)
  - Evidence: `appbar.rs:166,1597-1635` (poll loop), `appbar.rs:1951-1999` (heuristic). It hides the shell when the foreground window covers the monitor, or when a frameless window covers the work area.
  - Problem:
    - Constant wakeups (4 Hz forever, including while the session is locked or disconnected).
    - False positives: any borderless window sized exactly to the work area (some media players, custom-chrome apps, kiosk apps) releases the AppBars. The work area then grows, the app can maximize into it, and the bars stay hidden.
    - False negatives: exclusive-mode D3D, presentation mode, and "rude" windows on the primary monitor that do not match the rect test.
    - No per-monitor handling: a fullscreen video on a secondary monitor leaves the bars alone, which is correct, but a fullscreen app spanning monitors is unhandled.
  - Recommendation: Drive the state machine from `ABN_FULLSCREENAPP` (WIN-01). Corroborate with `SHQueryUserNotificationState` (`QUNS_RUNNING_D3D_FULL_SCREEN`, `QUNS_PRESENTATION_MODE`, `QUNS_BUSY`) and an `EVENT_SYSTEM_FOREGROUND` + `EVENT_OBJECT_LOCATIONCHANGE` (filtered to the foreground hwnd) WinEvent instead of polling. Use `MonitorFromWindow` of the foreground window to scope decisions per monitor.

- **WIN-09: Legacy taskbar guard polls EnumWindows every 100 ms forever and uses synchronous cross-process ShowWindow** (Medium, S)
  - Evidence:
    - `appbar.rs:2066-2086`: `thread::sleep(100 ms)` loop calling `explorer::reconcile_primary_taskbar_ownership` → `EnumWindows` (`explorer.rs:115-127`).
    - `enum_taskbars` allocates two UTF-16 `Vec`s per top-level window per call (`explorer.rs:540-555`).
    - `set_taskbar_visibility` → `ShowWindow(taskbar, SW_HIDE)` with retries and sleeps (`explorer.rs:517-534`).
    - The event-driven v2 path is opt-in only via `JASONSHELL_EXPLORER_SUPPRESSION_V2=1` (`appbar.rs:2112-2114`).
  - Problem: 10 Hz enumeration of every top-level window is measurable CPU and battery drain. `ShowWindow` on another process's window is synchronous. If Explorer hangs, the guard thread blocks while holding the `owned` mutex, and `stop_taskbar_guard` (`appbar.rs:2116-2131`) `join()`s it, so the cleanup lane stalls until the 3 s hard-exit.
  - Recommendation: Make the event-driven v2 path the default (`EVENT_OBJECT_SHOW` on `Shell_TrayWnd` plus `TaskbarCreated`). Use `ShowWindowAsync` for foreign windows. Precompute the class-name UTF-16 constants (`w!("Shell_TrayWnd")`). Better still, stop fighting Explorer by hiding its taskbar through supported means (see Big bets).

- **WIN-10: Infinite Unhook*/UnregisterClass retry loops can hang shutdown forever** (Medium, S)
  - Evidence:
    - `windows_key_hook.rs:1004`: `while UnhookWindowsHookEx(hook).is_err() { sleep(25ms) }`.
    - `windows_key_hook.rs:1008-1015`: `UnregisterHotKey` loops on any error other than `ERROR_HOTKEY_NOT_REGISTERED`.
    - `snipping/escape.rs:128` (same `UnhookWindowsHookEx` loop).
    - `snipping/foreground.rs:142`: `while !UnhookWinEvent(hook)`.
    - `task_windows/native_hooks.rs:558-568`: `UnhookWinEvent`, `DestroyWindow` and `UnregisterClassW` retry loops.
  - Problem: On Windows 7 and later, a WH_KEYBOARD_LL hook that exceeds `LowLevelHooksTimeout` is silently removed by the system. After that, `UnhookWindowsHookEx` fails with `ERROR_INVALID_HOOK_HANDLE` on every retry, so the loop never terminates. `UnregisterClassW` fails permanently if any window of the class still exists on another thread. These loops run on worker threads whose `JoinHandle`s are joined during drain, so the graceful path degrades into the 3 s `TerminateProcess` every time.
  - Recommendation: Treat `ERROR_INVALID_HOOK_HANDLE` / `ERROR_INVALID_PARAMETER` as already released. Bound retries (for example 3 attempts) and log. Wrap hooks in RAII types whose `Drop` makes one attempt.

- **WIN-11: LL keyboard hook thread also runs settings/registration work and app emission; silent hook removal is undetected** (Medium, M)
  - Evidence:
    - The WH_KEYBOARD_LL hook is installed on the same thread that runs the hotkey message loop (`windows_key_hook.rs:885-1004`).
    - That loop executes arbitrary `request.run` closures (`windows_key_hook.rs:953-961`, including `RegisterHotKey` sets) and `dispatch_hotkey`.
    - `dispatch_hotkey` performs `prepare_speech_paste_target()` and several `app.emit_to(...)` calls on the hook thread (`windows_key_hook.rs:823-882`).
  - Problem: While the thread is busy, LL callbacks queue up. If a callback is not serviced within `LowLevelHooksTimeout` (registry default 300 ms on many systems, max 1 s on Win10+), Windows silently unhooks it. Search (Ctrl/Alt+Space), which is delivered only through the LL hook (`windows_key_hook.rs:833-836` deliberately ignores `WM_HOTKEY` for Search), then stops working with no health signal.
  - Recommendation: Put the LL hook alone on a dedicated thread whose loop does nothing but `GetMessageW`. Post work to another thread. Add a liveness probe, for example a periodic self-injected marker or tracking last-callback time against `GetLastInputInfo`, and reinstall the hook when it goes stale. Use `WM_HOTKEY` as the fallback trigger for Search.

- **WIN-12: Search and snipping-Escape LL hooks are blind while an elevated window has focus (UIPI)** (Medium, M)
  - Evidence: Search is triggered only by the LL hook (`windows_key_hook.rs:833-836`), and Escape for snipping is also an LL hook (`snipping/escape.rs:116`). The binary runs `asInvoker` (default Tauri manifest, see SEC-11) with no `uiAccess`.
  - Problem: A medium-IL process's low-level keyboard hook does not observe keystrokes delivered to higher-IL foreground windows. This is the well-known UIPI behavior that also breaks AutoHotkey in admin windows. With an elevated terminal, Task Manager, regedit or installer focused, the Search hotkey does nothing. `RegisterHotKey` would have worked, because hot keys are delivered across integrity levels.
  - Recommendation: Use the `WM_HOTKEY` path for Search when the foreground process IL is higher (`GetTokenInformation(TokenIntegrityLevel)` on `OpenProcessToken` of the foreground pid), or always. Long term, ship a signed build in `%ProgramFiles%` with `uiAccess="true"` for the hook/injection helper only.

- **WIN-13: WinEvent hook range accidentally subscribes to EVENT_OBJECT_LOCATIONCHANGE and other high-frequency events** (Medium, S)
  - Evidence: `task_windows/native_hooks.rs:313-323`: `SetWinEventHook(EVENT_OBJECT_CREATE, EVENT_OBJECT_NAMECHANGE, ..., WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS)`. The range 0x8000-0x800C includes REORDER, FOCUS, SELECTION*, STATECHANGE and LOCATIONCHANGE (0x800B). `win_event_proc` filters them out only after delivery (`native_hooks.rs:625-649`).
  - Problem: Out-of-context hooks marshal every event in range, system-wide, onto this thread's message queue. LOCATIONCHANGE fires for caret and cursor movement, window drags and animations: thousands of events per second. Each one costs a context switch into JasonShell before being discarded. `handle_event` also calls `std::env::var` (process-global env lock) three times per event (`native_hooks.rs:452,464,476`).
  - Recommendation: Register separate hooks for exactly `EVENT_OBJECT_CREATE..EVENT_OBJECT_HIDE` (0x8000-0x8003) and `EVENT_OBJECT_NAMECHANGE` (0x800C). Cache env flags in `OnceLock<bool>`. Optionally scope hooks to `idProcess` for known windows.

- **WIN-14: ShellHook registered but only HSHELL_FLASH consumed; window lifecycle is inferred from WinEvents** (Low, M)
  - Evidence: `native_hooks.rs:354` `RegisterShellHookWindow(hwnd)`; `wnd_proc` handles only `HSHELL_FLASH` (`native_hooks.rs:605-621`).
  - Problem: Explorer's taskbar logic keys off `HSHELL_WINDOWCREATED/DESTROYED/ACTIVATED/RUDEAPPACTIVATED/REDRAW/WINDOWREPLACED/MONITORCHANGED/GETMINRECT/APPCOMMAND`. These are the shell's own definition of a "taskbar window" and are delivered only for windows Explorer would show. JasonShell instead re-implements the heuristic (`task_windows/windows.rs:647-670`) and re-enumerates after every WinEvent. `HSHELL_GETMINRECT` is not answered, so minimize/restore animations target Explorer's hidden taskbar. `HSHELL_APPCOMMAND` (media keys) is ignored.
  - Recommendation: Consume `HSHELL_WINDOWCREATED/DESTROYED/ACTIVATED/RUDEAPPACTIVATED/REDRAW/FLASH` as the primary signal and keep WinEvents only as a fallback. Answer `HSHELL_GETMINRECT` with the button rect of the matching task item (write `SHELLHOOKINFO.rc`).

- **WIN-15: Taskbar window filter diverges from Explorer's rules (owned WS_EX_APPWINDOW, ITaskbarList::DeleteTab, empty titles, virtual desktops)** (Medium, M)
  - Evidence: `task_windows/windows.rs:647-670`. `!candidate.has_owner` excludes all owned windows, even those with `WS_EX_APPWINDOW`. Untitled windows are excluded unless `WS_EX_APPWINDOW`. Cloaked windows are excluded, which also hides windows on other virtual desktops. There is no AUMID or `ITaskbarList` awareness.
  - Problem:
    - Explorer shows owned windows that have `WS_EX_APPWINDOW`. Some apps (installers, some Java/Qt apps) rely on this.
    - Apps that call `ITaskbarList::DeleteTab` to hide a visible window, or `AddTab` to show a tool window, are honored only by Explorer's hidden taskbar, so JasonShell shows or hides the wrong entries.
    - Grouping is by process path, not AppUserModelID, so UWP apps (`ApplicationFrameHost.exe`) collapse together, and apps that share an exe (Chrome profiles, Edge PWAs, Office) cannot be grouped or split correctly.
  - Recommendation: Mirror Explorer's rules: visible, unowned or `WS_EX_APPWINDOW`, not `WS_EX_TOOLWINDOW` unless APPWINDOW, not cloaked by `DWM_CLOAKED_APP`. Allow `DWM_CLOAKED_SHELL` when the window is on another virtual desktop and the setting is "all desktops". Group by `SHGetPropertyStoreForWindow` + `PKEY_AppUserModel_ID`, falling back to `GetApplicationUserModelId` on the process and then the exe path. For UWP frames, resolve the hosted `Windows.UI.Core.CoreWindow` child's process.

- **WIN-16: Taskbar "Close" escalates to TerminateProcess after ~1.75 s, killing apps that show "Save changes?"** (Critical, S)
  - Evidence: `task_windows/actions.rs:18-20` (`CLOSE_TIMEOUT_MS = 750`, `CLOSE_VERIFY_ATTEMPTS = 10 × 50 ms`) and `actions.rs:283-332`. Sequence: `SendMessageTimeoutW(WM_CLOSE)`; if the window still exists, `PostMessageW(WM_CLOSE)` + wait; then `terminate_window_process` → `TerminateProcess(process_handle, 1)` (`actions.rs:334-389`). On access denied it launches an elevated helper that does the same (`actions.rs:431-442`, `task_windows/helper.rs:107-130`). Reachable from the taskbar context menu "close" (`taskbar_menu.rs:295,400-401`) and `close_task_window` (`task_windows/mod.rs:225-231`).
  - Problem: Word, Notepad, VS Code, Photoshop and others respond to `WM_CLOSE` with a modal "Save changes?" dialog, so the window legitimately remains open while the user decides. JasonShell then force-kills the whole process (all windows and documents of that process) with no confirmation. That is data loss. For elevated targets, the user gets a UAC prompt for "JasonShell" whose effect is a kill. Explorer's "Close window" never terminates. Task Manager's "End task" is a separate, explicit action.
  - Recommendation: "Close" should post `WM_SYSCOMMAND, SC_CLOSE` (what Explorer sends) and return. It must never terminate. Offer a separate explicit "End task" that uses `IsHungAppWindow` / `EndTask(hwnd, FALSE, TRUE)` semantics, shows a confirmation, and kills only after the user confirms. Remove the automatic elevation path, or gate it behind an explicit "End task as administrator".

- **WIN-17: Win-key shortcuts and Start still route to the hidden Explorer taskbar** (Medium, M)
  - Evidence: The hotkey registry supports only Ctrl/Alt modifiers (`windows_key_hook.rs:108-113`, `HotkeyModifier::Ctrl|Alt`). The Win modifier is explicitly excluded from Search matching (`windows_key_hook.rs:108`). There is no handling of `VK_LWIN`/`VK_RWIN`.
  - Problem: With Explorer still the shell, these all hit Explorer while its taskbar is hidden:
    - Win+T and Win+B move keyboard focus into the invisible taskbar or tray, so the user loses keyboard focus.
    - Win+1..9 launch Explorer's pins, which can disagree with JasonShell if pins diverge (WIN-27).
    - Win (Start) opens the Start menu anchored to the hidden taskbar's edge, drawn over JasonShell's bottom bar.
    - Win+A and Win+N open Action Center over the bars.
  - Recommendation: Short term: intercept Win+T/Win+B/Win+number in the LL hook (swallow the key and route to JasonShell equivalents), and document that Start belongs to Explorer. Long term: Explorer-less mode (Big bets) with an own Start/launcher bound to the Win key (LWin up without chord → toggle launcher), handling `RegisterShellHookWindow` `HSHELL_TASKMAN` as Explorer does.

- **WIN-18: System tray is a cross-process scrape of Explorer's hidden ToolbarWindow32 using guessed private offsets (breaks on Windows 11)** (High, L)
  - Evidence:
    - `system_tray.rs:541-565` searches `Shell_TrayWnd`/`Shell_SecondaryTrayWnd` → `TrayNotifyWnd` → `ToolbarWindow32`, plus `NotifyIconOverflowWindow`.
    - `system_tray.rs:745-904` uses `OpenProcess(PROCESS_VM_OPERATION|VM_READ|VM_WRITE)` on explorer.exe, `VirtualAllocEx`, `TB_GETBUTTON`, then `ReadProcessMemory` of the private `TRAYDATA` behind `TBBUTTON::dwData` at candidate offsets (`system_tray.rs:480-495`).
    - Clicks are synthesized by posting `WM_LBUTTONDOWN/UP` to the hidden toolbar at the item center from `TB_GETITEMRECT` (`system_tray.rs:206-239`).
  - Problem:
    - Since Windows 11 22H2 the notification area is XAML-hosted, and `ToolbarWindow32` under `TrayNotifyWnd` no longer exists, so the feature returns nothing.
    - On Windows 10 it depends on undocumented structure layout and on Explorer running.
    - It writes into Explorer's address space.
    - It cannot deliver `NIN_SELECT`/`NIN_KEYSELECT`, `WM_CONTEXTMENU` with v4 semantics, `NIN_POPUPOPEN` tooltips, balloon/toast callbacks, hover (`WM_MOUSEMOVE`) or keyboard activation.
    - Context menus appear positioned relative to the hidden taskbar.
    - The toolbar owner process is not verified to be explorer.exe (any process can register a `Shell_TrayWnd` class window).
  - Recommendation:
    - The real solution is a `Shell_NotifyIcon` host. Register a top-level window of class `Shell_TrayWnd` (only when Explorer's taskbar is not running) and handle `WM_COPYDATA` with `dwData == 1` (`NOTIFYICONDATA` messages `NIM_ADD/MODIFY/DELETE/SETFOCUS/SETVERSION`). Answer `NIN_*` callbacks per `uVersion` (v3/v4), and broadcast `TaskbarCreated` on start so apps re-add icons. This is what Cairo, RetroBar/ManagedShell and other shells do.
    - When coexisting with Explorer, an alternative is a "tray proxy" that sub-registers through `Shell_TrayWnd` message interception (ManagedShell approach, which hooks `Shell_NotifyIcon` traffic via a `Shell_TrayWnd` with higher z-order created before Explorer's) or UI Automation of the XAML tray as a read-only fallback.
    - At minimum, verify the toolbar process image is `%WINDIR%\explorer.exe` before `PROCESS_VM_WRITE`.

- **WIN-19: SPI_SETWORKAREA set manually and repeatedly, fighting Explorer's work-area manager** (Medium, S)
  - Evidence: `appbar.rs:1032-1044` (`SPI_SETWORKAREA` with `SPIF_SENDCHANGE`), with up to 20 retries per call (`appbar.rs:1046-1073`). It is called at activation (`appbar.rs:684-706,730-736`), on resize (`appbar.rs:500-503`), on every fullscreen transition (`appbar.rs:1775,1823-1844,1866,1927`), and on cleanup.
  - Problem: When Explorer is the AppBar manager, `ABM_SETPOS` already adjusts the work area of the affected monitor. Writing `SPI_SETWORKAREA` directly races Explorer's recomputation, which happens on every `ABN_POSCHANGED`, taskbar move or display change. Each call broadcasts `WM_SETTINGCHANGE` to every top-level window in the session, and each retry repeats the broadcast, so all apps re-layout. A full-monitor work area is written while the Explorer taskbar is hidden (`appbar.rs:684-689`). If JasonShell crashes between steps, the work area persists in a wrong state until Explorer recalculates.
  - Recommendation: When Explorer is present, let `ABM_SETPOS` own the work area and only verify with `SPI_GETWORKAREA`/`GetMonitorInfo.rcWork`. Use `SPI_SETWORKAREA` only in Explorer-less mode. Do not retry with a broadcast each time; set once with `SPIF_SENDCHANGE`, then poll with `SPI_GETWORKAREA` without broadcasting.

- **WIN-20: No session, lock, sleep or resume awareness** (Medium, S)
  - Evidence: grep for `WTSRegisterSessionNotification|WM_WTSSESSION_CHANGE|WM_POWERBROADCAST|RegisterSuspendResumeNotification|RegisterPowerSettingNotification` → none.
  - Problem:
    - Polling threads keep running in locked or disconnected sessions: fullscreen guard 4 Hz, taskbar guard 10 Hz, native hook loop 20 Hz `MsgWaitForMultipleObjectsEx(50 ms)` (`native_hooks.rs:415-417`), notification poll 1 Hz (`task_windows/notifications.rs:11,69`).
    - After resume or RDP reconnect, display topology often changes (WIN-03), Explorer may have restarted (WIN-02), and `GetForegroundWindow` returns `NULL`/lock-screen windows. There is no re-sync trigger.
  - Recommendation: `WTSRegisterSessionNotification(hwnd, NOTIFY_FOR_THIS_SESSION)`: pause pollers on `WTS_SESSION_LOCK`/`WTS_REMOTE_DISCONNECT`/`WTS_CONSOLE_DISCONNECT`, and run full `relayout()` + re-registration on `WTS_SESSION_UNLOCK`/`WTS_REMOTE_CONNECT`. `RegisterSuspendResumeNotification(hwnd, DEVICE_NOTIFY_WINDOW_HANDLE)` → re-sync on `PBT_APMRESUMEAUTOMATIC`.

- **WIN-21: No WM_QUERYENDSESSION/WM_ENDSESSION handling; RunEvent::ExitRequested is always prevented** (Low, S)
  - Evidence: `main.rs:706-714` always calls `api.prevent_exit()`. grep for `WM_QUERYENDSESSION|WM_ENDSESSION|SetProcessShutdownParameters` → none.
  - Problem: At logoff or shutdown JasonShell neither runs its cleanup lane nor sets shutdown ordering. Session end makes most of that moot, but fast user switching plus sign-out, or a "restart apps" feature that relaunches, can leave a hidden taskbar or a stale work area visible briefly. Also, `SetProcessShutdownParameters(0x4FF, 0)` is what a shell uses to be notified last, after apps close.
  - Recommendation: Handle `WM_ENDSESSION(wParam=TRUE)` in the hook window by running `restore_on_shell_exit` synchronously with a short budget. Call `SetProcessShutdownParameters(0x4FF, SHUTDOWN_NORETRY)` if JasonShell ever runs as the real shell.

- **WIN-22: Graceful exit is always a hard TerminateProcess after 3 s; no flush guarantee** (Low, S)
  - Evidence: `graceful_exit.rs:46-56`: the watchdog sleeps until the deadline and then calls `hard_exit()` unconditionally. `graceful_exit.rs:172-179` calls `TerminateProcess(GetCurrentProcess(), 0)`.
  - Problem: Even when all lanes finish in 50 ms, the process lives for the full 3 s. Worse, if a lane needs longer (AppBar restore retries are up to 20 × 50 ms per `SPI` call, `ShowWindow` on a hung Explorer, WebView2 profile flush), it is cut off mid-restore with no record. `TerminateProcess` also skips WebView2 controller teardown, so the WebView2 user-data folder (localStorage, IndexedDB) may not flush and can trip "profile in use" on fast relaunch.
  - Recommendation: Join the lanes with a deadline and exit with `ExitProcess`/`app.exit(0)` as soon as they complete. Keep `TerminateProcess` only for the deadline case, write a "dirty exit" marker that the next start uses for recovery (WIN-04), and log which lane missed the deadline.

- **WIN-23: Multi-monitor is planning-only; layout code exists but is unused** (Medium, L)
  - Evidence:
    - `layout.rs:96-131` `plan_monitor_shell_layout` is referenced only by its own tests (grep shows no non-test callers).
    - 12 call sites use `primary_monitor()` (`appbar.rs:260,459-462`, `shell_windows.rs:111,163,204`, ...).
    - Legacy taskbar suppression hides only the primary `Shell_TrayWnd`, leaving `Shell_SecondaryTrayWnd` visible (`explorer.rs:197-215`, `appbar.rs:692-697`).
    - Fullscreen detection only checks the primary monitor rect (`appbar.rs:1725-1739`).
  - Problem: On multi-monitor setups the user gets JasonShell on the primary and Explorer taskbars on the others, each with its own task list and tray. Windows on secondary monitors have no window→monitor mapping (no `MonitorFromWindow` in task grouping). `virtual_desktop_rect()` parking below the virtual screen (`appbar.rs:1182-1231`) can land on a monitor positioned below the primary in some arrangements.
  - Recommendation: One bar pair per `HMONITOR` (`EnumDisplayMonitors`), each registered as its own AppBar with `ABM_SETPOS`. Per-monitor task filtering via `MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST)` updated on `EVENT_OBJECT_LOCATIONCHANGE` filtered to top-level windows. Hide `Shell_SecondaryTrayWnd` per monitor. Use the existing `plan_monitor_shell_layout` as the core.

- **WIN-24: WebView2 process failure is never handled, so the bars can go permanently blank** (Medium, M)
  - Evidence: No `ProcessFailed`/`BrowserProcessExited` handling in `src` or `vendor/wry/src` (grep). There are about 20 webview windows (`shell_windows.rs`), all sharing one WebView2 browser process.
  - Problem: A WebView2 runtime update (Evergreen updates in place), a GPU process crash or a renderer OOM leaves every shell surface white or frozen with no recovery. For a shell this is equivalent to a taskbar crash with no restart.
  - Recommendation: Expose `ICoreWebView2::add_ProcessFailed` through the vendored wry patch (or `with_webview` + `webview2-com`). On `COREWEBVIEW2_PROCESS_FAILED_KIND_BROWSER_PROCESS_EXITED`, recreate the controllers. On `RENDER_PROCESS_EXITED`/`UNRESPONSIVE`, call `Reload()`. Also handle `add_NewBrowserVersionAvailable` by scheduling a controlled reload.

- **WIN-25: WinRT notification listener runs without an apartment and polls every second** (Low, S)
  - Evidence: `task_windows/notifications.rs:37-70`: `UserNotificationListener::Current()`, `RequestAccessAsync().get()` and `GetNotificationsAsync().get()` in a 1 s loop on a worker. There is no `RoInitialize`/`CoInitializeEx`/`CoIncrementMTAUsage` (grep).
  - Problem: WinRT activation on a thread with no apartment works only if some other thread happens to keep the implicit MTA alive. The audio worker threads come and go (`audio.rs:380-398`), so activation can intermittently fail with `CO_E_NOTINITIALIZED`. The listener also requires package identity plus the `userNotificationListener` capability. The dev script registers a loose package (`scripts/register-notification-identity.ps1`) but does not declare that capability (lines 48-55), so in practice access is unavailable. Polling all toasts every second is wasteful.
  - Recommendation: `RoInitialize(RO_INIT_MULTITHREADED)` at worker start (or `CoIncrementMTAUsage` once at process start). Subscribe to `UserNotificationListener.NotificationChanged` when identity allows, otherwise back off to ≥ 5 s. Declare `<uap3:Capability Name="userNotificationListener"/>` in the package manifest.

- **WIN-26: Undocumented IPolicyConfig vtable for default audio device; no endpoint notifications** (Low, M)
  - Evidence: `audio.rs:196-280` hand-builds `IPolicyConfig` (CLSID `870af99c-...`, IID `f8679f50-...`) and calls `SetDefaultEndpoint` (`audio.rs:615`). There is no `IMMNotificationClient`, `IAudioEndpointVolumeCallback` or `IAudioSessionNotification` (grep). Each audio call spawns a fresh thread + `CoInitializeEx(MTA)` + enumerator (`audio.rs:380-398`).
  - Problem: An undocumented interface whose vtable layout has changed across Windows versions before. A wrong slot means calling an arbitrary function pointer. Device and volume changes made elsewhere (keyboard volume keys, Bluetooth connect) are seen only when the UI polls.
  - Recommendation: Keep `IPolicyConfig` but guard it by an OS build check, with a fallback to opening `ms-settings:sound`. Register `IMMDeviceEnumerator::RegisterEndpointNotificationCallback` and `IAudioEndpointVolume::RegisterControlChangeNotify` on one long-lived MTA thread and push events to the UI.

- **WIN-27: Pinning writes and deletes files in Explorer's private "User Pinned\TaskBar" folder; pins are keyed by exe path, not AUMID** (Medium, M)
  - Evidence: `launchers.rs:546-550` (`...\Internet Explorer\Quick Launch\User Pinned\TaskBar`). `pin_task_window_to_taskbar` creates a `.lnk` to the process exe (`launchers.rs:290-345`). `unpin_pinned_taskbar_app` calls `fs::remove_file` (`launchers.rs:276-284`).
  - Problem:
    - Explorer's real pin list is the `Taskband` registry blob (`HKCU\...\Explorer\Taskband\Favorites`). Dropping or deleting `.lnk` files there desynchronizes Explorer (broken or ghost pins if the user returns to Explorer) and does not honor AUMID.
    - Pinning a UWP window pins `ApplicationFrameHost.exe`.
    - Pinning a Squirrel/Electron app pins the versioned exe, which disappears on update.
    - Pinning a Chrome PWA pins `chrome.exe` without `--app-id`.
  - Recommendation: Own JasonShell's pin store (JSON under `%LOCALAPPDATA%\JasonShell`). Pin by AUMID when available (`SHGetPropertyStoreForWindow` → `PKEY_AppUserModel_ID`, plus `PKEY_AppUserModel_RelaunchCommand`/`RelaunchDisplayNameResource`/`RelaunchIconResource`, which is exactly what apps set for this purpose). Launch via `IApplicationActivationManager::ActivateApplication` for packaged apps and `shell:AppsFolder\<AUMID>` otherwise. Treat Explorer's folder as a read-only import source.

- **WIN-28: Process-manager command-line reading via PEB walking with PROCESS_VM_READ** (Low, S)
  - Evidence: `process_manager.rs:887-895,1170-1200`: `NtQueryInformationProcess(ProcessBasicInformation)` → `ReadProcessMemory(PEB)` → `RTL_USER_PROCESS_PARAMETERS` → buffer, for every process on every list refresh.
  - Problem: Three cross-process reads per process per refresh. Racy (the target can free or change the buffer). Fails for protected processes. Requires `PROCESS_VM_READ`, which is more privilege than needed. Undocumented structure use.
  - Recommendation: Use `NtQueryInformationProcess(ProcessCommandLineInformation = 60)`, which needs only `PROCESS_QUERY_LIMITED_INFORMATION` (Windows 8.1+). Cache by `(pid, creation_time)` since command lines never change.

- **WIN-29: Window icons use small (16 px) icons only; packaged apps get generic icons** (Low, S)
  - Evidence: `task_windows/icons.rs:145-163` (`ICON_SMALL2`, `ICON_SMALL`, `ICON_BIG`, class icons), `icons.rs:197-208` (`SHGFI_ICON | SHGFI_SMALLICON`).
  - Problem: At 150-200 % scaling, 16 px icons are upscaled and blurry. UWP/packaged windows (`ApplicationFrameHost`) return generic frame icons.
  - Recommendation: Prefer `ICON_BIG` and `SHGFI_LARGEICON`, or better `IShellItemImageFactory::GetImage` at `GetDpiForWindow`-scaled size. For packaged apps, resolve the AUMID → `IShellItem` for `shell:AppsFolder\<AUMID>` → `GetImage` (already done for pins in `launchers.rs:700-725`; reuse it for windows).

- **WIN-30: Pervasive `.expect("... poisoned")` on shared state; any panic in a guard thread cascades into UI-thread panics** (Medium, S)
  - Evidence: `appbar.rs:334,406,793,824,1902,1920` and many others use `state.lock().expect("shell runtime state is poisoned")`. Guard threads touch the same Mutex (`appbar.rs:1665-1669,1742-1745`).
  - Problem: A panic while the lock is held (for example inside a Win32 wrapper) poisons it. Every later command on the main thread then panics. In `extern "system"` callbacks or the tao loop that aborts the process, which leads to WIN-04.
  - Recommendation: Use `parking_lot::Mutex` (no poisoning) or `lock().unwrap_or_else(PoisonError::into_inner)`, and make state transitions panic-free. Add a panic hook (WIN-04).

- **WIN-31: No single-instance enforcement** (Medium, S)
  - Evidence: There is no `CreateMutexW`, `tauri-plugin-single-instance` or named-pipe guard. `automation.rs:78-110,208-218` is a plan-only "forwarding contract".
  - Problem: A second launch (double-clicking, autostart plus manual start, a crash-recovery relaunch while the old process is still exiting) creates a second set of AppBars, hides an already-hidden taskbar (recording it as "originally hidden", which feeds WIN-04), installs a second LL hook, and fights over the work area.
  - Recommendation: `CreateMutexW(None, TRUE, w!("Local\\JasonShell.Instance.{GUID}"))` at the top of `main()` before Tauri init. On `ERROR_ALREADY_EXISTS`, forward argv over a named pipe (`\\.\pipe\JasonShell-<SID>`, with a DACL restricted to the user SID) and exit. Alternatively use `tauri-plugin-single-instance`.

- **WIN-32: No autostart / shell registration story** (Low, M)
  - Evidence: No `Run`/`RunOnce`/`Winlogon\Shell` writes and no Task Scheduler logon task (grep). `bundle` in `tauri.conf.json:12-14` has only `resources`.
  - Problem: As a "shell replacement" it must start before the user interacts, ideally before Explorer's taskbar paints to avoid flicker, and restart on crash.
  - Recommendation:
    - Phase 1: HKCU `Run` value or a Task Scheduler logon trigger with "restart on failure".
    - Phase 2 (Explorer-less mode): per-user `HKCU\Software\Microsoft\Windows NT\CurrentVersion\Winlogon\Shell` (requires `Explorer\...\` policy awareness). Also launch `explorer.exe /separate` or not at all, run `Run` keys yourself, and signal `ShellDesktopSwitchEvent` / `msgina: ShellReadyEvent` so Winlogon dismisses the welcome screen.

- **WIN-33: Speech paste overwrites the user's clipboard and lands in clipboard history/cloud; SendInput can collide with held modifiers or be silently UIPI-blocked** (Medium, S)
  - Evidence: `speech_clipboard.rs:776` `write_unicode_text`. Prior-clipboard restore exists only in `#[cfg(test)]` (`speech_clipboard.rs:571-660`). There are no `ExcludeClipboardContentFromMonitorProcessing`/`CanIncludeInClipboardHistory` formats (grep). `speech_target.rs:85-104,131-163` sends four keyboard inputs (Ctrl down, V down, V up, Ctrl up) without checking or releasing other held modifiers.
  - Problem:
    - The user's clipboard content is destroyed by every dictation.
    - Transcripts persist in Win+V history and can sync to cloud clipboard (privacy).
    - If Alt or Shift is still physically down, the target receives Ctrl+Alt+V or Ctrl+Shift+V.
    - `SendInput` into an elevated foreground window is dropped by UIPI with no error indication.
  - Recommendation: Snapshot and restore the clipboard (`OleGetClipboard`, then `OleSetClipboard(prior)` after the target has consumed the paste). Add `CanIncludeInClipboardHistory = 0` and `CanUploadToCloudClipboard = 0` DWORD formats, plus `ExcludeClipboardContentFromMonitorProcessing`. Check `GetAsyncKeyState` for modifiers and inject key-ups first. Detect target IL (WIN-12) and refuse with a clear message.

- **WIN-34: OneDrive placeholders, deduplicated files and other benign reparse points are rejected by copy and move** (Medium, S)
  - Evidence: `stack_popup/items.rs:97-102` treats any `FILE_ATTRIBUTE_REPARSE_POINT` as a reparse point. `file_ops.rs:639-643,717-721` refuses copies ("Copying symbolic links or reparse points is not supported"). `local_drop.rs:264-266` refuses moves.
  - Problem: Every OneDrive "Files On-Demand" placeholder carries a reparse point (`IO_REPARSE_TAG_CLOUD_*`), as do Windows Dedup (`IO_REPARSE_TAG_DEDUP`), WCI and AppExecLink entries. Copying or moving anything in a OneDrive-backed Desktop or Documents folder (the default on many installs) fails.
  - Recommendation: Query the tag (`GetFileInformationByHandleEx(FileAttributeTagInfo)` with `FILE_FLAG_OPEN_REPARSE_POINT`). Reject only name-surrogate tags (`IsReparseTagNameSurrogate`: symlinks, mount points). Allow cloud, dedup and other data tags. For cloud placeholders, consider `CfHydratePlaceholder` or simply opening without `FILE_FLAG_OPEN_REPARSE_POINT` for content.

- **WIN-35: Recycle delete can silently become a permanent delete** (Medium, S)
  - Evidence: `stack_popup/file_ops.rs:796-827`: `SHFileOperationW(FO_DELETE)` with `FOF_ALLOWUNDO | FOF_NOCONFIRMATION | FOF_NOERRORUI` and no `FOF_WANTNUKEWARNING`.
  - Problem: When an item cannot be recycled (network share, removable drive, item larger than the Recycle Bin quota, Recycle Bin disabled), `FOF_NOCONFIRMATION` suppresses the "This file is too big to recycle; delete permanently?" prompt, and the shell deletes permanently. The UI still says "moved to Recycle Bin".
  - Recommendation: Add `FOF_WANTNUKEWARNING`, or migrate to `IFileOperation` with `FOFX_RECYCLEONDELETE | FOF_NOCONFIRMATION`, an `IFileOperationProgressSink` to detect `PostDeleteItem` with `TSF_DELETE_RECYCLE_IF_POSSIBLE` failure, and `FOFX_EARLYFAILURE`. Report non-recyclable items explicitly.

- **WIN-36: Settings persistence uses tmp+rename without flush** (Low, S)
  - Evidence: `settings.rs:1360-1365`: `fs::write(temp)` followed by `fs::rename(temp, path)`.
  - Problem: On power loss, NTFS can journal the rename before the data is flushed, which yields a zero-length or garbage `settings.json` (then the corrupt-backup path at `settings.rs:758` fires and user settings are lost).
  - Recommendation: Open the temp file, `write_all`, `sync_all()` (`FlushFileBuffers`), then `MoveFileExW(MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH)`, or `ReplaceFileW` to keep a backup.

- **WIN-37: Process-tree kill for git/process runner uses taskkill /T by PID** (Low, S)
  - Evidence: `stack_popup/process_runner.rs:555-566` (`taskkill /PID <pid> /T /F`).
  - Problem: Tree enumeration by PID is racy (PID reuse, orphaned grandchildren whose parent already exited are missed) and spawns an extra process. Quick Commands already uses job objects correctly (`quick_commands.rs:1706` `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`).
  - Recommendation: Reuse the Quick Commands job-object launcher for git and other runners: `CREATE_SUSPENDED` → `AssignProcessToJobObject` → resume → `TerminateJobObject` on timeout.

- **WIN-38: Power menu lacks sign-out/lock/hibernate and does not install pending updates** (Low, S)
  - Evidence: `system_power.rs:53-90`: restart/shutdown via `shutdown.exe /r|/s /t 0`, and sleep via `SetSuspendState(false,false,false)`.
  - Problem: No `LockWorkStation`, `ExitWindowsEx(EWX_LOGOFF)`, hibernate, or "Update and shut down" (`InitiateShutdownW(..., SHUTDOWN_INSTALL_UPDATES)`). `SetSuspendState` ignores Modern Standby semantics.
  - Recommendation: Use `InitiateShutdownW` with `SHUTDOWN_POWEROFF|SHUTDOWN_RESTART|SHUTDOWN_INSTALL_UPDATES` and the `SHTDN_REASON_FLAG_PLANNED` reason, plus `ExitWindowsEx` for logoff and `LockWorkStation`. Query `GetPwrCapabilities` before offering hibernate or sleep.

- **WIN-39: Elevated-window interaction relies on UAC-prompting a self-elevating helper** (Low, M)
  - Evidence: `task_windows/windows.rs:445-470` (`ShellExecuteExW` with verb `runas` on `current_exe()` plus `--task-window-helper`).
  - Problem: Under UIPI, posting `WM_CLOSE` to an elevated window is blocked, so JasonShell elevates itself (an unsigned binary, see SEC-11) to deliver one message. That UAC prompt is unexpected, says "Unknown publisher", and trains users to accept prompts from a user-writable binary (see SEC-12).
  - Recommendation: Use `WM_SYSCOMMAND/SC_CLOSE` (also blocked) only when the IL matches. Otherwise show "This window is running as administrator; close it from the window itself" (Explorer behaves the same for non-uiAccess callers). If elevation stays, sign the binary and install it under `%ProgramFiles%`.

- **WIN-40: Dead code paths and env-gated features increase maintenance surface** (Low, S)
  - Evidence: `layout.rs:96-131` (unused multi-monitor planner). The v2 suppression path sits behind `JASONSHELL_EXPLORER_SUPPRESSION_V2` (`appbar.rs:2112`, `native_hooks.rs:120-122`). `aes-gcm` is an unconditional dependency (`Cargo.toml:16`) used only by the `text-document-feasibility` feature module (`stack_popup/text_document/mod.rs:1-2`). The snipping diagnostics sink writes into `%LOCALAPPDATA%\Temp\opencode` when a marker file exists (`snipping/escape.rs:100-105`, `snipping/diagnostics.rs:78-98`). Production logic includes AI-tool process heuristics (`task_windows/windows.rs:872-895`).
  - Problem: Two parallel taskbar-suppression implementations double the test matrix. Developer-tooling paths ship in production.
  - Recommendation: Pick one suppression implementation and delete the other. Make `aes-gcm` optional under the feature. Move diagnostics sinks behind a `debug_assertions` or explicit setting.

- **WIN-41: Shell-surface windows are HWND_TOPMOST with no z-order cooperation** (Low, S)
  - Evidence: `appbar.rs:1168-1176,1216-1220` (`SetWindowPos(HWND_TOPMOST)`), `shell_windows.rs:180,221,248,274` (`always_on_top(true)` on panels).
  - Problem: Without `ABM_ACTIVATE`/`ABM_WINDOWPOSCHANGED` (WIN-01), Explorer cannot order AppBars relative to other topmost windows (Explorer's own flyouts, the Start menu, other appbars). Topmost bars cover topmost app windows (video PiP, OSDs), and vice versa, inconsistently.
  - Recommendation: Implement the AppBar z-order messages. Consider making bars non-topmost when "always on top" is off (`ABS_ALWAYSONTOP` semantics via `ABM_SETSTATE`).

- **WIN-42: No auto-hide support** (Low, M)
  - Evidence: No `ABM_SETAUTOHIDEBAR`/`ABM_SETAUTOHIDEBAREX`/`ABM_GETAUTOHIDEBAREX` usage (grep).
  - Problem: A standard taskbar feature, essential on small laptop screens.
  - Recommendation: `ABM_SETAUTOHIDEBAREX` per edge and monitor. Reserve a 1-2 px trigger strip and reveal on hover through a `WM_MOUSEMOVE`/`TrackMouseEvent` hot zone. Do not reserve work area in auto-hide mode.

---

### Findings: security (SEC-xx)

- **SEC-01: Command injection in "Open terminal here" through PowerShell -Command argument concatenation** (High, S)
  - Evidence: `stack_popup/file_ops.rs:95-140` `launch_terminal_at_dir`:
    1. It tries `%SystemRoot%\System32\wt.exe`, which does not exist there because Windows Terminal is an App Execution Alias under `%LOCALAPPDATA%\Microsoft\WindowsApps`. So the next branch is the effective default.
    2. `powershell.exe -NoExit -NoLogo -Command "Set-Location -LiteralPath $args[0]" <directory>`.
    3. The fallback is `cmd.exe /K cd /d <directory>`.

    It is reached from `open_terminal_here_path` (`file_ops.rs:73-76`) for any existing directory supplied by the Stack Browser.
  - Problem: With `-Command`, PowerShell treats every following argument as part of the script text. It joins them with spaces; it does not bind them to `$args`. A directory named, for example, `x;calc` or `x$(calc)` (both legal NTFS names, trivially delivered inside a zip or git clone) executes code when the user clicks "Open terminal here". The `cmd /K cd /d <dir>` fallback is likewise injectable via `&`, because Rust's `Command` quotes only on whitespace and cmd re-parses metacharacters.
  - Recommendation:
    - Use `-NoExit -NoLogo -WorkingDirectory`-free launching: set the child's working directory with `Command::current_dir(directory)` and launch `powershell.exe -NoExit -NoLogo` with no script text.
    - If a script is required, pass it via `-EncodedCommand` and pass the path through an environment variable (`$env:JASONSHELL_CWD`), never via argv.
    - For wt, resolve `wt.exe` via `SearchPathW` over `%LOCALAPPDATA%\Microsoft\WindowsApps` and use `wt.exe -d <dir>` with `current_dir`.
    - Drop the cmd fallback, or use `current_dir` with `cmd.exe /K` alone.
    - Add regression tests with `;`, `$()`, `&`, `%`, `^`, `"` in folder names.

- **SEC-02: Everything SDK DLL loaded from CWD ancestors and an environment variable before Program Files (DLL planting into the shell process)** (High, S)
  - Evidence: `search_sources/everything_ffi.rs:55-87` builds the candidate order:
    1. `JASONSHELL_EVERYTHING_SDK_DIR`.
    2. `current_dir().ancestors().take(4)` joined with `.local\Everything-SDK\dll\Everything64.dll`.
    3. The installed Everything exe directory.
    4. `%ProgramFiles%`.

    `detect_system_sdk` picks the first existing file (`everything_ffi.rs:47-53`). `query_everything_sdk` then calls `Library::new(dll_path)` per query (`everything_ffi.rs:96-104`) with default search flags. There is no Authenticode or hash check.
  - Problem: The shell's CWD is whatever launched it (a user folder, Downloads, a repo checkout). Any `.local\Everything-SDK\dll\Everything64.dll` up to three directories above it is loaded into the long-lived, hook-owning shell process, giving persistent code execution and keylogging via the existing LL hook. An attacker-controlled environment variable is equally trusted. `LoadLibraryW` without `LOAD_LIBRARY_SEARCH_*` also resolves the DLL's own dependencies through the standard search order. Loading and unloading per query is also slow and churns the loader lock.
  - Recommendation:
    - Remove the CWD candidates from release builds (`#[cfg(debug_assertions)]`), and gate the env override the same way.
    - Load once and cache.
    - Load with `LoadLibraryExW(path, None, LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR | LOAD_LIBRARY_SEARCH_SYSTEM32)` (libloading: `os::windows::Library::load_with_flags`).
    - Verify the Authenticode signature (`WinVerifyTrust` with `WINTRUST_ACTION_GENERIC_VERIFY_V2`, signer `voidtools`) or a pinned SHA-256.
    - Call `SetDefaultDllDirectories(LOAD_LIBRARY_SEARCH_DEFAULT_DIRS)` early in `main()`.
    - Better: talk to Everything over its documented IPC (`WM_COPYDATA` to the `EVERYTHING_TASKBAR_NOTIFICATION` window / Everything 1.5 named pipe) and drop the DLL entirely.

- **SEC-03: Git runs on every browsed folder without hardening, so a malicious repo config executes code** (High, S)
  - Evidence: `stack_popup/git_status.rs:1620-1655` `run_git_command` sets only `GIT_TERMINAL_PROMPT=0`, `GCM_INTERACTIVE=never` and `GIT_OPTIONAL_LOCKS=0`. There are no `-c core.fsmonitor=`, `--no-ext-diff`, `--no-textconv` or `safe.directory` controls (grep). Status and summary are fetched automatically as folders are browsed and watched (`folder_watch.rs`, `get_stack_git_status`/`summary`). The terminal prompt also runs `git status` in every directory (`stack_popup/terminal.rs:1268`).
  - Problem: `.git/config` inside a downloaded or extracted repo is owned by the current user, so `safe.directory` does not apply. Simply opening that folder in the Stack popup can execute:
    - `core.fsmonitor = <cmd>`: runs on `git status`.
    - `filter.<x>.clean`/`process` with matching `.gitattributes`: runs on `status` when stat data is dirty.
    - `diff.<x>.textconv` and `diff.external`: run on `stack_git_diff`.
    - `core.sshCommand`, `credential.helper`, `protocol.*.allow`: run on fetch/pull/push.
  - Recommendation:
    - For read paths, always prepend `-c core.fsmonitor=false -c core.untrackedCache=false -c diff.external= -c core.pager=cat` and use `--no-ext-diff --no-textconv` for diffs. Set `GIT_CONFIG_NOSYSTEM=1` where appropriate. Consider `-c protocol.ext.allow=never -c protocol.file.allow=user`.
    - Detect repos where `.git/config` contains `fsmonitor`, `filter.`, `textconv`, `sshCommand` or `hooksPath` and ask the user to trust the repo (VS Code "Restricted Mode" model) before running mutating commands (fetch, pull, commit; hooks).
    - Never auto-run git on removable or network paths.

- **SEC-04: Taskbar Close force-terminates processes (data-loss / integrity)** (High, S). Cross-reference: WIN-16.
  - Evidence: `task_windows/actions.rs:283-389,431-442`.
  - Problem: An unconfirmed `TerminateProcess` of arbitrary user processes, including through an elevated helper. The identity revalidation (`actions.rs:391-423`, pid + creation time + image path) is good, but the action itself is wrong.
  - Recommendation: See WIN-16.

- **SEC-05: open_shell_path executable denylist is incomplete; UNC/WebDAV paths allowed** (Medium, S)
  - Evidence: `shell_paths.rs:56-73,128-153`. The denylist is `exe com bat cmd ps1 psm1 vbs js jse wsf msi msc scr lnk url cpl reg`. Any other existing path goes to `ShellExecuteW(NULL verb)` (`shell_paths.rs:276-300`). `classify_shell_open_target` accepts `\\server\share\...`.
  - Problem: Missing dangerous types include `hta`, `pif`, `vbe`, `wsh`, `ws`, `scf`, `application`, `appref-ms`, `jar`, `py`/`pyw` (if associated), `ps1xml`, `psc1`, `msp`, `mst`, `msix`/`appx`/`appinstaller`, `settingcontent-ms`, `library-ms`, `search-ms`, `diagcab`, `chm`, `inf` (default verb on some), `xll`, `wsc`, `gadget`, `mshxml`, and files with no extension. A denylist will always be incomplete. UNC paths trigger SMB authentication (NTLM hash leak to a remote host) and WebDAV fetches just by checking `exists()`. `open_shell_path` takes no caller window, so any surface holding the legacy ACL (`capabilities/legacy-shell.json`, 18 windows) can call it (IPC audit covers the ACL itself).
  - Recommendation: Invert to an allowlist for the "open" verb, or use `AssocQueryStringW(ASSOCSTR_EXECUTABLE)` plus `AssocIsDangerous(path)`. That is the shell's own dangerous-type check, used by Attachment Manager. Use `IAttachmentExecute::CheckPolicy` for Mark-of-the-Web files. Reject `\\?\UNC\`, `\\` and `\\.\` prefixes unless explicitly allowed (`PathIsUNCW`, `PathIsNetworkPathW`).

- **SEC-06: Cross-process memory writes into explorer.exe for tray scraping** (Medium, M)
  - Evidence: `system_tray.rs:759-768` (`PROCESS_VM_OPERATION | VM_READ | VM_WRITE`), `system_tray.rs:877-904` (`VirtualAllocEx`/`VirtualFreeEx`). The toolbar owner is never validated as `explorer.exe`.
  - Problem: Any same-user process can create windows of class `Shell_TrayWnd`/`NotifyIconOverflowWindow` with a `ToolbarWindow32` child. JasonShell will then open it with write access, read attacker-chosen pointers (`TRAYDATA` offsets) and `CopyIcon` attacker-supplied handle values. The impact is limited (same user), but it is a fragile trust boundary, and EDR products flag `VirtualAllocEx` + write into explorer.exe as injection behavior.
  - Recommendation: Verify `QueryFullProcessImageNameW` == `%WINDIR%\explorer.exe`, and verify the window is the one returned by `FindWindowW("Shell_TrayWnd")` whose pid matches `GetShellWindow()`'s pid. Long term, replace with a real tray host (WIN-18).

- **SEC-07: No critical-process denylist in process termination** (Low, S)
  - Evidence: `process_manager.rs:352-354` `is_pid_killable` excludes only pid 0 and self. The guardrail plan (`process_manager.rs:395-440`) warns only about workspace associations.
  - Problem: If JasonShell is ever run elevated (WIN-39 helper, or the user starting it as admin), killing `csrss`, `wininit`, `smss` or `services` bugchecks the machine, and killing `lsass` forces a reboot. Unelevated, it can kill `explorer.exe` and `dwm.exe` (auto-restarts, but disruptive) or JasonShell's own WebView2 children.
  - Recommendation: Deny when `IsProcessCritical(handle)` is true, `ProcessProtectionLevel` is non-zero, the session id is 0, or the image is `%WINDIR%\System32\{csrss,wininit,smss,services,lsass,winlogon,dwm,fontdrvhost}.exe`. Add an explicit warning for `explorer.exe` and for JasonShell's own child processes.

- **SEC-08: Elevated helper and runas launches built from user-writable install locations (EoP-by-consent)** (Medium, M)
  - Evidence: `task_windows/windows.rs:445-470` elevates `current_exe()`. `launchers.rs:162-210` elevates pinned shortcut targets. The default Tauri bundle has no install-scope settings (`tauri.conf.json:12-14`), and the dev workflow runs from `src-tauri\target\debug`.
  - Problem: When the elevated binary lives in a user-writable directory (per-user NSIS install under `%LOCALAPPDATA%`, or target dirs), any same-user malware can replace it and wait for the user to approve the next UAC prompt, which is labelled with JasonShell's name but "Unknown publisher" (SEC-11).
  - Recommendation: Install per-machine to `%ProgramFiles%` (admin-only ACL) if elevation features remain. Before `runas`, verify the exe directory ACL denies writes to the user (`GetNamedSecurityInfoW` + `AccessCheck`), or drop self-elevation entirely (WIN-39).

- **SEC-09: CSP pulls fonts from Google at runtime (privacy/offline)** (Low, S)
  - Evidence: `tauri.conf.json:18`: `style-src 'self' 'unsafe-inline' https://fonts.googleapis.com; font-src 'self' https://fonts.gstatic.com`.
  - Problem: The shell makes third-party network requests at every start (IP and time leak to Google). Fonts fail offline or behind proxies, and the shell's appearance depends on a network fetch. `'unsafe-inline'` styles also widen the injection blast radius if any surface renders untrusted text as HTML (file names, window titles, tray tooltips, git output).
  - Recommendation: Vendor the fonts into `dist`, remove both Google origins, and move inline styles to classes or nonces where feasible. Confirm that all renderer sinks for window titles, file names and git output are text-only (IPC/front-end audit).

- **SEC-10: Privacy exposure in process manager and diagnostics** (Low, S)
  - Evidence: Full command lines of all processes are surfaced to the renderer (`process_manager.rs:906,920`). Diagnostics redaction is key-name based (`diagnostics.rs:99-150`).
  - Problem: Command lines routinely contain tokens and passwords (`--token=`, `-p`, connection strings). They are rendered in a WebView and could be exported via `export_diagnostics`.
  - Recommendation: Mask common secret patterns in command lines before sending them to the renderer, and reveal on explicit click. Extend `redact_text` to command-line patterns.

- **SEC-11: Windows application manifest incomplete; no code signing** (Medium, S)
  - Evidence: `build.rs:54-56` uses the default `tauri_build::Attributes` (only the ACL app manifest is customised). The embedded Windows manifest is tauri-build 2.5.6's `windows-app-manifest.xml`, which declares only the Common-Controls v6 dependency: no `<compatibility><supportedOS>` (Win10/11 GUID), no `<dpiAwareness>PerMonitorV2</dpiAwareness>`, no `<longPathAware>`, no `<requestedExecutionLevel>`. There is no `bundle.windows.certificateThumbprint`/`signCommand` in `tauri.conf.json`.
  - Problem:
    - Without the Win10 `supportedOS` GUID, the process runs in a Windows 8 compatibility context: `GetVersionEx` lies, and some APIs pick legacy behaviour.
    - DPI awareness is set only at runtime by tao. Helper modes that exit before tao starts (`main.rs:89-123`: volume helper, snip clipboard owner, task-window helper, elevated) are DPI-unaware, so their coordinates are virtualized on scaled displays.
    - Long-path awareness is missing for a file manager.
    - Unsigned binaries trigger SmartScreen, cannot pass WDAC/AppLocker publisher rules, show "Unknown publisher" on UAC, and are commonly flagged by EDR given the LL keyboard hook + `SendInput` + `VirtualAllocEx` into explorer + `TerminateProcess` combination.
  - Recommendation: Supply a custom manifest via `tauri_build::WindowsAttributes::new().app_manifest(include_str!("jasonshell.exe.manifest"))` with `supportedOS` (Win10/11 GUID `{8e0f7a12-bfb3-4fe8-b9a5-48fd50a15a9a}`), `<dpiAwareness>PerMonitorV2, PerMonitor</dpiAwareness>`, `<longPathAware>true</longPathAware>`, `<requestedExecutionLevel level="asInvoker" uiAccess="false"/>`, `<activeCodePage>UTF-8</activeCodePage>`, and the common-controls dependency. Sign with an EV/OV certificate through `signCommand` (Azure Trusted Signing works well with Tauri).

- **SEC-12: Dev identity script creates a code-signing certificate and trusts it** (Low, S)
  - Evidence: `scripts/register-notification-identity.ps1:20-31`: `New-SelfSignedCertificate -KeyUsage DigitalSignature -TextExtension 2.5.29.37={text}1.3.6.1.5.5.7.3.3` (code-signing EKU) in `CurrentUser\My`, imported into `CurrentUser\TrustedPeople`. It runs with `-ExecutionPolicy Bypass` (line 6). It then copies a debug exe into `.local\notification-package` and registers it (lines 34-36, 65-66).
  - Problem: An exportable code-signing private key is left in the user store indefinitely, and a self-signed root is trusted for package installs. It is not actually used to sign anything (`Add-AppxPackage -Register` of a loose manifest does not need it). The registered package also omits the capabilities the toast listener needs (WIN-25).
  - Recommendation: Drop certificate creation for loose registration. If signing is needed, create the cert with `-KeyExportPolicy NonExportable`, scope it to the package, and offer an `-Unregister` path that removes the cert and package.

- **SEC-13: Clipboard content shared with history/cloud; snipping and speech data not excluded** (Low, S). Cross-reference: WIN-33.
  - Evidence: `speech_clipboard.rs:776`; `snipping/clipboard.rs:389` (PNG format) without exclusion formats.
  - Recommendation: Add `ExcludeClipboardContentFromMonitorProcessing`, `CanIncludeInClipboardHistory=0` and `CanUploadToCloudClipboard=0` for transient shell-generated content, or make it a setting.

- **SEC-14: LL keyboard hook in the shell process enlarges attack and detection surface** (Low, M)
  - Evidence: `windows_key_hook.rs:907-910` (desktop-wide `WH_KEYBOARD_LL`), `snipping/escape.rs:116` (second desktop-wide hook).
  - Problem: Two global keyboard hooks inside a WebView-hosting process mean that any code execution in the shell (for example SEC-02 or SEC-03) inherits a ready keylogger. Each hook also adds latency to every keystroke system-wide while its thread is busy (WIN-11).
  - Recommendation: Consolidate into one hook in a minimal helper process with no WebView. Prefer `RegisterHotKey` wherever chords allow. Install the snipping Escape hook only while a snip is active (verify it is scoped).

- **SEC-15: Vendored wry patch upgrade risk** (Medium, S)
  - Evidence: `Cargo.toml:84-86` (`[patch.crates-io] wry = { path = "vendor/wry" }`), `vendor/wry/JASONSHELL_PATCH.md`. The patch is a teardown backport of upstream PR 1795 (released in wry 0.56.1) plus a custom same-STA local-drop observer (`vendor/wry/src/webview2/drag_drop.rs:38-148`, `src/webview2/mod.rs`, `src/lib.rs`). The app is pinned at tauri-runtime-wry 2.10.1, which requires wry `^0.54`.
  - Problem:
    - Any Tauri upgrade that moves tauri-runtime-wry to wry 0.55+ makes the `[patch]` silently inapplicable (cargo only warns "patch was not used"). The custom local-drop API then disappears (a compile break, which is good) or, if feature-gated, silently reverts drop behavior.
    - Security fixes in wry/WebView2 glue between 0.54.4 and current are not received.
    - The local-drop observer is a JasonShell-specific fork with no upstream path, so every upgrade requires a manual re-port.
    - There is no CI check that `Cargo.lock` resolves wry to the vendored path.
  - Recommendation:
    - Add a CI assertion (`cargo metadata` → wry `source == null`, version `0.54.4`).
    - Upstream the local-drop observer as a generic `DragDropHandler` extension (wry already exposes `with_drag_drop_handler`), or move it out of wry by registering your own `IDropTarget` on a child HWND (`RegisterDragDrop`, after `RevokeDragDrop` of WebView2's).
    - Plan the Tauri bump to a wry ≥ 0.56.1 release and delete the teardown backport.

---

### Shell-replacement completeness matrix

| Feature | Windows taskbar/Explorer has | JasonShell status | Evidence |
|---|---|---|---|
| Work-area reservation | AppBar manager, per monitor | Partial: primary only, two edges, startup geometry only; ABN_* ignored | `appbar.rs:260-276,860-889,974-983` |
| Explorer-restart recovery (`TaskbarCreated`) | Yes (apps re-add tray icons, appbars re-register) | Missing | grep `TaskbarCreated` → none |
| Display/DPI change handling | Yes | Missing (v2 env flag only reconciles Explorer taskbar) | `native_hooks.rs:600-604` |
| Auto-hide | Yes | Missing | grep `ABM_SETAUTOHIDEBAR` → none |
| Fullscreen app handling | `ABN_FULLSCREENAPP` / rude-window detection | Polling heuristic, 250 ms | `appbar.rs:1597-1635,1986-1999` |
| Multi-monitor taskbars | Per monitor, configurable button placement | Missing (planner unused; secondary Explorer taskbars stay visible) | `layout.rs:96-131`, `explorer.rs:197-215` |
| Task buttons (window list) | ShellHook-driven, AUMID grouping | Heuristic enumeration + WinEvents; grouping by exe | `task_windows/windows.rs:607-670` |
| ITaskbarList (DeleteTab/AddTab/MarkFullscreen) | Yes | Not honored (calls go to hidden Explorer taskbar) | no ITaskbarList server |
| Progress / overlay icons (ITaskbarList3) | Yes | Missing | grep `SetProgressValue|SetOverlayIcon` → none |
| Thumbnail toolbars / thumbnail clip | Yes | Missing (DWM thumbnail preview exists) | `task_preview.rs:441`, no ThumbBar |
| Live previews (DWM thumbnails) | Yes (+ Aero Peek) | Present (preview + gallery, RAII handles) | `task_preview.rs:17,90,441`, `task_gallery/thumbnails.rs:96-104` |
| Flash / attention | Yes | Present via `HSHELL_FLASH` | `native_hooks.rs:34,605-621` |
| Minimize animation target (`HSHELL_GETMINRECT`) | Yes | Missing | `native_hooks.rs:600-623` |
| Jump lists (`ICustomDestinationList`, recent/frequent) | Yes | Missing | grep `DestinationList|JumpList` → none |
| Pinned apps | Taskband registry, AUMID-aware | Reads/writes Explorer's `User Pinned\TaskBar` .lnk folder; exe-path pins | `launchers.rs:276-345,546-583` |
| UWP/packaged app launch (AUMID) | `IApplicationActivationManager` | Partial: launches `shell:AppsFolder` .lnk proxies; WindowsApps elevation fallback to explorer.exe | `launchers.rs:186-215,686-748` |
| Notification area hosting (`Shell_NotifyIcon`) | Real host (`WM_COPYDATA` NIM_*) | Proxy scrape of Explorer's hidden Win10 toolbar; broken on Win11 XAML tray | `system_tray.rs:155-239,480-495,541-565` |
| Tray tooltips / balloons / keyboard / hover | Yes | Missing | `system_tray.rs:206-239` |
| Toast notification center | Action Center | Own quick-command notifications; listener polls `UserNotificationListener` (likely denied without capability) | `quick_command_notification.rs`, `task_windows/notifications.rs:37-70` |
| Start menu | Yes | Explorer's Start still used (Win key not handled); own search panel | `windows_key_hook.rs:108-113` |
| Win+key shortcuts (Win+T/B/1..9/D/A/N) | Yes | Missing; Win+T/B focus hidden Explorer taskbar | `windows_key_hook.rs:98-119` |
| Global hotkeys | n/a | Present (RegisterHotKey + LL hook for Search) | `windows_key_hook.rs:647-655,907-910` |
| Window snapping / Snap Layouts | DWM/Explorer | Delegated to Windows (no interference found) | n/a |
| Virtual desktops (`IVirtualDesktopManager`) | Task View | Missing; cloaked windows hidden (other desktops vanish) | `task_windows/windows.rs:659` |
| Show desktop / Aero Peek | Yes | Not found | grep `ShowDesktop|0x7ffff` → none |
| Window system menu (Restore/Move/Size) | Shift+right-click / Alt+Space | Custom menu: focus/minimize/close/pin/process only | `taskbar_menu.rs:293-298,393-404` |
| Elevated window interaction (UIPI) | Explorer runs medium IL too, but owns uiAccess-capable components | Close via self-elevating helper (UAC prompt); LL hook blind | `task_windows/windows.rs:445-470` |
| Clock / calendar | Yes | Present (calendar panel) | `calendar_panel.rs` |
| Volume / audio devices | Yes | Present (Core Audio + undocumented `IPolicyConfig`); no change notifications | `audio.rs:196-280,615` |
| Network / battery / Wi-Fi flyouts | Yes | Not found | n/a |
| Power menu | Shut down/restart/sleep/sign out/lock/update | Restart/shutdown/sleep only | `system_power.rs:53-90` |
| Desktop icons / wallpaper (Progman) | Explorer | Delegated to Explorer | `appbar.rs:2010-2015` treats Progman/WorkerW as desktop |
| File management | Explorer | Stack Browser (copy/move/recycle/rename, git panel) | `stack_popup/*` |
| Crash recovery / watchdog | Explorer auto-restart by Winlogon | Missing (no panic hook, no watchdog, hidden taskbar persists) | WIN-04 |
| Single instance | Yes | Missing | WIN-31 |
| Autostart / shell registration | Winlogon `Shell` | Missing | WIN-32 |
| Works without explorer.exe | n/a | No (`SHAppBarMessage`, tray, pins, Start all require Explorer) | `appbar.rs:860-901`, `system_tray.rs:541-565` |

---

### Big bets

1. **Decide the architecture: "Explorer companion" vs "Explorer-less shell".** Today's hybrid pays the costs of both: it hides Explorer's taskbar by force (polling `ShowWindow(SW_HIDE)`, WIN-09) yet depends on Explorer for AppBars, tray, pins, Start, Win-keys and ITaskbarList.
   - **Companion mode** (lower risk): stop hiding `Shell_TrayWnd` with `ShowWindow`. Instead set Explorer's taskbar to auto-hide via `ABM_SETSTATE(ABS_AUTOHIDE)` on its HWND, or the `StuckRects3` settings with a restore record, and complete the AppBar protocol (WIN-01/02/03). This removes the crash-leaves-taskbar-hidden class of bugs (WIN-04).
   - **Explorer-less mode** (true replacement): implement the four shell contracts Explorer provides:
     - a `Shell_TrayWnd` window that hosts `Shell_NotifyIcon` (`WM_COPYDATA`) and is the AppBar manager (`WM_COPYDATA` `ABM_*` messages, own work-area arithmetic via `SPI_SETWORKAREA`);
     - `SetShellWindow`/`SetTaskmanWindow` plus a `RegisterShellHookWindow` that receives `HSHELL_*`;
     - an `ITaskbarList` COM server (`CLSID_TaskbarList` is in-proc `explorerframe.dll`, which forwards to the window registered as taskbar via `RegisterWindowMessage("TaskbarButtonCreated")` and private messages; follow ManagedShell/Cairo for the details);
     - Winlogon integration (`Shell` value, `ShellReadyEvent`, running `Run` keys).

     ManagedShell (C#, MIT) and RetroBar are good references for the exact message plumbing.

2. **Out-of-process watchdog and recovery agent.** A 200 KB native helper (no WebView) started first, which:
   - spawns the shell;
   - holds the single-instance mutex (WIN-31);
   - waits on the shell's process handle and, on abnormal exit, restores Explorer's taskbar, removes AppBars, restores the work area from a persisted snapshot, and restarts the shell with backoff;
   - writes minidumps through `MiniDumpWriteDump` from the watchdog side, or `WerRegisterRuntimeExceptionModule` (WIN-04, WIN-22, WIN-24).

   The same helper is the right home for the global keyboard hook, which keeps hooks responsive and out of the WebView process (WIN-11, SEC-14), and, once signed and installed under `%ProgramFiles%`, for `uiAccess="true"` so hotkeys and injection work over elevated windows (WIN-12, WIN-33).

3. **Single main-thread "shell window" with a real WndProc.** Create one hidden top-level native window on the UI thread that subclasses or owns both bars. It receives `uCallbackMessage`, `TaskbarCreated`, `WM_DISPLAYCHANGE`, `WM_DPICHANGED`, `WM_SETTINGCHANGE`, `WM_WTSSESSION_CHANGE`, `WM_POWERBROADCAST`, `SHELLHOOK` and `WM_ENDSESSION`, and drives one idempotent `relayout()` / `reregister()` state machine. This replaces the four polling threads (fullscreen 4 Hz, taskbar 10 Hz, native hooks 20 Hz, notifications 1 Hz) and removes the cross-thread `SetWindowPos`/lock hazards (WIN-06). All geometry mutations happen on that thread.

4. **AUMID-first task model.** Key everything (pins, grouping, icons, jump lists, launch) on AppUserModelID from `SHGetPropertyStoreForWindow`/`GetApplicationUserModelId`, falling back to exe path. Launch with `IApplicationActivationManager`. Read jump lists through `IApplicationDocumentLists` / `IAutomaticDestinationList` and custom destinations. This closes WIN-15, WIN-27 and WIN-29 and most of the "UWP looks wrong" class of issues.

5. **Per-monitor bars.** Generalize `ShellSurfaceLayout` to `Vec<MonitorShell>` keyed by `HMONITOR` + device name, wire up the already-written `plan_monitor_shell_layout`, and filter task buttons per monitor (WIN-23).

6. **Hardening pass for spawn and load sites.** One `trusted_spawn` module used everywhere, which:
   - resolves executables only from system dirs or registered install paths;
   - never builds script text from paths (SEC-01);
   - runs everything in job objects (WIN-37);
   - hardens git (SEC-03);
   - calls `SetDefaultDllDirectories` early, with signed and cached DLL loads only (SEC-02);
   - uses `AssocIsDangerous`/`IAttachmentExecute` for opens (SEC-05).

   Also enable `clippy::undocumented_unsafe_blocks` in CI and backfill SAFETY comments in the eight files listed in the inventory.

### Finding count

42 WIN findings + 15 SEC findings = 57 findings.
