// Production state and in-process queue/callback acceptance. Never installs hooks,
// creates windows, emits Tauri events, or generates/observes desktop input.
// Included by windows_key_hook.rs under cfg(test).
mod search_ownership_acceptance {
    use super::*;

    const SPACE: u32 = 0x20;
    const K: u32 = b'K' as u32;

    fn ctrl() -> SearchModifiers {
        SearchModifiers { ctrl: true, ..SearchModifiers::default() }
    }

    fn alt() -> SearchModifiers {
        SearchModifiers { alt: true, ..SearchModifiers::default() }
    }

    fn binding(chord: &str) -> RegistryBinding {
        registry_binding(chord, ConfiguredHotkeyAction::Search).unwrap()
    }

    fn ready(chord: &str) -> SearchKeyOwnership {
        let mut state = SearchKeyOwnership::default();
        state.set_binding(Some(binding(chord)), false);
        state
    }

    fn event(state: &mut SearchKeyOwnership, key: u32, down: bool, modifiers: SearchModifiers, toggles: &mut usize) -> bool {
        state.handle_event(0, key, down, modifiers, || { *toggles += 1; true })
    }

    #[test]
    fn search_down_repeats_and_release_consumed_one_immediate_toggle() {
        let mut state = ready("Ctrl+Space");
        let mut toggles = 0;
        assert!(event(&mut state, SPACE, true, ctrl(), &mut toggles));
        assert_eq!(toggles, 1, "toggle must queue on initial down, not release");
        for _ in 0..20 {
            assert!(event(&mut state, SPACE, true, ctrl(), &mut toggles));
        }
        assert!(event(&mut state, SPACE, false, ctrl(), &mut toggles));
        assert_eq!(toggles, 1, "repeats consumed, toggle count 1");
        assert!(event(&mut state, SPACE, true, ctrl(), &mut toggles));
        assert_eq!(toggles, 2, "next released cycle must work");
    }

    #[test]
    fn owned_release_survives_focus_change_and_modifier_release() {
        // Focus is deliberately not an input: ownership cannot depend on HWND.
        let mut state = ready("Ctrl+Space");
        let mut toggles = 0;
        assert!(event(&mut state, SPACE, true, ctrl(), &mut toggles));
        assert!(!event(&mut state, 0xa2, false, SearchModifiers::default(), &mut toggles));
        assert!(event(&mut state, SPACE, true, SearchModifiers::default(), &mut toggles));
        assert!(event(&mut state, SPACE, false, SearchModifiers::default(), &mut toggles));
        assert_eq!(toggles, 1);
    }

    #[test]
    fn configured_alt_search_owns_trigger_but_never_alt_release() {
        let mut state = ready("Alt+K");
        let mut toggles = 0;
        assert!(!event(&mut state, K, true, ctrl(), &mut toggles));
        assert!(!event(&mut state, K, false, ctrl(), &mut toggles));
        assert!(!event(&mut state, 0xa4, true, alt(), &mut toggles));
        assert!(event(&mut state, K, true, alt(), &mut toggles));
        assert!(!event(&mut state, 0xa4, false, SearchModifiers::default(), &mut toggles));
        assert!(event(&mut state, K, true, SearchModifiers::default(), &mut toggles));
        assert!(event(&mut state, K, false, SearchModifiers::default(), &mut toggles));
        assert_eq!(toggles, 1);
    }

    #[test]
    fn fresh_install_state_does_not_inherit_ownership_and_preheld_waits_for_release() {
        let mut old = ready("Ctrl+Space");
        let mut toggles = 0;
        assert!(event(&mut old, SPACE, true, ctrl(), &mut toggles));
        old.set_binding(None, false);
        assert!(event(&mut old, SPACE, false, ctrl(), &mut toggles));
        let mut fresh = SearchKeyOwnership::default();
        fresh.set_binding(Some(binding("Ctrl+Space")), true);
        assert!(!event(&mut fresh, SPACE, true, ctrl(), &mut toggles));
        assert!(!event(&mut fresh, SPACE, false, ctrl(), &mut toggles));
        assert_eq!(toggles, 1, "preheld after reinstall cannot toggle");
        assert!(event(&mut fresh, SPACE, true, ctrl(), &mut toggles));
        assert_eq!(toggles, 2);
    }

