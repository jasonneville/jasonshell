// Child of production windows_key_hook; all OS operations are injected.
// No RegisterHotKey, AppBar or live Webview is created by this suite.
#[cfg(windows)]
mod seven_hotkey_registry_acceptance {
    use super::*;
    use std::cell::RefCell;

    fn registry() -> StandardHotkeyRegistry {
        StandardHotkeyRegistry::from_settings(&StandardHotkeySettings::default()).unwrap()
    }
    fn state(registry: StandardHotkeyRegistry) -> NativeState {
        NativeState::new(registry.clone(), registry.bindings)
    }

    fn caller(id: u64) -> crate::snipping::coordinator::Caller {
        crate::snipping::coordinator::Caller { label: "settings-panel".into(), window_id: id }
    }
    fn packed(binding: RegistryBinding) -> u32 {
        (binding.key << 16) | match binding.modifier { HotkeyModifier::Ctrl => MOD_CONTROL.0, HotkeyModifier::Alt => MOD_ALT.0 }
    }

    #[test]
    fn seven_hotkey_ids_append_six_and_seven_and_keep_no_repeat() {
        assert_eq!(HOTKEY_IDS, [1, 2, 3, 4, 5, 6, 7]);
        let r = registry();
        for (index, action) in [ConfiguredHotkeyAction::Search, ConfiguredHotkeyAction::Terminal,
            ConfiguredHotkeyAction::StackBrowser, ConfiguredHotkeyAction::Speech,
            ConfiguredHotkeyAction::Snipping, ConfiguredHotkeyAction::ProcessManager,
            ConfiguredHotkeyAction::Settings].into_iter().enumerate() {
            let binding = r.bindings[index].unwrap();
            assert_eq!(binding.action, action);
            assert!(modifiers(binding).contains(MOD_NOREPEAT));
        }
        assert_eq!(r.bindings[5].unwrap().key, b'P' as u32);
        assert_eq!(r.bindings[6].unwrap().key, b'O' as u32);
    }

    #[test]
    fn seven_hotkey_mixed_inferred_internal_conflicts_leave_exact_slots_vacant() {
        let mut settings = StandardHotkeySettings::default();
        settings.search.0 = "Alt+P".into(); settings.terminal.0 = "Alt+O".into(); settings.stack_browser.0 = "Alt+S".into();
        let r = StandardHotkeyRegistry::from_migration(&settings,
            MissingHotkeys { snipping: true, process_manager: true, settings: true }).unwrap();
        assert!(r.bindings[..4].iter().all(Option::is_some));
        assert_eq!(&r.bindings[4..], &[None, None, None]);
        assert_eq!(r.bindings[0].unwrap().key, b'P' as u32);
        assert_eq!(r.bindings[1].unwrap().key, b'O' as u32);
        assert_eq!(r.bindings[2].unwrap().key, b'S' as u32);
        assert!(StandardHotkeyRegistry::from_settings(&settings).is_err());
    }

    #[test]
    fn seven_hotkey_inferred_os_conflicts_do_not_fail_startup_or_unregister_old_actions() {
      for first_inferred in [4, 5] {
        let settings = StandardHotkeySettings::default();
        let r = StandardHotkeyRegistry::from_migration(&settings,
            MissingHotkeys { snipping: first_inferred == 4, process_manager: true, settings: true }).unwrap();
        let mut current = [None; 7]; let calls = RefCell::new(Vec::new());
        install_migrated_registrations(&mut current, &r, |index, _| {
            calls.borrow_mut().push(index);
            if index >= first_inferred { Err("another app owns inferred chord".into()) } else { Ok(()) }
        }, |_| panic!("tolerated inferred conflicts must not unregister old actions")).unwrap();
        assert_eq!(current[..first_inferred], r.bindings[..first_inferred]);
        assert!(current[first_inferred..].iter().all(Option::is_none));
        assert_eq!(*calls.borrow(), vec![0, 1, 2, 3, 4, 5, 6]);
        let mut native = state(r); native.current = current;
        let status = native.status();
        for index in 0..7 {
            assert_eq!(status["bindings"][index]["active"], index < first_inferred);
            assert_eq!(status["bindings"][index]["state"], if index < first_inferred { "active" } else { "inactive" });
        }
      }
    }

