mod snip_acceptance {
    use super::*;
    #[test]
    fn snip_hotkey_registry_preserves_ids_one_to_four_adds_fifth_no_repeat_alt_s() {
        let settings=crate::settings::StandardHotkeySettings::default();
        let registry=StandardHotkeyRegistry::from_settings(&settings).unwrap();
        assert_eq!(HOTKEY_IDS,[1,2,3,4,5]);
        assert_eq!(registry.bindings.len(),5);
        // Amended registry stores optional fifth; original four remain present in order.
        for (slot,action) in [ConfiguredHotkeyAction::Search,ConfiguredHotkeyAction::Terminal,
            ConfiguredHotkeyAction::StackBrowser,ConfiguredHotkeyAction::Speech,ConfiguredHotkeyAction::Snipping].into_iter().enumerate() {
            let binding=registry.bindings[slot].expect("conflict-free slots must all be present");
            assert_eq!(binding.action,action);
            assert!(modifiers(binding).contains(MOD_NOREPEAT));
        }
        assert_eq!(registry.bindings[4].unwrap().key,b'S' as u32);
        assert_eq!(registry.bindings[4].unwrap().modifier,HotkeyModifier::Alt);
    }
    #[test]
    fn snip_hotkey_legacy_conflict_leaves_fifth_vacant_until_repaired() {
        let mut settings=crate::settings::StandardHotkeySettings::default(); settings.search.0="Alt+S".into();
        // Narrow production registry seam requested for legacy startup. Alias may
        // be negotiated; it must call the SAME registry builder used by startup.
        let loaded=StandardHotkeyRegistry::from_loaded_settings(&settings,true).unwrap();
        assert!(loaded.bindings[4].is_none());
        for slot in 0..4 { assert!(loaded.bindings[slot].is_some()); }
        assert_eq!(loaded.bindings[0].unwrap().key,b'S' as u32);
        assert!(StandardHotkeyRegistry::from_loaded_settings(&settings,false).is_err());
        assert!(StandardHotkeyRegistry::from_settings(&settings).is_err());
        settings.snipping.0="Alt+X".into();
        let repaired=StandardHotkeyRegistry::from_settings(&settings).unwrap();
        assert_eq!(repaired.bindings[4].unwrap().key,b'X' as u32);
        for slot in 0..4 { assert_eq!(loaded.bindings[slot],repaired.bindings[slot]); }
    }
}