    #[test]
    fn bare_wrong_extra_modifiers_and_other_triggers_pass_whole_cycle() {
        for modifiers in [
            SearchModifiers::default(), alt(),
            SearchModifiers { shift: true, ..ctrl() },
            SearchModifiers { alt: true, ..ctrl() },
            SearchModifiers { win: true, ..ctrl() },
        ] {
            let mut state = ready("Ctrl+Space");
            let mut toggles = 0;
            assert!(!event(&mut state, SPACE, true, modifiers, &mut toggles));
            // Gaining the matching modifier during autorepeat is not fresh down.
            assert!(!event(&mut state, SPACE, true, ctrl(), &mut toggles));
            assert!(!event(&mut state, SPACE, false, ctrl(), &mut toggles));
            assert_eq!(toggles, 0);
            assert!(event(&mut state, SPACE, true, ctrl(), &mut toggles));
        }
        let mut state = ready("Ctrl+Space");
        let mut toggles = 0;
        for key in [K, b'D' as u32, 0xa2, 0xa3, 0xa4, 0xa5, 0x5b, 0x5c] {
            assert!(!event(&mut state, key, true, ctrl(), &mut toggles));
            assert!(!event(&mut state, key, false, ctrl(), &mut toggles));
        }
        assert_eq!(toggles, 0, "other standard chords and modifier keys pass");
    }

    #[test]
    fn preheld_trigger_on_install_or_rebind_waits_for_release() {
        let mut state = SearchKeyOwnership::default();
        state.set_binding(Some(binding("Ctrl+Space")), true);
        let mut toggles = 0;
        assert!(!event(&mut state, SPACE, true, ctrl(), &mut toggles));
        assert!(!event(&mut state, SPACE, false, ctrl(), &mut toggles));
        assert_eq!(toggles, 0);
        assert!(event(&mut state, SPACE, true, ctrl(), &mut toggles));
        assert!(event(&mut state, SPACE, false, ctrl(), &mut toggles));
        state.set_binding(Some(binding("Alt+K")), true);
        assert!(!event(&mut state, K, true, alt(), &mut toggles));
        assert!(!event(&mut state, K, false, alt(), &mut toggles));
        assert_eq!(toggles, 1);
        assert!(event(&mut state, K, true, alt(), &mut toggles));
        assert_eq!(toggles, 2);
    }

    #[test]
    fn ownership_outlives_rebind_and_disable_until_original_release() {
        let mut state = ready("Ctrl+Space");
        let mut toggles = 0;
        assert!(event(&mut state, SPACE, true, ctrl(), &mut toggles));
        state.set_binding(Some(binding("Alt+K")), false);
        assert!(event(&mut state, SPACE, true, SearchModifiers::default(), &mut toggles));
        assert!(event(&mut state, SPACE, false, SearchModifiers::default(), &mut toggles));
        assert!(!event(&mut state, SPACE, true, ctrl(), &mut toggles));
        assert!(!event(&mut state, SPACE, false, ctrl(), &mut toggles));
        assert!(event(&mut state, K, true, alt(), &mut toggles));
        state.set_binding(None, false);
        assert!(event(&mut state, K, false, SearchModifiers::default(), &mut toggles));
        assert!(!event(&mut state, K, true, alt(), &mut toggles));
        assert!(!event(&mut state, K, false, alt(), &mut toggles));
        assert_eq!(toggles, 2);
    }