    #[test]
    fn seven_hotkey_explicit_os_conflict_remains_a_strict_startup_failure() {
        let r = registry(); let mut current = [None; 7]; let removed = RefCell::new(Vec::new());
        let result = install_migrated_registrations(&mut current, &r,
            |index, _| if index == 5 { Err("explicit OS conflict".into()) } else { Ok(()) },
            |index| removed.borrow_mut().push(index));
        assert!(result.is_err()); assert_eq!(current, [None; 7]);
        assert_eq!(*removed.borrow(), vec![0, 1, 2, 3, 4]);
    }

    #[test]
    fn seven_hotkey_failed_replacement_restores_exact_verified_vacancies() {
        let mut previous = registry(); previous.bindings[5] = None; previous.bindings[6] = None;
        let mut current = previous.bindings; let original = current;
        let mut candidate = StandardHotkeySettings::default(); candidate.search.0 = "Alt+X".into();
        let next = StandardHotkeyRegistry::from_settings(&candidate).unwrap();
        let attempts = RefCell::new(Vec::new());
        let result = replace_registrations(&mut current, next, |index, binding| {
            attempts.borrow_mut().push((index, binding.key));
            if index == 6 { Err("new Settings registration rejected".into()) } else { Ok(()) }
        }, |_| {});
        assert!(result.is_err()); assert_eq!(current, original);
        assert_eq!(current[5], None, "rollback must not activate a previously vacant PM slot");
        assert_eq!(current[6], None, "rollback must not activate a previously vacant Settings slot");
    }

    #[test]
    fn seven_hotkey_partial_replacement_rollback_exposes_only_verified_slots() {
        let previous = registry(); let mut current = previous.bindings;
        let mut candidate = StandardHotkeySettings::default(); candidate.search.0 = "Alt+X".into(); candidate.terminal.0 = "Alt+Y".into();
        let result = replace_registrations(&mut current, StandardHotkeyRegistry::from_settings(&candidate).unwrap(),
            |index, binding| if index == 1 || (index == 0 && binding.key == 0x20) { Err("OS collision".into()) } else { Ok(()) }, |_| {});
        assert!(result.unwrap_err().contains("rollback"));
        assert_eq!(&current[..2], &[None, None]); assert_eq!(current[2..], previous.bindings[2..]);
        let mut native = state(previous); native.current = current;
        let status = native.status(); assert_eq!(status["bindings"][0]["active"], false);
        assert_eq!(status["bindings"][1]["active"], false);
        assert!(status["bindings"][0]["reason"].is_string());
    }

    #[test]
    fn seven_hotkey_disk_failure_restore_never_restores_inferred_wishes() {
        let mut snapshot = registry(); snapshot.bindings[4] = None; snapshot.bindings[5] = None;
        let mut current = registry().bindings;
        restore_registrations(&mut current, &snapshot, |_, _| Ok(()), |_| {}).unwrap();
        assert_eq!(current, snapshot.bindings);
        assert_eq!(current[4], None); assert_eq!(current[5], None);
    }

    #[test]
    fn seven_hotkey_partial_disk_failure_restore_is_not_rolled_back_to_new_draft() {
        let snapshot = registry(); let mut current = snapshot.bindings;
        current[0] = Some(registry_binding("Alt+X", ConfiguredHotkeyAction::Search).unwrap());
        current[1] = Some(registry_binding("Alt+Y", ConfiguredHotkeyAction::Terminal).unwrap());
        let result = restore_registrations(&mut current, &snapshot,
            |index, _| if index == 0 { Err("old search claimed by other app".into()) } else { Ok(()) }, |_| {});
        assert!(result.unwrap_err().contains("partially restored"));
        assert_eq!(current[0], None); assert_eq!(current[1], snapshot.bindings[1]);
        assert_eq!(current[2..], snapshot.bindings[2..]);
    }

    #[test]
    fn seven_hotkey_capture_authorizes_exact_visible_focused_settings_instance_only() {
        let current = caller(10); let stale = caller(9);
        assert!(settings_eligibility(&current, Some(&current), true, true).is_ok());
        for registered in [None, Some(&stale)] { assert!(settings_eligibility(&current, registered, true, true).is_err()); }
        for (visible, focused) in [(false, true), (true, false), (false, false)] {
            assert!(settings_eligibility(&current, Some(&current), visible, focused).is_err());
        }
        let foreign = crate::snipping::coordinator::Caller { label: "process-manager".into(), window_id: 10 };
        assert!(settings_eligibility(&foreign, Some(&foreign), true, true).is_err());
        let mut native = state(registry());
        assert!(native.capture_transition(current, "begin", 1, None, std::time::Instant::now(), false).is_err());
        assert!(native.capture.is_none()); assert_eq!(native.revision, 1);
    }

