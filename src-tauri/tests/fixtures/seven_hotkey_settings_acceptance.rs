// Included inside the actual settings module. OS-free persistence evidence;
// these tests never launch Tauri or register a real Windows shortcut.
mod seven_hotkey_acceptance {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT: AtomicU64 = AtomicU64::new(0);
    struct Fixture(PathBuf);
    impl Fixture {
        fn new(mut value: Value) -> Self {
            value["schema"] = serde_json::json!("jasonshell.settings");
            let root = PathBuf::from(std::env::var_os("LOCALAPPDATA").expect("Windows local app data"))
                .join("Temp/opencode");
            fs::create_dir_all(&root).unwrap();
            let file = Self(root.join(format!("seven-hotkeys-{}-{}.json", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed))));
            fs::write(&file.0, serde_json::to_vec(&value).unwrap()).unwrap();
            file
        }
        fn disk(&self) -> Value { serde_json::from_slice(&fs::read(&self.0).unwrap()).unwrap() }
    }
    impl Drop for Fixture { fn drop(&mut self) { let _ = fs::remove_file(&self.0); } }

    fn legacy(search: &str, missing_snipping: bool) -> Value {
        let mut value = serde_json::to_value(ShellSettings::default()).unwrap();
        let hotkeys = value["hotkeys"].as_object_mut().unwrap();
        hotkeys.remove("processManager");
        hotkeys.remove("settings");
        if missing_snipping { hotkeys.remove("snipping"); }
        hotkeys.insert("search".into(), serde_json::json!(search));
        value
    }

    #[test]
    fn seven_hotkeys_append_defaults_without_changing_existing_five() {
        let value = serde_json::to_value(ShellSettings::default()).unwrap();
        assert_eq!(value["hotkeys"], serde_json::json!({
            "search":"Ctrl+Space", "terminal":"Alt+Backquote", "stackBrowser":"Alt+1",
            "speechTranscription":"Ctrl+D", "snipping":"Alt+S", "processManager":"Alt+P", "settings":"Alt+O"
        }));
    }

    #[test]
    fn seven_hotkeys_missing_new_defaults_conflicting_with_old_customization_load_without_rewrite() {
        for search in ["Alt+P", "Alt+O"] {
            let file = Fixture::new(legacy(search, false));
            let before = fs::read(&file.0).unwrap();
            let loaded = load_settings_from_path(&file.0).expect("inferred default conflict must not reject old settings");
            assert_eq!(loaded.hotkeys.search.0, search);
            let wire = serde_json::to_value(loaded).unwrap();
            assert_eq!(wire["hotkeys"]["processManager"], "Alt+P");
            assert_eq!(wire["hotkeys"]["settings"], "Alt+O");
            assert_eq!(fs::read(&file.0).unwrap(), before);
        }
    }

    #[test]
    fn seven_hotkeys_unrelated_write_and_restart_preserve_missing_fields_even_with_mixed_legacy_conflicts() {
        for search in ["Alt+P", "Alt+O", "Alt+S"] {
            let file = Fixture::new(legacy(search, true));
            update_settings_to_path(&file.0, |settings| settings.ui.enable_diagnostics_export = true)
                .expect("unrelated writes must not require shortcut repair");
            let disk = file.disk();
            assert_eq!(disk["hotkeys"]["search"], search);
            for action in ["snipping", "processManager", "settings"] {
                assert!(disk["hotkeys"].get(action).is_none(), "{action} must stay omitted until explicit shortcut Save");
            }
            let restarted = load_settings_from_path(&file.0).unwrap();
            assert_eq!(restarted.hotkeys.search.0, search);
            assert!(restarted.ui.enable_diagnostics_export);
        }
    }

    #[test]
    fn seven_hotkeys_explicit_new_action_duplicates_are_strictly_rejected() {
        for action in ["processManager", "settings"] {
            let mut value = serde_json::to_value(ShellSettings::default()).unwrap();
            value["hotkeys"][action] = serde_json::json!("Ctrl+Space");
            let file = Fixture::new(value);
            assert!(load_settings_from_path(&file.0).is_err(), "explicit {action} duplicate must not gain inferred-default exemption");
        }
    }

    #[test]
    fn seven_hotkeys_new_default_omissions_do_not_exempt_explicit_snipping_duplicate() {
        let file = Fixture::new(legacy("Alt+S", false));
        assert!(load_settings_from_path(&file.0).is_err(), "new-field omissions cannot broaden the snipping exemption");
    }

    #[test]
    fn seven_hotkeys_every_unrelated_write_path_preserves_legacy_customization_and_omissions_after_restart() {
        for route in ["command-owner", "settings-panel", "top-height", "bottom-height", "top-lock", "bottom-lock", "quick-command-update"] {
            let mut value = legacy("Alt+P", true);
            value["hotkeys"]["terminal"] = serde_json::json!("Alt+O");
            value["hotkeys"]["stackBrowser"] = serde_json::json!("Alt+S");
            let file = Fixture::new(value);
            let previous = load_settings_from_path(&file.0).unwrap();
            match route {
                "command-owner" | "settings-panel" => {
                    let mut submitted = previous.clone();
                    submitted.ui.enable_diagnostics_export = true;
                    submitted.hotkeys.search.0 = "Alt+Z".into();
                    save_unrelated_settings(&file.0, submitted, route == "settings-panel").unwrap();
                }
                "top-height" | "bottom-height" => {
                    save_shell_bar_height_to_path(&file.0, SaveShellBarHeightRequest {
                        edge: if route == "top-height" { crate::appbar::ShellBarResizeEdge::Top } else { crate::appbar::ShellBarResizeEdge::Bottom },
                        height_logical: 45.0,
                    }).unwrap();
                }
                "top-lock" | "bottom-lock" => {
                    save_shell_bar_lock_to_path(&file.0, SaveShellBarLockRequest {
                        edge: if route == "top-lock" { crate::appbar::ShellBarResizeEdge::Top } else { crate::appbar::ShellBarResizeEdge::Bottom }, locked: false,
                    }).unwrap();
                }
                "quick-command-update" => {
                    update_settings_to_path(&file.0, |settings| {
                        settings.quick_commands.list_width = 300;
                        settings.hotkeys.search.0 = "Alt+Z".into();
                    }).unwrap();
                }
                _ => unreachable!(),
            }
            let restarted = load_settings_from_path(&file.0).unwrap();
            assert_eq!(restarted.hotkeys, previous.hotkeys, "{route} changed committed hotkeys");
            let disk = file.disk();
            for action in ["snipping", "processManager", "settings"] {
                assert!(disk["hotkeys"].get(action).is_none(), "{route} materialized {action}");
            }
            match route {
                "command-owner" | "settings-panel" => assert!(restarted.ui.enable_diagnostics_export),
                "top-height" => assert_eq!(restarted.ui.top_bar_height_logical, 45.0),
                "bottom-height" => assert_eq!(restarted.ui.bottom_bar_height_logical, 45.0),
                "top-lock" => assert!(!restarted.ui.lock_top_bar_height),
                "bottom-lock" => assert!(!restarted.ui.lock_bottom_bar_height),
                "quick-command-update" => assert_eq!(restarted.quick_commands.list_width, 300),
                _ => unreachable!(),
            }
        }
    }

    #[test]
    fn seven_hotkeys_explicit_repair_registers_before_disk_and_restart_has_all_seven() {
        use std::cell::RefCell;
        let file = Fixture::new(legacy("Alt+P", true));
        let before = fs::read(&file.0).unwrap(); let effects = RefCell::new(Vec::new());
        let mut repaired = load_settings_from_path(&file.0).unwrap().hotkeys;
        repaired.process_manager.0 = "Alt+X".into();
        save_standard_hotkeys_transaction(&file.0, repaired.clone(), |hotkeys| {
            assert_eq!(hotkeys, &repaired); assert_eq!(fs::read(&file.0).unwrap(), before);
            effects.borrow_mut().push("register"); Ok(())
        }, |path, settings| {
            assert_eq!(*effects.borrow(), vec!["register"]); effects.borrow_mut().push("persist");
            persist_settings_to_path(path, settings, MissingHotkeys::default())
        }, |_| panic!("successful repair must not restore")).unwrap();
        assert_eq!(*effects.borrow(), vec!["register", "persist"]);
        let disk = file.disk(); assert_eq!(disk["hotkeys"].as_object().unwrap().len(), 7);
        assert_eq!(disk["hotkeys"]["search"], "Alt+P");
        assert_eq!(load_settings_from_path(&file.0).unwrap().hotkeys, repaired);
    }

    #[test]
    fn seven_hotkeys_invalid_explicit_save_and_os_rejection_have_zero_disk_effects() {
        for failure in ["validate", "register"] {
            let file = Fixture::new(legacy("Alt+P", true)); let before = fs::read(&file.0).unwrap();
            let mut candidate = StandardHotkeySettings::default();
            if failure == "validate" { candidate.process_manager.0 = candidate.search.0.clone(); }
            let result = save_standard_hotkeys_transaction(&file.0, candidate,
                |_| -> Result<(), String> { assert_eq!(failure, "register"); Err("OS rejected candidate".into()) },
                |_, _| panic!("invalid or unregistered bindings must not reach disk"),
                |_| panic!("a failed registration owns its own rollback, not persistence rollback"));
            assert!(result.is_err()); assert_eq!(fs::read(&file.0).unwrap(), before);
        }
    }

    #[test]
    fn seven_hotkeys_persistence_failure_passes_exact_verified_snapshot_to_restore_and_reports_partial_truth() {
        use std::cell::RefCell;
        for partial in [false, true] {
            let file = Fixture::new(legacy("Alt+P", true)); let before = fs::read(&file.0).unwrap();
            // Opaque snapshot deliberately contains vacant inferred slots. The
            // transaction must pass it through, not reconstruct disk wishes.
            let snapshot = vec![Some("Alt+P"), Some("Alt+Backquote"), Some("Alt+1"), Some("Ctrl+D"), None, None, Some("Alt+O")];
            let trace = RefCell::new(Vec::new()); let expected = snapshot.clone();
            let result = save_standard_hotkeys_transaction(&file.0, StandardHotkeySettings::default(),
                |_| { trace.borrow_mut().push("register"); Ok(snapshot) },
                |_, _| { trace.borrow_mut().push("persist"); Err("disk write failed".into()) },
                |actual| { trace.borrow_mut().push("restore"); assert_eq!(actual, expected);
                    if partial { Err("previous hotkeys only partially restored: Search unavailable".into()) } else { Ok(()) }
                });
            let error = result.unwrap_err(); assert!(error.contains("disk write failed"));
            assert_eq!(error.contains("partially restored"), partial);
            if partial { assert!(error.contains("refresh shortcut status")); }
            assert_eq!(*trace.borrow(), vec!["register", "persist", "restore"]);
            assert_eq!(fs::read(&file.0).unwrap(), before);
        }
    }

    #[test]
    fn seven_hotkeys_explicit_save_request_requires_all_seven_fields_instead_of_disk_migration_defaults() {
        let hotkeys = serde_json::to_value(StandardHotkeySettings::default()).unwrap();
        let valid = serde_json::json!({"hotkeys": hotkeys, "revision": 1});
        assert!(serde_json::from_value::<SaveStandardHotkeysRequest>(valid.clone()).is_ok());
        for key in ["search", "terminal", "stackBrowser", "speechTranscription", "snipping", "processManager", "settings"] {
            let mut value = valid.clone(); value["hotkeys"].as_object_mut().unwrap().remove(key);
            assert!(serde_json::from_value::<SaveStandardHotkeysRequest>(value).is_err(), "explicit Save silently defaulted {key}");
        }
        let mut unknown = valid.clone(); unknown["hotkeys"]["arbitraryAction"] = serde_json::json!("Alt+X");
        assert!(serde_json::from_value::<SaveStandardHotkeysRequest>(unknown).is_err());
        let mut unknown_request = valid; unknown_request["caller"] = serde_json::json!("settings-panel");
        assert!(serde_json::from_value::<SaveStandardHotkeysRequest>(unknown_request).is_err());
    }

    #[test]
    fn seven_hotkeys_unrelated_panel_and_explicit_save_preserve_private_command_owner_data() {
        let mut value = legacy("Alt+P", true);
        value["quickCommands"]["listWidth"] = serde_json::json!(321);
        value["quickCommands"]["entries"] = serde_json::json!([{
            "id":"private", "label":"Private command", "mode":"direct", "targetPath":"cmd",
            "args":[], "commands":[], "cwd":null, "artifactLocation":null
        }]);
        let file = Fixture::new(value); let previous = load_settings_from_path(&file.0).unwrap();
        let mut redacted = previous.clone(); redacted.quick_commands = QuickCommandsSettings::default();
        redacted.ui.enable_diagnostics_export = true;
        let saved = save_unrelated_settings(&file.0, redacted, true).unwrap();
        assert_eq!(saved.quick_commands, previous.quick_commands);
        let mut repaired = saved.hotkeys; repaired.process_manager.0 = "Alt+X".into();
        let saved = save_standard_hotkeys_transaction(&file.0, repaired, |_| Ok(()),
            |path, settings| persist_settings_to_path(path, settings, MissingHotkeys::default()), |_| Ok(())).unwrap();
        assert_eq!(saved.quick_commands, previous.quick_commands);
        let visible = settings_for_caller(saved, "settings-panel");
        assert_eq!(visible.quick_commands, QuickCommandsSettings::default());
        assert_eq!(load_settings_from_path(&file.0).unwrap().quick_commands, previous.quick_commands);
    }

    #[test]
    fn seven_hotkeys_committed_transaction_survives_focus_loss_while_unauthorized_admission_has_no_disk_effect() {
        use std::cell::Cell;
        let file = Fixture::new(legacy("Alt+P", true)); let before = fs::read(&file.0).unwrap();
        let denied = save_standard_hotkeys_transaction(&file.0, StandardHotkeySettings::default(),
            |_| -> Result<(), String> { Err("Unauthorized Settings instance".into()) },
            |_, _| panic!("unauthorized admission must not persist"), |_| panic!("unauthorized admission cannot own a rollback snapshot"));
        assert!(denied.unwrap_err().contains("Unauthorized")); assert_eq!(fs::read(&file.0).unwrap(), before);
        let focused = Cell::new(true);
        let saved = save_standard_hotkeys_transaction(&file.0, StandardHotkeySettings::default(),
            |_| { assert!(focused.get()); Ok(()) },
            |path, settings| { let saved = persist_settings_to_path(path, settings, MissingHotkeys::default())?; focused.set(false); Ok(saved) },
            |_| panic!("focus loss after successful persistence cannot cause rollback")).unwrap();
        assert!(!focused.get()); assert_eq!(load_settings_from_path(&file.0).unwrap().hotkeys, saved.hotkeys);
        // Companion source-wiring test pins the actual command's postcommit
        // snapshot to committed_standard_hotkey_status, not public reauthorization.
    }

    #[test]
    fn seven_hotkeys_response_remains_committed_after_real_disk_write_and_failed_native_observation_without_fabricating_status() {
        for observe_fails in [false, true] {
            let mut value = legacy("Alt+P", true);
            value["quickCommands"]["listWidth"] = serde_json::json!(321);
            let file = Fixture::new(value);
            let response = standard_hotkey_save_response(
                || save_standard_hotkeys_transaction(&file.0, StandardHotkeySettings::default(),
                    |_| Ok(()), |path, settings| persist_settings_to_path(path, settings, MissingHotkeys::default()),
                    |_| panic!("successful publication cannot trigger rollback")),
                || {
                    assert_eq!(file.disk()["hotkeys"].as_object().unwrap().len(), 7, "observation must follow real disk commit");
                    if observe_fails { Err("secret-native-channel-diagnostic".into()) }
                    else { Ok(serde_json::json!({"revision":17,"bindings":[]})) }
                }).unwrap();
            assert_eq!(response["committed"], true);
            assert_eq!(response["settings"]["hotkeys"]["search"], "Ctrl+Space");
            assert_eq!(response["settings"]["quickCommands"]["listWidth"], 180, "response must retain owner redaction");
            assert_eq!(load_settings_from_path(&file.0).unwrap().quick_commands.list_width, 321, "private persisted owner data must survive");
            if observe_fails {
                assert!(response["status"].is_null(), "failed observation cannot fabricate active bindings or a revision");
                assert_eq!(response["statusError"], "Shortcuts saved; runtime status unavailable. Refresh shortcut status.");
                assert!(!response.to_string().contains("secret-native-channel-diagnostic"));
            } else {
                assert_eq!(response["status"]["revision"], 17); assert!(response["statusError"].is_null());
            }
        }
    }

    #[test]
    fn seven_hotkeys_response_never_observes_or_reports_commit_when_actual_transaction_admission_or_persistence_fails() {
        for failure in ["register", "persist"] {
            let file = Fixture::new(legacy("Alt+P", true)); let before = fs::read(&file.0).unwrap();
            let response = standard_hotkey_save_response(
                || save_standard_hotkeys_transaction(&file.0, StandardHotkeySettings::default(),
                    |_| if failure == "register" { Err("Unauthorized Settings instance".into()) } else { Ok(()) },
                    |_, _| Err("disk write failed".into()), |_| Ok(())),
                || panic!("precommit failure must never enter response observation"));
            assert!(response.is_err());
            assert_eq!(fs::read(&file.0).unwrap(), before);
        }
    }
}