    #[test]
    fn unavailable_search_or_non_search_binding_never_owns_input() {
        for configured in [None, Some(registry_binding("Ctrl+Space", ConfiguredHotkeyAction::Speech).unwrap())] {
            let mut state = SearchKeyOwnership::default();
            state.set_binding(configured, false);
            let mut toggles = 0;
            assert!(!event(&mut state, SPACE, true, ctrl(), &mut toggles));
            assert!(!event(&mut state, SPACE, false, ctrl(), &mut toggles));
            assert_eq!(toggles, 0, "only verified active Search may own");
        }
    }

    #[test]
    fn negative_hook_codes_never_queue_consume_or_mutate_state() {
        let mut state = ready("Ctrl+Space");
        assert!(!state.handle_event(-1, SPACE, true, ctrl(), || panic!("negative hook code queued")));
        let mut toggles = 0;
        assert!(event(&mut state, SPACE, true, ctrl(), &mut toggles));
        assert!(!state.handle_event(-1, SPACE, false, ctrl(), || panic!("negative hook code queued")));
        assert!(event(&mut state, SPACE, true, ctrl(), &mut toggles));
        assert!(event(&mut state, SPACE, false, ctrl(), &mut toggles));
        assert_eq!(toggles, 1, "negative release must not erase ownership");
    }

    #[test]
    fn queue_failure_passes_complete_cycle_without_partial_ownership_or_retry() {
        let mut state = ready("Ctrl+Space");
        let mut attempts = 0;
        assert!(!state.handle_event(0, SPACE, true, ctrl(), || { attempts += 1; false }));
        for _ in 0..10 {
            assert!(!state.handle_event(0, SPACE, true, ctrl(), || { attempts += 1; true }));
        }
        assert!(!state.handle_event(0, SPACE, false, ctrl(), || { attempts += 1; true }));
        assert_eq!(attempts, 1);
        assert!(state.handle_event(0, SPACE, true, ctrl(), || { attempts += 1; true }));
        assert_eq!(attempts, 2);
    }

    #[test]
    fn rebind_to_observed_preheld_other_key_does_not_steal_release() {
        let mut state = ready("Ctrl+Space");
        let mut toggles = 0;
        assert!(!event(&mut state, K, true, ctrl(), &mut toggles));
        state.set_binding(Some(binding("Ctrl+K")), true);
        assert!(!event(&mut state, K, true, ctrl(), &mut toggles));
        assert!(!event(&mut state, K, false, ctrl(), &mut toggles));
        assert_eq!(toggles, 0);
        assert!(event(&mut state, K, true, ctrl(), &mut toggles));
        assert_eq!(toggles, 1);
    }

    #[test]
    fn same_binding_refresh_does_not_forget_owned_or_unowned_down() {
        let mut state = ready("Ctrl+Space");
        let mut toggles = 0;
        assert!(event(&mut state, SPACE, true, ctrl(), &mut toggles));
        state.set_binding(Some(binding("Ctrl+Space")), true);
        assert!(event(&mut state, SPACE, true, ctrl(), &mut toggles));
        assert!(event(&mut state, SPACE, false, ctrl(), &mut toggles));
        assert!(!event(&mut state, SPACE, true, SearchModifiers::default(), &mut toggles));
        state.set_binding(Some(binding("Ctrl+Space")), true);
        assert!(!event(&mut state, SPACE, true, ctrl(), &mut toggles));
        assert!(!event(&mut state, SPACE, false, ctrl(), &mut toggles));
        assert_eq!(toggles, 1);
    }

