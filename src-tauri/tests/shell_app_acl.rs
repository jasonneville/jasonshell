//! Actual Tauri ACL resolver + generated production manifests/capabilities.
//! No Builder, WebviewWindow, custom handler, window activation or shell startup.
use std::{collections::{BTreeMap,BTreeSet},fs,path::PathBuf};
use tauri::{ipc::{Origin,RuntimeAuthority},utils::{acl::{manifest::Manifest,capability::Capability,resolved::Resolved},platform::Target}};
const LEGACY_SURFACES:&[&str]=&["top-bar","bottom-bar","quick-launch-panel","task-gallery","task-preview",
    "search-panel","stack-popup","process-manager","control-plane","settings-panel","tray-panel",
    "terminal-panel","command-panel","audio-panel","calendar-panel","speech-history-panel","speech-indicator","context-menu-overlay"];
const SNIP:&[&str]=&["start_snip","get_snip_context","get_snip_image","snip_ready","snip_begin_selection",
    "complete_snip","cancel_snip","copy_snip","save_snip","dismiss_snip","snip_bar_ready"];
fn read(relative:&str)->String {fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)).unwrap()}
fn authority()->(RuntimeAuthority,bool) {
    // SAME generated inputs and SAME Resolved/RuntimeAuthority used by Tauri's
    // webview IPC gate. Do not substitute a hand-authored permission validator.
    let manifests:BTreeMap<String,Manifest>=serde_json::from_str(&read("gen/schemas/acl-manifests.json")).unwrap();
    let capabilities:BTreeMap<String,Capability>=serde_json::from_str(&read("gen/schemas/capabilities.json")).unwrap();
    let resolved=Resolved::resolve(&manifests,capabilities,Target::Windows).unwrap();
    let enabled=resolved.has_app_acl;
    (RuntimeAuthority::new(manifests,resolved),enabled)
}
fn allowed(authority:&RuntimeAuthority,command:&str,surface:&str)->bool {
    authority.resolve_access(command,surface,surface,&Origin::Local).is_some()
}
fn registered_commands()->BTreeSet<String> {
    // This is inventory ONLY, not an authorization oracle: resolution above uses
    // actual generated ACL. Tauri's registered handler list is the coverage floor.
    let source=read("src/main.rs");
    let list=source.split(".invoke_handler(tauri::generate_handler![").nth(1).expect("actual invoke handler inventory")
        .split("])").next().unwrap();
    list.split(',').map(str::trim).filter(|s|!s.is_empty()).map(|s|{
        let command=s.rsplit("::").next().unwrap();
        assert!(command.bytes().all(|c|c.is_ascii_lowercase()||c.is_ascii_digit()||c==b'_'),"unparsed handler: {s}");
        command.to_owned()
    }).collect()
}
#[test]
fn folder_import_registered_and_settings_authorized_with_remote_unknown_snip_denial() {
    let command="import_speech_model_folder";
    let (authority,enabled)=authority();assert!(enabled);
    // Actual resolver positive FIRST: catches missing generated application grant.
    assert!(allowed(&authority,command,"settings-panel"),"new folder import missing Settings application ACL grant");
    assert!(registered_commands().contains(command),"new folder import must be an actual registered handler");
    for label in ["unknown","snip-overlay-7-m0","snip-preview-7"] {assert!(!allowed(&authority,command,label));}
    let remote=Origin::Remote{url:"https://not-authorized.invalid/".parse().unwrap()};
    assert!(authority.resolve_access(command,"settings-panel","settings-panel",&remote).is_none());
    for command in SNIP {assert!(!allowed(&authority,command,"settings-panel"));}
}
#[test]
fn existing_surface_ipc_resolves_before_handlers_without_broadening_snipping() {
    let (authority,enabled)=authority();assert!(enabled,"snipping command capability enforcement must stay enabled");
    let required:&[(&str,&[&str])]=&[
        ("bottom-bar",&["list_open_task_windows","activate_task_window","list_pinned_taskbar_apps","launch_pinned_taskbar_app",
            "show_task_window_preview","hide_task_window_preview","show_task_window_context_menu"]),
        ("top-bar",&["list_pinned_taskbar_apps","launch_pinned_taskbar_app","show_top_bar_pin_context_menu",
            "show_terminal_panel","show_command_panel","show_tray_panel","show_audio_panel","show_calendar_panel",
            "show_centered_search_panel","load_shell_settings"]),
        ("settings-panel",&["load_shell_settings","save_shell_settings"]),
        ("process-manager",&["list_processes"]),
        ("audio-panel",&["get_audio_state","list_audio_devices","list_audio_sessions"]),
        ("stack-popup",&["read_stack_folder"]),
    ];
    let registered=registered_commands();let mut denied=Vec::new();
    for (surface,commands) in required {for command in *commands {
        assert!(registered.contains(*command),"test must use actual registered command {command}");
        if !allowed(&authority,command,surface) {denied.push(format!("{surface} -> {command}: DENIED by Tauri RuntimeAuthority before handler"));}
    }}
    assert!(denied.is_empty(),"existing shell IPC regression:\n{}",denied.join("\n"));
}
#[test]
fn every_registered_legacy_command_has_explicit_existing_surface_acl_coverage() {
    let (authority,enabled)=authority();assert!(enabled);
    let registered=registered_commands();assert!(registered.len()>100,"coverage inventory unexpectedly shrank");
    for command in SNIP {assert!(registered.contains(*command));}
    let missing:Vec<_>=registered.iter().filter(|cmd|!SNIP.contains(&cmd.as_str()))
        .filter(|cmd|!LEGACY_SURFACES.iter().any(|surface|allowed(&authority,cmd,surface))).cloned().collect();
    assert!(missing.is_empty(),"registered legacy commands have NO resolved existing-surface grant (handler inventory {}, missing {}):\n{}",
        registered.len(),missing.len(),missing.join(", "));
}
#[test]
fn actual_resolver_preserves_snip_surface_partition_and_remote_denial() {
    let (authority,enabled)=authority();assert!(enabled);
    let overlay="snip-overlay-7-m0";let preview="snip-preview-7";
    let expected=[("top-bar",vec!["start_snip","snip_bar_ready"]),
        (overlay,vec!["get_snip_context","get_snip_image","snip_ready","snip_begin_selection","complete_snip","cancel_snip"]),
        (preview,vec!["get_snip_context","get_snip_image","copy_snip","save_snip","dismiss_snip"])];
    for (surface,permitted) in expected {for command in SNIP {
        assert_eq!(allowed(&authority,command,surface),permitted.contains(command),"{surface} -> {command}: exact partition");
    }}
    for surface in LEGACY_SURFACES.iter().copied().filter(|s|*s!="top-bar").chain(["unknown","future-panel","snip-overlay","snip-preview"]) {
        for command in SNIP {assert!(!allowed(&authority,command,surface),"unrelated surface {surface} gained {command}");}
    }
    let remote=Origin::Remote {url:"https://not-authorized.invalid/".parse().unwrap()};
    for surface in LEGACY_SURFACES.iter().copied().chain([overlay,preview]) {for command in registered_commands() {
        assert!(authority.resolve_access(&command,surface,surface,&remote).is_none(),"remote origin gained {surface} -> {command}");
    }}
}
#[test]
fn resolved_app_command_grants_never_use_global_window_or_webview_wildcard() {
    let manifests:BTreeMap<String,Manifest>=serde_json::from_str(&read("gen/schemas/acl-manifests.json")).unwrap();
    let capabilities:BTreeMap<String,Capability>=serde_json::from_str(&read("gen/schemas/capabilities.json")).unwrap();
    let resolved=Resolved::resolve(&manifests,capabilities,Target::Windows).unwrap();
    for (command,grants) in resolved.allowed_commands {if !command.starts_with("plugin:") {for grant in grants {
        assert!(!grant.windows.iter().chain(&grant.webviews).any(|pattern|pattern.as_str()=="*"),"global app grant: {command}");
    }}}
    let (authority,_)=authority();
    for command in registered_commands() {for surface in ["unknown","future-panel","not-a-shell-window"] {
        assert!(!allowed(&authority,&command,surface),"unknown/future surface gained {command}");
    }}
}
