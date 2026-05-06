use crate::shell_windows::{
    SEARCH_PANEL_HEIGHT_LOGICAL, SEARCH_PANEL_LABEL, SEARCH_PANEL_WIDTH_LOGICAL, TOP_BAR_LABEL,
};
use serde::Deserialize;
use serde_json::Value;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize};

const SEARCH_PANEL_UPDATE_EVENT: &str = "search-panel:update";
pub const SEARCH_PANEL_INTERACTION_EVENT: &str = "search-panel:interaction";
pub const SEARCH_PANEL_CLOSED_EVENT: &str = "search-panel:closed";
const SEARCH_PANEL_MARGIN_PHYSICAL: i32 = 6;
const SEARCH_PANEL_EDGE_PADDING_PHYSICAL: i32 = 8;
const CENTERED_SEARCH_WIDTH_LOGICAL: f64 = 720.0;
const CENTERED_SEARCH_HEIGHT_LOGICAL: f64 = 560.0;
const CENTERED_SEARCH_MIN_WIDTH_LOGICAL: f64 = 420.0;
const CENTERED_SEARCH_MIN_HEIGHT_LOGICAL: f64 = 320.0;
const CENTERED_SEARCH_MAX_WIDTH_LOGICAL: f64 = 1_200.0;
const CENTERED_SEARCH_MAX_HEIGHT_LOGICAL: f64 = 900.0;

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShowSearchPanelRequest {
    pub anchor_left: f64,
    pub anchor_width: f64,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CenteredSearchPanelRequest {
    pub width: f64,
    pub height: f64,
}

#[derive(Default)]
pub struct SearchPanelRuntimeState {
    latest_payload: Option<Value>,
    latest_payload_sequence: u64,
    latest_payload_query: Option<String>,
    latest_payload_phase_rank: u8,
}

#[tauri::command]
pub fn show_search_panel(
    app_handle: AppHandle,
    request: ShowSearchPanelRequest,
) -> Result<(), String> {
    let panel = app_handle
        .get_webview_window(SEARCH_PANEL_LABEL)
        .ok_or_else(|| "Search panel window is unavailable".to_string())?;
    let top_bar = app_handle
        .get_webview_window(TOP_BAR_LABEL)
        .ok_or_else(|| "Top bar window is unavailable".to_string())?;

    let scale_factor = top_bar
        .scale_factor()
        .map_err(|error| format!("Failed to read the top bar scale factor: {error}"))?;
    let top_position = top_bar
        .outer_position()
        .map_err(|error| format!("Failed to read the top bar position: {error}"))?;
    let top_size = top_bar
        .outer_size()
        .map_err(|error| format!("Failed to read the top bar size: {error}"))?;
    let panel_width = (SEARCH_PANEL_WIDTH_LOGICAL * scale_factor).round() as i32;
    let panel_height = (SEARCH_PANEL_HEIGHT_LOGICAL * scale_factor).round() as u32;
    let panel_x = anchored_panel_x(
        top_position.x,
        top_size.width as i32,
        request.anchor_left,
        request.anchor_width,
        panel_width,
        scale_factor,
    );
    let panel_y = top_position.y + top_size.height as i32 + SEARCH_PANEL_MARGIN_PHYSICAL;

    panel
        .set_size(PhysicalSize::new(panel_width as u32, panel_height))
        .map_err(|error| format!("Failed to size the search panel: {error}"))?;
    panel
        .set_position(PhysicalPosition::new(panel_x, panel_y))
        .map_err(|error| format!("Failed to position the search panel: {error}"))?;
    panel
        .show()
        .map_err(|error| format!("Failed to show the search panel: {error}"))
}

#[tauri::command]
pub fn show_centered_search_panel(
    app_handle: AppHandle,
    request: CenteredSearchPanelRequest,
) -> Result<(), String> {
    let panel = app_handle
        .get_webview_window(SEARCH_PANEL_LABEL)
        .ok_or_else(|| "Search panel window is unavailable".to_string())?;
    let top_bar = app_handle
        .get_webview_window(TOP_BAR_LABEL)
        .ok_or_else(|| "Top bar window is unavailable".to_string())?;
    let monitor = top_bar
        .current_monitor()
        .map_err(|error| format!("Failed to read search monitor: {error}"))?
        .ok_or_else(|| "Search monitor is unavailable".to_string())?;
    let scale_factor = top_bar
        .scale_factor()
        .map_err(|error| format!("Failed to read the top bar scale factor: {error}"))?;
    let logical_size = bounded_centered_search_size(request);
    let width = (logical_size.width * scale_factor).round() as u32;
    let height = (logical_size.height * scale_factor).round() as u32;
    let monitor_position = monitor.position();
    let monitor_size = monitor.size();
    let x = monitor_position.x + ((monitor_size.width.saturating_sub(width)) / 2) as i32;
    let y = monitor_position.y + ((monitor_size.height.saturating_sub(height)) / 3) as i32;

    panel
        .set_size(PhysicalSize::new(width, height))
        .map_err(|error| format!("Failed to size centered search panel: {error}"))?;
    panel
        .set_position(PhysicalPosition::new(x, y))
        .map_err(|error| format!("Failed to position centered search panel: {error}"))?;
    panel
        .show()
        .map_err(|error| format!("Failed to show centered search panel: {error}"))?;
    panel
        .set_focus()
        .map_err(|error| format!("Failed to focus centered search panel: {error}"))
}

#[tauri::command]
pub fn resize_search_panel(
    app_handle: AppHandle,
    request: CenteredSearchPanelRequest,
) -> Result<(), String> {
    let panel = app_handle
        .get_webview_window(SEARCH_PANEL_LABEL)
        .ok_or_else(|| "Search panel window is unavailable".to_string())?;
    let scale_factor = panel
        .scale_factor()
        .map_err(|error| format!("Failed to read the search panel scale factor: {error}"))?;
    let logical_size = bounded_centered_search_size(request);
    panel
        .set_size(PhysicalSize::new(
            (logical_size.width * scale_factor).round() as u32,
            (logical_size.height * scale_factor).round() as u32,
        ))
        .map_err(|error| format!("Failed to resize search panel: {error}"))
}

#[tauri::command]
pub fn hide_search_panel(app_handle: AppHandle) -> Result<(), String> {
    let panel = app_handle
        .get_webview_window(SEARCH_PANEL_LABEL)
        .ok_or_else(|| "Search panel window is unavailable".to_string())?;
    panel
        .hide()
        .map_err(|error| format!("Failed to hide the search panel: {error}"))?;
    emit_search_panel_closed_to_top_bar(&app_handle)
}

pub fn emit_search_panel_closed_to_top_bar(app_handle: &AppHandle) -> Result<(), String> {
    app_handle
        .emit_to(TOP_BAR_LABEL, SEARCH_PANEL_CLOSED_EVENT, ())
        .map_err(|error| format!("Failed to publish search panel closed event: {error}"))
}

#[tauri::command]
pub fn publish_search_panel(
    app_handle: AppHandle,
    state: tauri::State<'_, Mutex<SearchPanelRuntimeState>>,
    payload: Value,
) -> Result<(), String> {
    if !store_search_panel_payload(&state, payload.clone()) {
        return Ok(());
    }
    let _panel = app_handle
        .get_webview_window(SEARCH_PANEL_LABEL)
        .ok_or_else(|| "Search panel window is unavailable".to_string())?;
    app_handle
        .emit_to(SEARCH_PANEL_LABEL, SEARCH_PANEL_UPDATE_EVENT, payload)
        .map_err(|error| format!("Failed to publish search panel results: {error}"))
}

#[tauri::command]
pub fn get_search_panel_payload(
    state: tauri::State<'_, Mutex<SearchPanelRuntimeState>>,
) -> Result<Option<Value>, String> {
    Ok(latest_search_panel_payload(&state))
}

fn store_search_panel_payload(state: &Mutex<SearchPanelRuntimeState>, payload: Value) -> bool {
    let mut guard = state
        .lock()
        .expect("search panel runtime state is poisoned");
    if let Some(sequence) = search_panel_payload_sequence(&payload) {
        let query = search_panel_payload_query(&payload);
        let phase_rank = search_panel_payload_phase_rank(&payload);
        if sequence < guard.latest_payload_sequence {
            return false;
        }
        if sequence == guard.latest_payload_sequence {
            if query.as_deref() != guard.latest_payload_query.as_deref() {
                return false;
            }
            if phase_rank < guard.latest_payload_phase_rank {
                return false;
            }
        }
        guard.latest_payload_sequence = sequence;
        guard.latest_payload_query = query;
        guard.latest_payload_phase_rank = phase_rank;
    }
    guard.latest_payload = Some(payload);
    true
}

fn latest_search_panel_payload(state: &Mutex<SearchPanelRuntimeState>) -> Option<Value> {
    state
        .lock()
        .expect("search panel runtime state is poisoned")
        .latest_payload
        .clone()
}

fn search_panel_payload_sequence(payload: &Value) -> Option<u64> {
    payload.get("sequence").and_then(Value::as_u64)
}

fn search_panel_payload_query(payload: &Value) -> Option<String> {
    payload
        .get("query")
        .and_then(Value::as_str)
        .map(|query| query.trim().to_string())
}

fn search_panel_payload_phase_rank(payload: &Value) -> u8 {
    match payload.get("phase").and_then(Value::as_str) {
        Some("typing") => 0,
        Some("local") => 1,
        Some("provider") => 2,
        Some("error") => 2,
        Some("complete") => 3,
        _ => 3,
    }
}

fn bounded_centered_search_size(request: CenteredSearchPanelRequest) -> CenteredSearchPanelRequest {
    CenteredSearchPanelRequest {
        width: bounded_centered_dimension(
            request.width,
            CENTERED_SEARCH_MIN_WIDTH_LOGICAL,
            CENTERED_SEARCH_MAX_WIDTH_LOGICAL,
            CENTERED_SEARCH_WIDTH_LOGICAL,
        ),
        height: bounded_centered_dimension(
            request.height,
            CENTERED_SEARCH_MIN_HEIGHT_LOGICAL,
            CENTERED_SEARCH_MAX_HEIGHT_LOGICAL,
            CENTERED_SEARCH_HEIGHT_LOGICAL,
        ),
    }
}

fn bounded_centered_dimension(value: f64, min: f64, max: f64, default: f64) -> f64 {
    if value.is_finite() {
        value.clamp(min, max)
    } else {
        default
    }
}

fn anchored_panel_x(
    host_x: i32,
    host_width: i32,
    anchor_left: f64,
    anchor_width: f64,
    panel_width: i32,
    scale_factor: f64,
) -> i32 {
    let anchor_right = host_x + ((anchor_left + anchor_width) * scale_factor).round() as i32;
    let min_x = host_x + SEARCH_PANEL_EDGE_PADDING_PHYSICAL;
    let max_x = host_x + host_width - panel_width - SEARCH_PANEL_EDGE_PADDING_PHYSICAL;
    (anchor_right - panel_width).clamp(min_x, max_x.max(min_x))
}

#[cfg(test)]
mod tests {
    use super::{
        anchored_panel_x, bounded_centered_search_size, latest_search_panel_payload,
        store_search_panel_payload, CenteredSearchPanelRequest, SearchPanelRuntimeState,
        CENTERED_SEARCH_HEIGHT_LOGICAL, CENTERED_SEARCH_WIDTH_LOGICAL,
    };
    use crate::shell_windows::{SEARCH_PANEL_HEIGHT_LOGICAL, SEARCH_PANEL_WIDTH_LOGICAL};
    use serde_json::json;
    use std::sync::Mutex;

    #[test]
    fn anchors_panel_to_search_control_right_edge() {
        assert_eq!(anchored_panel_x(0, 1_920, 1_600.0, 260.0, 420, 1.0), 1_440);
    }

    #[test]
    fn clamps_panel_inside_host_edges() {
        assert_eq!(anchored_panel_x(0, 500, 20.0, 100.0, 420, 1.0), 8);
        assert_eq!(anchored_panel_x(0, 500, 470.0, 40.0, 420, 1.0), 72);
    }

    #[test]
    fn centered_search_uses_larger_keyboard_launcher_size() {
        assert_eq!(SEARCH_PANEL_WIDTH_LOGICAL, 420.0);
        assert_eq!(SEARCH_PANEL_HEIGHT_LOGICAL, 320.0);
        assert_eq!(CENTERED_SEARCH_WIDTH_LOGICAL, 720.0);
        assert_eq!(CENTERED_SEARCH_HEIGHT_LOGICAL, 560.0);
    }

    #[test]
    fn centered_search_resize_clamps_to_supported_bounds() {
        assert_eq!(
            bounded_centered_search_size(CenteredSearchPanelRequest {
                width: 12.0,
                height: 9_999.0,
            })
            .width,
            420.0
        );
        assert_eq!(
            bounded_centered_search_size(CenteredSearchPanelRequest {
                width: 12.0,
                height: 9_999.0,
            })
            .height,
            900.0
        );
        assert_eq!(
            bounded_centered_search_size(CenteredSearchPanelRequest {
                width: f64::NAN,
                height: f64::INFINITY,
            }),
            CenteredSearchPanelRequest {
                width: CENTERED_SEARCH_WIDTH_LOGICAL,
                height: CENTERED_SEARCH_HEIGHT_LOGICAL,
            }
        );
    }

    #[test]
    fn stores_latest_payload_with_visible_results_for_panel_fetch() {
        let state = Mutex::new(SearchPanelRuntimeState::default());
        let payload = json!({
            "query": "firefox",
            "results": [{
                "id": "app:C:\\Pins\\Firefox.lnk",
                "kind": "app",
                "title": "Firefox",
                "subtitle": "Pinned app",
                "terms": "firefox browser",
                "priority": 100
            }],
            "selectedIndex": 0,
            "statusMessage": "Type to search apps, windows, folders, and commands"
        });

        assert!(store_search_panel_payload(&state, payload.clone()));

        let stored = latest_search_panel_payload(&state).expect("payload should be stored");
        assert_eq!(stored["query"], "firefox");
        assert_eq!(stored["selectedIndex"], 0);
        assert_eq!(stored["results"][0]["title"], "Firefox");
    }

    #[test]
    fn rejects_stale_search_payload_sequences() {
        let state = Mutex::new(SearchPanelRuntimeState::default());
        let latest = json!({
            "query": "spotify",
            "results": [],
            "selectedIndex": 0,
            "statusMessage": "Searching",
            "phase": "local",
            "sequence": 4
        });
        let stale = json!({
            "query": "spot",
            "results": [],
            "selectedIndex": 0,
            "statusMessage": "Searching",
            "phase": "provider",
            "sequence": 3
        });

        assert!(store_search_panel_payload(&state, latest));
        assert!(!store_search_panel_payload(&state, stale));

        let stored = latest_search_panel_payload(&state).expect("payload should be stored");
        assert_eq!(stored["query"], "spotify");
        assert_eq!(stored["sequence"], 4);
    }

    #[test]
    fn rejects_same_sequence_for_different_query() {
        let state = Mutex::new(SearchPanelRuntimeState::default());
        assert!(store_search_panel_payload(
            &state,
            json!({
                "query": "spotify",
                "results": [],
                "selectedIndex": 0,
                "statusMessage": "Showing local results",
                "phase": "local",
                "sequence": 7
            })
        ));
        assert!(!store_search_panel_payload(
            &state,
            json!({
                "query": "spot",
                "results": [],
                "selectedIndex": 0,
                "statusMessage": "Showing provider results",
                "phase": "provider",
                "sequence": 7
            })
        ));
    }

    #[test]
    fn accepts_same_sequence_when_only_trailing_space_differs() {
        let state = Mutex::new(SearchPanelRuntimeState::default());
        assert!(store_search_panel_payload(
            &state,
            json!({
                "query": "spotify ",
                "results": [],
                "selectedIndex": 0,
                "statusMessage": "Searching",
                "phase": "typing",
                "sequence": 9
            })
        ));
        assert!(store_search_panel_payload(
            &state,
            json!({
                "query": "spotify",
                "results": [{
                    "id": "app:spotify",
                    "kind": "app",
                    "title": "Spotify",
                    "subtitle": "Installed app",
                    "terms": "spotify",
                    "priority": 100
                }],
                "selectedIndex": 0,
                "statusMessage": "Showing search results",
                "phase": "complete",
                "sequence": 9
            })
        ));

        let stored = latest_search_panel_payload(&state).expect("payload should be stored");
        assert_eq!(stored["query"], "spotify");
        assert_eq!(stored["results"][0]["title"], "Spotify");
    }

    #[test]
    fn rejects_phase_regression_for_same_query_and_sequence() {
        let state = Mutex::new(SearchPanelRuntimeState::default());
        assert!(store_search_panel_payload(
            &state,
            json!({
                "query": "spotify",
                "results": [],
                "selectedIndex": 0,
                "statusMessage": "Showing provider results",
                "phase": "provider",
                "sequence": 8
            })
        ));
        assert!(!store_search_panel_payload(
            &state,
            json!({
                "query": "spotify",
                "results": [],
                "selectedIndex": 0,
                "statusMessage": "Showing local results",
                "phase": "local",
                "sequence": 8
            })
        ));
    }

    #[test]
    fn allows_complete_after_recoverable_provider_error() {
        let state = Mutex::new(SearchPanelRuntimeState::default());
        assert!(store_search_panel_payload(
            &state,
            json!({
                "query": "spotify",
                "results": [],
                "selectedIndex": 0,
                "statusMessage": "Everything unavailable",
                "phase": "error",
                "sequence": 9
            })
        ));
        assert!(store_search_panel_payload(
            &state,
            json!({
                "query": "spotify",
                "results": [],
                "selectedIndex": 0,
                "statusMessage": "Showing search results",
                "phase": "complete",
                "sequence": 9
            })
        ));

        let stored = latest_search_panel_payload(&state).expect("payload should be stored");
        assert_eq!(stored["phase"], "complete");
        assert_eq!(stored["statusMessage"], "Showing search results");
    }
}