    #[cfg(windows)]
    #[test]
    fn failed_rebind_uses_verified_rollback_not_attempted_search_chord() {
        let old = StandardHotkeyRegistry::from_settings(&StandardHotkeySettings::default()).unwrap();
        let mut settings = StandardHotkeySettings::default();
        settings.search.0 = "Ctrl+K".into();
        let next = StandardHotkeyRegistry::from_settings(&settings).unwrap();
        let mut active = old.bindings;
        let mut calls = 0;
        assert!(replace_registrations(&mut active, next, |_, _| {
            calls += 1;
            if calls == 1 { Err("new Search unavailable".into()) } else { Ok(()) }
        }, |_| {}).is_err());
        assert_eq!(active[0], old.bindings[0]);
        let mut state = SearchKeyOwnership::default();
        state.set_binding(active[0], false);
        let mut toggles = 0;
        assert!(!event(&mut state, K, true, ctrl(), &mut toggles));
        assert!(!event(&mut state, K, false, ctrl(), &mut toggles));
        assert!(event(&mut state, SPACE, true, ctrl(), &mut toggles));
        assert!(event(&mut state, SPACE, false, ctrl(), &mut toggles));
        assert_eq!(toggles, 1);
    }

    #[cfg(windows)]
    #[test]
    fn failed_search_rollback_disables_new_ownership_but_drains_owned_release() {
        let old = StandardHotkeyRegistry::from_settings(&StandardHotkeySettings::default()).unwrap();
        let mut settings = StandardHotkeySettings::default();
        settings.search.0 = "Ctrl+K".into();
        let next = StandardHotkeyRegistry::from_settings(&settings).unwrap();
        let mut active = old.bindings;
        let mut state = ready("Ctrl+Space");
        let mut toggles = 0;
        assert!(event(&mut state, SPACE, true, ctrl(), &mut toggles));
        let error = replace_registrations(&mut active, next, |_, _| Err("Search occupied".into()), |_| {}).unwrap_err();
        assert!(error.contains("rollback failed"));
        assert_eq!(active[0], None);
        for index in 1..active.len() { assert_eq!(active[index], old.bindings[index]); }
        state.set_binding(active[0], false);
        assert!(event(&mut state, SPACE, true, SearchModifiers::default(), &mut toggles));
        assert!(event(&mut state, SPACE, false, SearchModifiers::default(), &mut toggles));
        for key in [SPACE, K] {
            assert!(!event(&mut state, key, true, ctrl(), &mut toggles));
            assert!(!event(&mut state, key, false, ctrl(), &mut toggles));
        }
        assert_eq!(toggles, 1);
    }

    #[cfg(windows)]
    mod adapter {
        use super::*;
        use windows::Win32::UI::WindowsAndMessaging::PM_REMOVE;

        fn native_state() -> NativeState {
            let registry = StandardHotkeyRegistry::from_settings(&StandardHotkeySettings::default()).unwrap();
            assert_eq!(registry.bindings.len(), 8);
            NativeState::new(registry.clone(), registry.bindings)
        }

        fn capture_caller() -> crate::snipping::coordinator::Caller {
            crate::snipping::coordinator::Caller { label: "settings-panel".into(), window_id: 9001 }
        }

        fn install_search_fixture(dispatch: Arc<SearchDispatch>) -> Cleanup {
            SEARCH_HOOK.with(|slot| {
                assert!(slot.borrow().is_none());
                let mut pressed = [false; 256];
                // Seed an observed held trigger so refresh_search_binding takes
                // its short-circuit path, never samples the real desktop state.
                pressed[SPACE as usize] = true;
                *slot.borrow_mut() = Some(SearchHookState {
                    ownership: ready("Ctrl+Space"), pressed,
                    startup_observed: None, dispatch,
                });
            });
            Cleanup
        }

        fn fixture_event(down: bool) -> bool {
            SEARCH_HOOK.with(|slot| {
                let mut slot = slot.borrow_mut();
                let state = slot.as_mut().unwrap();
                state.pressed[SPACE as usize] = down;
                state.ownership.handle_event(0, SPACE, down, ctrl(), || state.dispatch.queue())
            })
        }

