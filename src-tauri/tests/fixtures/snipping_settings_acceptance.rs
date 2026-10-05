// Child of ACTUAL settings module, giving acceptance access to existing private
// load/validate/transaction seams without production visibility edits or mock validator.
mod snip_acceptance {
    use super::*;
    use std::{cell::RefCell, sync::atomic::{AtomicU64, Ordering}};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    struct File(PathBuf);
    impl File {
        fn new() -> Self {
            let root=PathBuf::from(std::env::var_os("LOCALAPPDATA").unwrap()).join("Temp/opencode");
            let path=root.join(format!("snip-settings-{}-{}.json",std::process::id(),NEXT.fetch_add(1,Ordering::Relaxed)));
            Self(path)
        }
        fn write(&self, value: &serde_json::Value) { fs::write(&self.0,serde_json::to_vec(value).unwrap()).unwrap(); }
    }
    impl Drop for File { fn drop(&mut self) { let _=fs::remove_file(&self.0); } }
    fn legacy(conflict: bool) -> serde_json::Value {
        let mut value=serde_json::to_value(ShellSettings::default()).unwrap();
        value["hotkeys"].as_object_mut().unwrap().remove("snipping");
        if conflict { value["hotkeys"]["search"]=serde_json::json!(" alt + s "); }
        value
    }
    #[test]
    fn snip_hotkey_default_fifth_preserves_old_four_canonical_chords() {
        let h=ShellSettings::default().hotkeys;
        assert_eq!((&*h.search.0,&*h.terminal.0,&*h.stack_browser.0,&*h.speech_transcription.0),
            ("Ctrl+Space","Alt+Backquote","Alt+1","Ctrl+D"));
        assert_eq!(h.snipping.0,"Alt+S");
    }
    #[test]
    fn snip_hotkey_legacy_missing_fifth_load_adds_default_without_file_rewrite() {
        for conflict in [false,true] {
            let file=File::new(); file.write(&legacy(conflict)); let before=fs::read(&file.0).unwrap();
            let loaded=load_settings_from_path(&file.0).unwrap();
            assert_eq!(loaded.hotkeys.snipping.0,"Alt+S");
            assert_eq!(loaded.hotkeys.search.0,if conflict {"Alt+S"} else {"Ctrl+Space"});
            assert_eq!(loaded.hotkeys.terminal.0,"Alt+Backquote");
            assert_eq!(loaded.hotkeys.stack_browser.0,"Alt+1"); assert_eq!(loaded.hotkeys.speech_transcription.0,"Ctrl+D");
            assert_eq!(fs::read(&file.0).unwrap(),before,"migration must not rewrite persisted original chords");
            assert_eq!(validate_settings(loaded).is_err(),conflict,"ordinary saves remain strict despite load exception");
        }
    }
    #[test]
    fn snip_hotkey_explicit_five_way_canonical_duplicate_is_rejected_not_defaulted() {
        let mut s=ShellSettings::default(); s.hotkeys.snipping.0=" alt + s ".into(); s.hotkeys.search.0="Alt+S".into();
        assert!(validate_settings(s.clone()).is_err());
        let file=File::new(); file.write(&serde_json::to_value(s).unwrap());
        assert!(load_settings_from_path(&file.0).is_err(),"explicit duplicate must not gain legacy exception or silently fall back");
    }
    #[test]
    fn snip_hotkey_unrelated_save_of_legacy_conflict_never_registers_or_persists() {
        let file=File::new(); file.write(&legacy(true)); let before=fs::read(&file.0).unwrap();
        let mut submitted=load_settings_from_path(&file.0).unwrap(); submitted.ui.enable_diagnostics_export=true;
        let effects=RefCell::new(Vec::new());
        let result=save_settings_transaction(&file.0,submitted,|s,_|s,
            |_| { effects.borrow_mut().push("register"); Ok(()) },
            |_,s| { effects.borrow_mut().push("persist"); Ok(s) });
        assert!(result.is_err()); assert!(effects.borrow().is_empty()); assert_eq!(fs::read(&file.0).unwrap(),before);
    }
    #[test]
    fn snip_hotkey_repair_registers_before_persist_and_failure_restores_exact_prior_chords() {
        for fail in ["none","register","persist"] {
            let file=File::new(); file.write(&legacy(true)); let before=fs::read(&file.0).unwrap();
            let old=load_settings_from_path(&file.0).unwrap().hotkeys;
            let mut candidate=load_settings_from_path(&file.0).unwrap(); candidate.hotkeys.snipping.0="Alt+X".into();
            let active=RefCell::new(old.clone()); let trace=RefCell::new(Vec::new());
            let result=save_settings_transaction(&file.0,candidate,|s,_|s,
                |h| { trace.borrow_mut().push("register"); if fail=="register" { return Err("synthetic register rejection".into()); }
                    *active.borrow_mut()=h.clone(); Ok(()) },
                |path,s| { trace.borrow_mut().push("persist"); if fail=="persist" { return Err("synthetic persist rejection".into()); }
                    fs::write(path,serde_json::to_vec(&s).unwrap()).unwrap(); Ok(s) });
            if fail=="none" { assert!(result.is_ok()); assert_eq!(*trace.borrow(),vec!["register","persist"]); assert_eq!(active.borrow().snipping.0,"Alt+X"); }
            else { assert!(result.is_err()); assert_eq!(*active.borrow(),old); assert_eq!(fs::read(&file.0).unwrap(),before);
                assert_eq!(*trace.borrow(),if fail=="register" {vec!["register"]} else {vec!["register","persist","register"]}); }
        }
    }
}
