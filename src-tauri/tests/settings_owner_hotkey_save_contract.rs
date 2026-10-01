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
fn settings_panel_hotkeys_reach_registration_aware_persistence() {
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
        "Settings Panel must pass the submitted hotkeys into its owner-specific transaction"
    );
    let owner = source
        .split("fn save_settings_panel_for_app(")
        .nth(1)
        .expect("panel save helper")
        .split("// OS-free seam")
        .next()
        .unwrap();
    assert!(
        owner.contains("save_settings_transaction(")
            && owner.contains("configure_standard_hotkeys(app_handle, hotkeys)")
            && owner.contains("save_settings_to_path"),
        "Settings Panel must register hotkeys and persist via the shared transaction"
    );
    let transaction = source
        .split("fn save_settings_transaction(")
        .nth(1)
        .expect("save transaction")
        .split("pub(crate) fn update_shell_settings_for_app(")
        .next()
        .unwrap();
    assert!(
        transaction.contains("configure(&settings.hotkeys)?")
            && transaction.contains("persist(path, settings.clone())")
            && transaction.contains("configure(&previous.hotkeys)"),
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
            .contains("settings.quick_commands = previous.quick_commands.clone()"),
        "Settings Panel must retain stored Quick Commands rather than persisting the redacted submitted defaults"
    );
}