        #[test]
        fn capture_pause_blocks_new_acquisition_while_owned_release_remains_swallowed() {
            let dispatch = dispatch_for_this_test_thread();
            let _cleanup = install_search_fixture(dispatch.clone());
            let mut native = native_state();
            let now = std::time::Instant::now();
            assert!(fixture_event(true));
            assert_eq!(drain_search_messages(), 1);
            native.capture_transition(capture_caller(), "begin", 1, None, now, true).unwrap();
            refresh_search_state(&native);
            assert!(!dispatch.pending.load(Ordering::Acquire));
            assert!(dispatch.try_admit_emission().is_none());
            assert!(fixture_event(true), "pre-capture owned repeat still swallowed");
            assert!(fixture_event(false), "pre-capture owned release still swallowed");
            assert!(!fixture_event(true), "capture must not acquire a fresh trigger");
            assert!(!fixture_event(false));
            assert_eq!(drain_search_messages(), 0);
            let search = native.current[0].unwrap();
            let packed = search.key << 16 | MOD_CONTROL.0;
            let route = native.route_hotkey_observed(1, 1, packed, now, true);
            let HotkeyRoute::Capture(payload) = route else { panic!("registered Search must route only to capture lease"); };
            assert_eq!(payload["action"], "search");
        }

        #[test]
        fn capture_end_discards_queued_search_and_preheld_trigger_waits_for_fresh_cycle() {
            let dispatch = dispatch_for_this_test_thread();
            let _cleanup = install_search_fixture(dispatch.clone());
            let mut native = native_state();
            let now = std::time::Instant::now();
            assert!(fixture_event(true));
            assert!(fixture_event(false));
            let ack = native.capture_transition(capture_caller(), "begin", 1, None, now, true).unwrap();
            native.fence();
            refresh_search_state(&native);
            assert!(!dispatch.pending.load(Ordering::Acquire));
            assert_eq!(drain_search_messages(), 1, "old wake may remain, but it has no admission authority");
            assert!(!fixture_event(true), "trigger starts unowned during capture");
            native.capture_transition(capture_caller(), "end", 1, ack["leaseId"].as_str(), now, true).unwrap();
            native.fence();
            refresh_search_state(&native); // Observed preheld trigger avoids OS sampling.
            assert!(dispatch.try_admit_emission().is_none(), "old queued Search cannot replay after capture");
            assert!(!fixture_event(true), "preheld repeat cannot acquire halfway through gesture");
            assert!(!fixture_event(false));
            assert!(fixture_event(true));
            assert!(fixture_event(false));
            assert_eq!(drain_search_messages(), 1, "exactly one fresh post-capture toggle");
        }

        #[test]
        fn capture_fence_allows_prior_permit_but_drop_cannot_reopen_paused_or_retired_gate() {
            let dispatch = dispatch_for_this_test_thread();
            let _cleanup = install_search_fixture(dispatch.clone());
            let mut native = native_state();
            dispatch.pending.store(true, Ordering::Release);
            let permit = dispatch.try_admit_emission().unwrap();
            native.capture_transition(capture_caller(), "begin", 1, None, std::time::Instant::now(), true).unwrap();
            native.fence();
            refresh_search_state(&native);
            assert!(!dispatch.queue());
            assert!(dispatch.try_admit_emission().is_none());
            let mut emissions = 0;
            emissions += 1; // Only pre-fence permit owns this mock side effect.
            drop(permit);
            assert_eq!(emissions, 1);
            assert_eq!(dispatch.emission_gate.load(Ordering::Acquire), 4, "permit Drop preserves capture pause");
            assert!(!dispatch.queue());
            dispatch.retire();
            native.retire_observed();
            refresh_search_state(&native); // Preseeded pressed key, no native input sampling.
            dispatch.pending.store(true, Ordering::Release);
            assert!(dispatch.try_admit_emission().is_none(), "capture resume cannot erase permanent retirement");
            assert_eq!(drain_search_messages(), 0);
        }