    #[test]
    fn seven_hotkey_capture_ack_reports_only_verified_registered_chords_and_true_paused_status() {
        let mut r = registry(); r.bindings[5] = None;
        let mut native = state(r); let now = std::time::Instant::now();
        let ack = native.capture_transition(caller(1), "begin", 7, None, now, true).unwrap();
        assert_eq!(ack["revision"], 7); assert_eq!(ack["sequence"], 0); assert_eq!(ack["expiresInMs"], 2000);
        assert_eq!(ack["registeredBindings"].as_array().unwrap().len(), 6);
        assert!(!ack["registeredBindings"].as_array().unwrap().contains(&serde_json::json!("Alt+P")));
        let status = native.status(); assert_eq!(status["revision"], 2);
        for index in 0..7 { assert_eq!(status["bindings"][index]["state"], "capturePaused"); }
        assert_eq!(status["bindings"][5]["active"], false);
    }

    #[test]
    fn seven_hotkey_capture_redirects_every_registered_action_to_capture_not_dispatch() {
        let mut native = state(registry()); let now = std::time::Instant::now();
        let ack = native.capture_transition(caller(1), "begin", 3, None, now, true).unwrap();
        for index in 0..7 {
            let payload = match native.route_hotkey_observed(HOTKEY_IDS[index], 1, packed(native.current[index].unwrap()), now, true) {
                HotkeyRoute::Capture(payload) => payload,
                other => panic!("registered action escaped capture: {other:?}"),
            };
            assert_eq!(payload["leaseId"], ack["leaseId"]); assert_eq!(payload["revision"], 3);
            assert_eq!(payload["sequence"], index + 1);
            assert_eq!(payload["action"], native.status()["bindings"][index]["action"]);
            assert_eq!(payload["binding"], native.status()["bindings"][index]["binding"]);
        }
    }

    #[test]
    fn seven_hotkey_capture_rejects_stale_begin_renew_end_without_retiring_newer_lease() {
        let mut native = state(registry()); let now = std::time::Instant::now(); let owner = caller(1);
        let first = native.capture_transition(owner.clone(), "begin", 1, None, now, true).unwrap();
        let second = native.capture_transition(owner.clone(), "begin", 2, None, now, true).unwrap();
        assert_ne!(first["leaseId"], second["leaseId"]);
        for revision in [0, 1, 2, 9_007_199_254_740_992] {
            assert!(native.capture_transition(owner.clone(), "begin", revision, None, now, true).is_err());
        }
        for operation in ["renew", "end"] {
            for (who, rev, id) in [(owner.clone(), 1, first["leaseId"].as_str().unwrap()), (owner.clone(), 1, second["leaseId"].as_str().unwrap()), (caller(2), 2, second["leaseId"].as_str().unwrap())] {
                assert!(native.capture_transition(who, operation, rev, Some(id), now, true).is_err());
            }
        }
        assert_eq!(native.ack_at(now)["leaseId"], second["leaseId"]);
    }

    #[test]
    fn seven_hotkey_native_expiry_is_independent_and_renewal_is_bounded() {
        use std::time::Duration;
        let mut native = state(registry()); let now = std::time::Instant::now(); let owner = caller(1);
        let ack = native.capture_transition(owner.clone(), "begin", 1, None, now, true).unwrap();
        let id = ack["leaseId"].as_str().unwrap();
        let renewed = native.capture_transition(owner.clone(), "renew", 1, Some(id), now + Duration::from_millis(500), true).unwrap();
        assert_eq!(renewed["expiresInMs"], 2000);
        assert!(!native.expire_observed(now + Duration::from_millis(2499), true));
        let revision = native.revision;
        assert!(native.expire_observed(now + Duration::from_millis(2500), true));
        assert_eq!(native.revision, revision + 1); assert!(native.capture.is_none());
        assert!(!native.expire_observed(now + Duration::from_secs(3), true));
        assert!(native.capture_transition(owner, "renew", 1, Some(id), now + Duration::from_secs(3), true).is_err());
    }

