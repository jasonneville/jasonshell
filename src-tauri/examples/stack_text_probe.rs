//! Standalone packaged measurement host. No shell hooks, AppBars, user files or clipboard.
#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]
use serde::{Deserialize, Serialize};
use std::{fs::OpenOptions, io::Write, sync::Mutex};

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Event {
    kind: Kind,
    time_ms: f64,
    metadata: serde_json::Value,
}
#[derive(Debug, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
enum Kind {
    Intent,
    WorkerReady,
    FirstPaintCallback,
    LocalMutation,
    InputPaintCallback,
    Ack,
    ControlStall,
}
struct Probe {
    output: Mutex<std::fs::File>,
    mode: String,
}
fn authorize(window: &tauri::WebviewWindow) -> Result<(), String> {
    if window.label() != "p01-probe" {
        return Err("Unauthorized".into());
    }
    Ok(())
}
#[tauri::command]
fn probe_mode(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, Probe>,
) -> Result<String, String> {
    authorize(&window)?;
    Ok(state.mode.clone())
}
#[tauri::command]
fn probe_prefix(window: tauri::WebviewWindow) -> Result<String, String> {
    authorize(&window)?;
    Ok("Seeded P01 prefix. Original UTF-8 bytes; EOF withheld.\n".repeat(3)[..128].into())
}
#[tauri::command]
async fn probe_ack(window: tauri::WebviewWindow) -> Result<(), String> {
    authorize(&window)?;
    tauri::async_runtime::spawn_blocking(|| {
        std::thread::sleep(std::time::Duration::from_millis(8000))
    })
    .await
    .map_err(|_| "IoFailure".to_string())?;
    Ok(())
}
#[tauri::command]
fn probe_report(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, Probe>,
    events: Vec<Event>,
) -> Result<(), String> {
    authorize(&window)?;
    if events.len() > 32 {
        return Err("ResourceLimit".into());
    }
    validate_events(&events, &state.mode)?;
    let bytes = serde_json::to_vec(&events).map_err(|_| "InvalidMeasurement")?;
    let mut output = state.output.lock().map_err(|_| "IoFailure")?;
    output
        .write_all(&bytes)
        .and_then(|_| output.write_all(b"\n"))
        .and_then(|_| output.sync_all())
        .map_err(|_| "IoFailure".to_string())
}
fn validate_events(events: &[Event], mode: &str) -> Result<(), String> {
    let expected = match mode {
        "editor" => vec![
            Kind::Intent,
            Kind::WorkerReady,
            Kind::FirstPaintCallback,
            Kind::LocalMutation,
            Kind::InputPaintCallback,
            Kind::Ack,
        ],
        "control" => vec![Kind::Intent, Kind::ControlStall],
        _ => return Err("InvalidMeasurement".into()),
    };
    if events.len() != expected.len()
        || events
            .iter()
            .zip(expected)
            .any(|(event, kind)| event.kind != kind)
    {
        return Err("InvalidMeasurement".into());
    }
    let mut previous = 0.0;
    for event in events {
        let metadata = match event.kind {
            Kind::FirstPaintCallback => serde_json::json!({"bytesRead": 128}),
            Kind::Ack => serde_json::json!({"revision": "1"}),
            _ => serde_json::json!({}),
        };
        if !event.time_ms.is_finite() || event.time_ms < previous || event.metadata != metadata {
            return Err("InvalidMeasurement".into());
        }
        previous = event.time_ms;
    }
    Ok(())
}
fn main() {
    let mut args = std::env::args().skip(1);
    let output = args
        .next()
        .expect("usage: stack_text_probe OUTPUT [editor|control]");
    let mode = args.next().unwrap_or_else(|| "editor".into());
    assert!(["editor", "control"].contains(&mode.as_str()));
    let output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)
        .expect("new output file required");
    tauri::Builder::default()
        .manage(Probe {
            output: Mutex::new(output),
            mode,
        })
        .invoke_handler(tauri::generate_handler![
            probe_mode,
            probe_prefix,
            probe_ack,
            probe_report
        ])
        .setup(|app| {
            tauri::WebviewWindowBuilder::new(
                app,
                "p01-probe",
                tauri::WebviewUrl::App("index.html".into()),
            )
            .title("P01 Measurement Probe")
            .inner_size(800.0, 500.0)
            .on_navigation(|url| {
                url.host_str() == Some("tauri.localhost") || url.scheme() == "tauri"
            })
            .build()?;
            Ok(())
        })
        .run(tauri::generate_context!(
            "examples/text-probe/tauri.conf.json"
        ))
        .expect("probe runtime failed");
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn measurement_metadata_and_sequence_are_exact() {
        let mut events: Vec<Event> = serde_json::from_value(serde_json::json!([
            {"kind":"intent","timeMs":0,"metadata":{}},
            {"kind":"workerReady","timeMs":1,"metadata":{}},
            {"kind":"firstPaintCallback","timeMs":2,"metadata":{"bytesRead":128}},
            {"kind":"localMutation","timeMs":3,"metadata":{}},
            {"kind":"inputPaintCallback","timeMs":4,"metadata":{}},
            {"kind":"ack","timeMs":1504,"metadata":{"revision":"1"}}
        ]))
        .unwrap();
        assert!(validate_events(&events, "editor").is_ok());
        assert!(validate_events(&events, "control").is_err());
        for wrong in [
            serde_json::json!({}),
            serde_json::json!({"revision":null}),
            serde_json::json!({"bytesRead":128}),
            serde_json::json!({"revision":"1", "bytesRead":128}),
        ] {
            events[5].metadata = wrong;
            assert!(validate_events(&events, "editor").is_err());
        }
        events[5].metadata = serde_json::json!({"revision":"1"});
        events[5].time_ms = 0.0;
        assert!(validate_events(&events, "editor").is_err());
    }
}