        #[test]
        fn capture_expiry_resumes_only_verified_search_and_preserves_other_seven_slots() {
            let dispatch = dispatch_for_this_test_thread();
            let _cleanup = install_search_fixture(dispatch.clone());
            let mut native = native_state();
            let original = native.current;
            let now = std::time::Instant::now();
            native.capture_transition(capture_caller(), "begin", 1, None, now, true).unwrap();
            refresh_search_state(&native);
            assert!(native.expire_observed(now + std::time::Duration::from_secs(2), true));
            native.current[0] = None; // Verified partial rollback, not configured wishes.
            native.fence();
            refresh_search_state(&native);
            for index in 1..native.current.len() { assert_eq!(native.current[index], original[index]); }
            assert!(!fixture_event(true));
            assert!(!fixture_event(false));
            assert_eq!(drain_search_messages(), 0);
            SEARCH_HOOK.with(|slot| assert_eq!(slot.borrow().as_ref().unwrap().ownership.binding, None));
        }

        #[test]
        fn startup_gap_ctrl_release_does_not_make_bare_space_a_search_chord() {
            // Ctrl was down in the old pre-hook snapshot, released in the gap.
            // The new helper receives ONLY the authoritative post-install sample.
            let sampled_after_install = [false; 256];
            let observed = [None; 256];
            let pressed = reconcile_search_startup_keys(sampled_after_install, observed);
            assert!(!pressed[0xa2]);
            assert!(!pressed[0xa3]);
            let mut state = SearchKeyOwnership { held: pressed, ..SearchKeyOwnership::default() };
            state.set_binding(Some(binding("Ctrl+Space")), pressed[SPACE as usize]);
            let modifiers = SearchModifiers { ctrl: pressed[0xa2] || pressed[0xa3], ..SearchModifiers::default() };
            let mut toggles = 0;
            assert!(!event(&mut state, SPACE, true, modifiers, &mut toggles));
            assert!(!event(&mut state, SPACE, false, modifiers, &mut toggles));
            assert_eq!(toggles, 0, "stale startup Ctrl must never steal bare Space");
        }

        #[test]
        fn startup_reconciliation_preserves_callback_transitions_and_preheld_trigger() {
            let mut sampled = [false; 256];
            sampled[0xa2] = true;
            sampled[SPACE as usize] = true;
            let mut observed = [None; 256];
            // Physical callback release/down happened while snapshot was sampled.
            observed[0xa2] = Some(false);
            observed[0xa3] = Some(true);
            observed[b'K' as usize] = Some(true);
            let pressed = reconcile_search_startup_keys(sampled, observed);
            assert!(!pressed[0xa2], "observed release beats stale sampled down");
            assert!(pressed[0xa3], "observed press beats stale sampled up");
            assert!(pressed[b'K' as usize]);
            assert!(pressed[SPACE as usize], "sampled preheld trigger must seed held state");
            let mut state = SearchKeyOwnership { held: pressed, ..SearchKeyOwnership::default() };
            state.set_binding(Some(binding("Ctrl+Space")), pressed[SPACE as usize]);
            let mut toggles = 0;
            assert!(!event(&mut state, SPACE, true, ctrl(), &mut toggles));
            assert!(!event(&mut state, SPACE, false, ctrl(), &mut toggles));
            assert_eq!(toggles, 0);
            assert!(event(&mut state, SPACE, true, ctrl(), &mut toggles));
            assert_eq!(toggles, 1, "fresh released cycle remains usable");
        }

        #[test]
        fn retirement_wins_after_stale_live_read_before_emission_admission() {
            let dispatch = dispatch_for_this_test_thread();
            let (read_done, read_wait) = mpsc::sync_channel(0);
            let (resume, resume_wait) = mpsc::sync_channel(0);
            dispatch.pending.store(true, Ordering::Release);
            let candidate = dispatch.clone();
            let worker = thread::spawn(move || {
                assert!(!candidate.retired.load(Ordering::Acquire));
                read_done.send(()).unwrap();
                resume_wait.recv().unwrap();
                candidate.try_admit_emission().is_some()
            });
            read_wait.recv().unwrap();
            dispatch.retire();
            resume.send(()).unwrap();
            assert!(!worker.join().unwrap(), "stale false read grants no post-retirement emission");
            assert!(dispatch.try_admit_emission().is_none());
        }

