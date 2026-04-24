use crate::shell_windows::{SEARCH_PANEL_LABEL, SEARCH_PANEL_WIDTH_LOGICAL, TOP_BAR_LABEL};
use serde::Deserialize;
use serde_json::Value;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition};

const SEARCH_PANEL_UPDATE_EVENT: &str = "search-panel:update";
const SEARCH_PANEL_MARGIN_PHYSICAL: i32 = 6;
const SEARCH_PANEL_EDGE_PADDING_PHYSICAL: i32 = 8;

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShowSearchPanelRequest {
    pub anchor_left: f64,
    pub anchor_width: f64,
}

#[derive(Default)]
pub struct SearchPanelRuntimeState {
    latest_payload: Option<Value>,
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
        .set_position(PhysicalPosition::new(panel_x, panel_y))
        .map_err(|error| format!("Failed to position the search panel: {error}"))?;
    panel
        .show()
        .map_err(|error| format!("Failed to show the search panel: {error}"))
}

#[tauri::command]
pub fn hide_search_panel(app_handle: AppHandle) -> Result<(), String> {
    let panel = app_handle
        .get_webview_window(SEARCH_PANEL_LABEL)
        .ok_or_else(|| "Search panel window is unavailable".to_string())?;
    panel
        .hide()
        .map_err(|error| format!("Failed to hide the search panel: {error}"))
}

#[tauri::command]
pub fn publish_search_panel(
    app_handle: AppHandle,
    state: tauri::State<'_, Mutex<SearchPanelRuntimeState>>,
    payload: Value,
) -> Result<(), String> {
    store_search_panel_payload(&state, payload.clone());
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

fn store_search_panel_payload(state: &Mutex<SearchPanelRuntimeState>, payload: Value) {
    state
        .lock()
        .expect("search panel runtime state is poisoned")
        .latest_payload = Some(payload);
}

fn latest_search_panel_payload(state: &Mutex<SearchPanelRuntimeState>) -> Option<Value> {
    state
        .lock()
        .expect("search panel runtime state is poisoned")
        .latest_payload
        .clone()
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
        anchored_panel_x, latest_search_panel_payload, store_search_panel_payload,
        SearchPanelRuntimeState,
    };
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

        store_search_panel_payload(&state, payload.clone());

        let stored = latest_search_panel_payload(&state).expect("payload should be stored");
        assert_eq!(stored["query"], "firefox");
        assert_eq!(stored["selectedIndex"], 0);
        assert_eq!(stored["results"][0]["title"], "Firefox");
    }
}