    #[test]
    fn seven_hotkey_focus_hide_destroy_retirement_discards_triggering_event_and_never_replays_it() {
        let now = std::time::Instant::now();
        for lifecycle in ["blur", "hide", "native focus loss", "destroy", "dispose"] {
            let mut native = state(registry()); native.capture_transition(caller(1), "begin", 1, None, now, true).unwrap();
            let chord = packed(native.current[5].unwrap());
            assert_eq!(native.route_hotkey_observed(6, 20, chord, now, false), HotkeyRoute::Discard, "{lifecycle}");
            assert!(native.capture.is_none());
            // Production fences immediately after the observed retirement.
            native.fence = 20;
            assert_eq!(native.route_hotkey_observed(6, 20, chord, now, true), HotkeyRoute::Discard);
            assert_eq!(native.route_hotkey_observed(6, 19, chord, now, true), HotkeyRoute::Discard);
            assert_eq!(native.route_hotkey_observed(6, 21, chord, now, true), HotkeyRoute::Dispatch(6));
        }
    }

    #[test]
    fn seven_hotkey_queue_fence_rejects_stale_mismatched_unknown_and_vacant_messages() {
        let mut native = state(registry()); let now = std::time::Instant::now(); native.fence = 100;
        let chord = packed(native.current[6].unwrap());
        for stamp in [99, 100] { assert_eq!(native.route_hotkey_observed(7, stamp, chord, now, true), HotkeyRoute::Discard); }
        assert_eq!(native.route_hotkey_observed(7, 101, chord ^ 1, now, true), HotkeyRoute::Discard);
        assert_eq!(native.route_hotkey_observed(99, 101, chord, now, true), HotkeyRoute::Discard);
        native.current[6] = None;
        assert_eq!(native.route_hotkey_observed(7, 101, chord, now, true), HotkeyRoute::Discard);
        // Windows uptime rollover uses signed wrapping comparison, not >.
        native.fence = u32::MAX - 5;
        let pm = packed(native.current[5].unwrap());
        assert_eq!(native.route_hotkey_observed(6, 3, pm, now, true), HotkeyRoute::Dispatch(6));
    }

    #[test]
    fn seven_hotkey_new_registered_instance_has_new_button_domain_but_old_lease_cannot_end_it() {
        let mut native = state(registry()); let now = std::time::Instant::now();
        let old = native.capture_transition(caller(1), "begin", 100, None, now, true).unwrap();
        native.retire_observed();
        let current = native.capture_transition(caller(2), "begin", 1, None, now, true).unwrap();
        assert_ne!(old["leaseId"], current["leaseId"]);
        assert!(native.capture_transition(caller(1), "end", 100, old["leaseId"].as_str(), now, true).is_err());
        assert_eq!(native.ack_at(now)["leaseId"], current["leaseId"]);
    }

    #[test]
    fn seven_hotkey_ineligible_retired_instance_cannot_cancel_newer_registered_owner_lease() {
        let mut native = state(registry()); let now = std::time::Instant::now();
        let old = native.capture_transition(caller(1), "begin", 9, None, now, true).unwrap();
        let current = native.capture_transition(caller(2), "begin", 1, None, now, true).unwrap();
        let revision = native.revision;
        for operation in ["begin", "renew", "end"] {
            assert!(native.capture_transition(caller(1), operation, 9, old["leaseId"].as_str(), now, false).is_err());
            assert_eq!(native.ack_at(now)["leaseId"], current["leaseId"]);
            assert_eq!(native.revision, revision);
        }
        // The same observation for the actual current owner does retire it.
        assert!(native.capture_transition(caller(2), "renew", 1, current["leaseId"].as_str(), now, false).is_err());
        assert!(native.capture.is_none()); assert!(native.revision > revision);
    }

    #[test]
    fn seven_hotkey_ancient_uptime_fence_cannot_disable_new_hotkeys_after_long_idle() {
        use std::time::Duration;
        let mut native = state(registry()); let now = std::time::Instant::now();
        native.fence = 100; native.fence_time = now;
        let chord = packed(native.current[5].unwrap());
        assert_eq!(native.route_hotkey_observed(6, 99, chord, now, true), HotkeyRoute::Discard);
        let later = now + Duration::from_millis(i32::MAX as u64);
        assert_eq!(native.route_hotkey_observed(6, 99, chord, later, true), HotkeyRoute::Dispatch(6));
    }
}