        #[test]
        fn already_admitted_emission_can_finish_but_retirement_rejects_later_admission() {
            let dispatch = dispatch_for_this_test_thread();
            dispatch.pending.store(true, Ordering::Release);
            let (admitted, admitted_wait) = mpsc::sync_channel(0);
            let (resume, resume_wait) = mpsc::sync_channel(0);
            let emissions = Arc::new(std::sync::atomic::AtomicUsize::new(0));
            let count = emissions.clone();
            let candidate = dispatch.clone();
            let worker = thread::spawn(move || {
                let permit = candidate.try_admit_emission().expect("live queued dispatch is admitted");
                admitted.send(()).unwrap();
                resume_wait.recv().unwrap();
                count.fetch_add(1, Ordering::AcqRel); // Mock IPC side effect, not Tauri.
                drop(permit);
            });
            admitted_wait.recv().unwrap();
            let (retired, retired_wait) = mpsc::sync_channel(1);
            let retiring = dispatch.clone();
            let retirement = thread::spawn(move || { retiring.retire(); retired.send(()).unwrap(); });
            let retirement_completed = retired_wait.recv_timeout(std::time::Duration::from_secs(2));
            let rejected_in_flight = retirement_completed.is_ok()
                && dispatch.try_admit_emission().is_none();
            // Release even a broken lock-holding implementation, so a regression
            // reports an assertion instead of hanging the entire test process.
            resume.send(()).unwrap();
            worker.join().unwrap();
            retirement.join().unwrap();
            assert!(retirement_completed.is_ok(), "retirement cannot wait on already-admitted IPC");
            assert!(rejected_in_flight, "retired gate rejects while prior emission remains in flight");
            assert_eq!(emissions.load(Ordering::Acquire), 1, "only pre-retirement admitted dispatch may finish");
            // Make pending true again so this proves permit Drop preserves the
            // retired gate, rather than passing only because pending was cleared.
            dispatch.pending.store(true, Ordering::Release);
            assert!(dispatch.try_admit_emission().is_none(), "permit release cannot reopen retired admission");
        }

        fn dispatch_for_this_test_thread() -> Arc<SearchDispatch> {
            let mut message = MSG::default();
            // Create only this test thread's message queue; no hook/window exists.
            unsafe { let _ = PeekMessageW(&mut message, None, 0, 0, PM_NOREMOVE); }
            Arc::new(SearchDispatch {
                id: unsafe { GetCurrentThreadId() },
                pending: AtomicBool::new(false),
                retired: AtomicBool::new(false),
                emission_gate: std::sync::atomic::AtomicU8::new(0),
            })
        }

        fn drain_search_messages() -> usize {
            let mut count = 0;
            let mut message = MSG::default();
            while unsafe { PeekMessageW(&mut message, None, WM_SEARCH_TOGGLE, WM_SEARCH_TOGGLE, PM_REMOVE) }.as_bool() {
                count += 1;
            }
            count
        }

        struct Cleanup;
        impl Drop for Cleanup {
            fn drop(&mut self) {
                SEARCH_HOOK.with(|slot| *slot.borrow_mut() = None);
                drain_search_messages();
            }
        }

        #[test]
        fn production_queue_admits_one_message_and_rejects_pending_or_retired() {
            let dispatch = dispatch_for_this_test_thread();
            let _cleanup = Cleanup;
            assert!(dispatch.queue());
            assert!(dispatch.pending.load(Ordering::Acquire));
            assert!(!dispatch.queue(), "pending delivery cannot acquire another cycle");
            assert_eq!(drain_search_messages(), 1);
            dispatch.pending.store(false, Ordering::Release);
            assert!(dispatch.queue(), "settled delivery permits another message");
            assert_eq!(drain_search_messages(), 1);
            dispatch.pending.store(false, Ordering::Release);
            dispatch.retired.store(true, Ordering::Release);
            assert!(!dispatch.queue());
            assert!(!dispatch.pending.load(Ordering::Acquire));
            assert_eq!(drain_search_messages(), 0, "retired dispatcher must not post");
        }

