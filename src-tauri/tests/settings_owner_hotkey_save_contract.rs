//! Test-only acceptance for the Settings Panel's owner-scoped hotkey save.
//! No app/window is created: OS-free transaction callbacks are exercised in
//! settings::tests; this file pins the command-to-transaction wiring.

use std::fs;
use std::path::PathBuf;

fn settings_source() -> String {
    fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/settings.rs"))
        .expect("settings source")
}

fn owner_save_handler(source: &str) -> &str {
    source
        .split("pub fn save_shell_settings(")
        .nth(1)
        .expect("Settings Panel save command")
        .split("fn settings_for_caller(")
        .next()
        .expect("owner-scoped settings save")
}

#[test]
fn settings_panel_explicit_hotkeys_reach_registration_aware_persistence_while_unrelated_saves_preserve_committed_hotkeys() {
    let source = settings_source();
    let handler = owner_save_handler(&source);
    let panel = handler
        .split("if window.label() == crate::contracts::surfaces::COMMAND_PANEL")
        .nth(1)
        .expect("owner-specific save paths")
        .split("fn settings_for_caller(")
        .next()
        .unwrap();

    assert!(
        handler.contains("SETTINGS_PANEL"),
        "Settings Panel must be authorized"
    );
    assert!(
        !panel.contains("update_shell_settings_for_app(&app_handle"),
        "Settings Panel save must not use the direct disk update that skips native registration"
    );
    assert!(
        panel.contains("save_settings_panel_for_app(&app_handle, settings)"),
        "Settings Panel must retain its owner-specific unrelated settings transaction"
    );
    let owner = source
        .split("fn save_settings_panel_for_app(")
        .nth(1)
        .expect("panel save helper")
        .split("// OS-free seam")
        .next()
        .unwrap();
    assert!(
        owner.contains("save_unrelated_settings(&path, settings, true)")
            && owner.contains("settings.hotkeys = previous.hotkeys")
            && owner.contains("save_settings_to_path"),
        "unrelated Settings Panel writes must persist committed hotkeys, never an unsaved draft"
    );
    let explicit = source.split("pub async fn save_standard_hotkeys(").nth(1).expect("explicit shortcut Save command")
        .split("// The explicit shortcut Save path.").next().unwrap();
    assert!(explicit.contains("settings_hotkey_worker(window, app_handle"));
    assert!(explicit.contains("save_standard_hotkeys_transaction("));
    assert!(explicit.contains("register_standard_hotkeys(caller.clone(), hotkeys.clone(), request.revision)"));
    assert!(explicit.contains("persist_settings_to_path(path, settings, MissingHotkeys::default())"));
    assert!(explicit.contains("restore_standard_hotkey_snapshot"),
        "rollback must restore verified slots, not configured legacy wishes");
    let transaction = source
        .split("fn save_standard_hotkeys_transaction<Snapshot>(")
        .nth(1)
        .expect("save transaction")
        .split("pub async fn begin_standard_hotkey_capture(")
        .next()
        .unwrap();
    assert!(
        transaction.contains("register(&settings.hotkeys)?")
            && transaction.contains("persist(path, settings)")
            && transaction.contains("restore(previous)"),
        "registration rejection must not write; disk failure must restore previous bindings"
    );
}

#[test]
fn settings_panel_save_preserves_private_quick_commands_and_redacts_response() {
    let source = settings_source();
    let handler = owner_save_handler(&source);
    let panel = handler
        .split("if window.label() == crate::contracts::surfaces::COMMAND_PANEL")
        .nth(1)
        .expect("owner-specific save paths");

    assert!(
        panel.contains("settings_for_caller(saved, window.label())"),
        "Settings Panel response must continue redacting Quick Command definitions, history and transcripts"
    );
    assert!(
        source.contains("settings.quick_commands = QuickCommandsSettings::default()"),
        "non-command-panel settings responses must redact private Quick Commands"
    );
    assert!(
        source.split("fn save_settings_panel_for_app(").nth(1).expect("panel save helper")
            .split("// OS-free seam").next().unwrap()
            .contains("settings.quick_commands = previous.quick_commands"),
        "Settings Panel must retain stored Quick Commands rather than persisting the redacted submitted defaults"
    );
}

#[test]
fn committed_shortcut_save_uses_trusted_status_without_postcommit_focus_reauthorization() {
    let source = settings_source();
    let explicit = source.split("pub async fn save_standard_hotkeys(").nth(1).unwrap()
        .split("// The explicit shortcut Save path.").next().unwrap();
    assert!(explicit.contains("settings_hotkey_worker(window, app_handle"), "initial concrete-instance authorization must remain");
    assert!(explicit.contains("register_standard_hotkeys(caller.clone(), hotkeys.clone(), request.revision)"), "registration admission must remain authorized");
    assert!(explicit.contains("standard_hotkey_save_response("), "command must use the committed response-phase seam");
    assert!(explicit.contains("crate::windows_key_hook::committed_standard_hotkey_status"), "already committed Save must use the trusted status snapshot even if Settings loses focus");
    assert!(!explicit.contains("standard_hotkey_status(caller)"), "postcommit focus reauthorization would falsely report a failed committed Save");
    let hook = fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/windows_key_hook.rs")).unwrap();
    let public_status = hook.split("pub(crate) fn standard_hotkey_status(").nth(1).unwrap().split("pub(crate) fn committed_standard_hotkey_status").next().unwrap();
    assert!(public_status.contains("registered_settings_window(app, &caller)?"), "public status must keep its exact-instance visible/focused guard");
    let trusted = hook.split("pub(crate) fn committed_standard_hotkey_status(").nth(1).unwrap().split("pub(crate) fn register_standard_hotkeys(").next().unwrap();
    assert!(trusted.contains("state.status()"));
    assert!(!trusted.contains("registered_settings_window"), "trusted postcommit snapshot must not introduce a new focus gate");
    let main = fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/main.rs")).unwrap();
    assert!(!main.contains("windows_key_hook::committed_standard_hotkey_status"), "trusted helper must not become a renderer command");
}
