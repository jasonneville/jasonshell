//! P03-only native WebView2 mount. Never linked into product binary.
#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]
use serde::{Deserialize, Serialize};
use std::{fs::{File, OpenOptions}, io::Write, path::{Path, PathBuf}, sync::Mutex};

const MAX_EVENTS: usize = 4096;
const MAX_EVENT_BYTES: usize = 16 * 1024;
const P03_EXPERIMENT_ONLY: &str = "--p03-experiment";

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Event { kind: String, time_ms: f64, metadata: serde_json::Value }

struct P03State { events: Mutex<File>, count: Mutex<usize>, run_dir: PathBuf }

fn authorize(window: &tauri::WebviewWindow) -> Result<(), String> {
    if window.label() != "p03-experiment" { return Err("Unauthorized".into()); }
    Ok(())
}

#[tauri::command]
fn p03_record(window: tauri::WebviewWindow, state: tauri::State<'_, P03State>, event: Event) -> Result<(), String> {
    authorize(&window)?;
    if event.kind.is_empty() || event.kind.len() > 64 || !event.time_ms.is_finite() { return Err("InvalidEvidence".into()); }
    let bytes = serde_json::to_vec(&event).map_err(|_| "InvalidEvidence")?;
    if bytes.len() > MAX_EVENT_BYTES { return Err("ResourceLimit".into()); }
    let mut count = state.count.lock().map_err(|_| "IoFailure")?;
    if *count >= MAX_EVENTS { return Err("ResourceLimit".into()); }
    let mut file = state.events.lock().map_err(|_| "IoFailure")?;
    file.write_all(&bytes).and_then(|_| file.write_all(b"\n")).and_then(|_| file.flush()).map_err(|_| "IoFailure")?;
    *count += 1;
    Ok(())
}

#[tauri::command]
fn p03_save_summary(window: tauri::WebviewWindow, state: tauri::State<'_, P03State>, summary: serde_json::Value) -> Result<(), String> {
    authorize(&window)?;
    let bytes = serde_json::to_vec_pretty(&summary).map_err(|_| "InvalidEvidence")?;
    if bytes.len() > MAX_EVENT_BYTES { return Err("ResourceLimit".into()); }
    let path = state.run_dir.join("p03-summary.json");
    let mut file = OpenOptions::new().write(true).create_new(true).open(path).map_err(|_| "EvidenceAlreadyExists")?;
    file.write_all(&bytes).and_then(|_| file.sync_all()).map_err(|_| "IoFailure".into())
}

fn parse_args(mut args: impl Iterator<Item = String>) -> Result<PathBuf, &'static str> {
    if args.next().as_deref() != Some(P03_EXPERIMENT_ONLY) { return Err("P03_EXPERIMENT_ONLY guard required"); }
    let path = args.next().ok_or("usage: stack_text_p03 --p03-experiment RUN_DIRECTORY")?;
    if args.next().is_some() { return Err("unexpected argument"); }
    let path = PathBuf::from(path);
    if !path.is_absolute() || !Path::new(&path).is_dir() { return Err("existing absolute run directory required"); }
    Ok(path)
}

fn main() {
    let run_dir = parse_args(std::env::args().skip(1)).expect("test-only guard/arguments rejected");
    let events = OpenOptions::new().write(true).create_new(true).open(run_dir.join("p03-events.ndjson")).expect("new p03-events.ndjson required");
    tauri::Builder::default()
        .manage(P03State { events: Mutex::new(events), count: Mutex::new(0), run_dir })
        .invoke_handler(tauri::generate_handler![p03_record, p03_save_summary])
        .setup(|app| {
            tauri::WebviewWindowBuilder::new(app, "p03-experiment", tauri::WebviewUrl::App("p03-experiment.html".into()))
                .title("P03 TEST ONLY — Native Projection Experiment").inner_size(1100.0, 760.0)
                .on_navigation(|url| url.host_str() == Some("tauri.localhost") || url.scheme() == "tauri")
                .build()?;
            Ok(())
        })
        .run(tauri::generate_context!("examples/p03-experiment/tauri.conf.json"))
        .expect("P03 experiment runtime failed");
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_guard_and_absolute_existing_directory_are_required() {
        assert!(parse_args(["wrong".into()].into_iter()).is_err());
        assert!(parse_args([P03_EXPERIMENT_ONLY.into(), "relative".into()].into_iter()).is_err());
        assert!(parse_args([P03_EXPERIMENT_ONLY.into(), std::env::temp_dir().to_string_lossy().into_owned()].into_iter()).is_ok());
    }
}