        #[test]
        fn lifecycle_admission_rejects_overlap_without_holding_global_state_mutex() {
            // Only an empty ownership slot and an ordinary completed Rust worker;
            // never starts run_hotkey_thread or installs/uninstalls a native hook.
            let mut access = hotkey_access().unwrap();
            assert!(access.is_none());
            assert!(hotkey_access().is_err(), "overlapping lifecycle operation fails closed");
            assert!(hotkey_thread().try_lock().is_ok(), "global mutex is not held across work");
            let (sender, _) = mpsc::channel();
            let worker = thread::spawn(|| {});
            while !worker.is_finished() { thread::yield_now(); }
            *access = Some(HotkeyThread { id: 0, sender, worker, stopping: true });
            // Dropping access must retain the stopping handle, not free it early.
            drop(access);
            let mut restored = hotkey_access().unwrap();
            assert!(restored.as_ref().is_some_and(|state| state.stopping));
            reap_finished_thread(&mut restored).unwrap();
            assert!(restored.is_none(), "finished old worker is reaped before reinstall");
            drop(restored);
            assert!(hotkey_access().unwrap().is_none());
        }

        #[test]
        fn actual_consumed_callback_maps_both_modifier_sides_and_posts_once_per_cycle() {
            for (chord, modifier, down_message, up_message) in [
                ("Ctrl+Space", 0xa2, WM_KEYDOWN, WM_KEYUP),
                ("Ctrl+Space", 0xa3, WM_KEYDOWN, WM_KEYUP),
                ("Alt+Space", 0xa4, WM_SYSKEYDOWN, WM_SYSKEYUP),
                ("Alt+Space", 0xa5, WM_SYSKEYDOWN, WM_SYSKEYUP),
            ] {
                let dispatch = dispatch_for_this_test_thread();
                let _cleanup = Cleanup;
                SEARCH_HOOK.with(|slot| {
                    assert!(slot.borrow().is_none());
                    let mut pressed = [false; 256];
                    pressed[modifier] = true;
                    *slot.borrow_mut() = Some(SearchHookState {
                        ownership: ready(chord), pressed, startup_observed: None, dispatch: dispatch.clone(),
                    });
                });
                let input = KBDLLHOOKSTRUCT { vkCode: SPACE, ..KBDLLHOOKSTRUCT::default() };
                let pointer = LPARAM((&input as *const KBDLLHOOKSTRUCT) as isize);
                // Stack-owned callback data only: this does not send desktop input
                // or register the function with Windows. All calls take owned path.
                assert_eq!(unsafe { search_keyboard_hook(0, WPARAM(down_message as usize), pointer) }.0, 1);
                assert_eq!(drain_search_messages(), 1, "initial down posts immediately");
                // Clear pending as the worker would: repeats must still not queue.
                dispatch.pending.store(false, Ordering::Release);
                for _ in 0..20 {
                    assert_eq!(unsafe { search_keyboard_hook(0, WPARAM(down_message as usize), pointer) }.0, 1);
                }
                SEARCH_HOOK.with(|slot| {
                    let mut slot = slot.borrow_mut();
                    slot.as_mut().unwrap().pressed[modifier] = false;
                });
                assert_eq!(unsafe { search_keyboard_hook(0, WPARAM(up_message as usize), pointer) }.0, 1);
                assert_eq!(drain_search_messages(), 0, "repeats/up consumed, queued toggle count 1");
                SEARCH_HOOK.with(|slot| {
                    let slot = slot.borrow();
                    let state = slot.as_ref().unwrap();
                    assert!(!state.pressed[SPACE as usize]);
                    assert!(!state.ownership.owned[SPACE as usize]);
                });
            }
        }
    }
}
