mod apps;
mod everything;
mod everything_ffi;
mod everything_install;
mod files;
mod index;
mod provider;
mod query;
mod scoring;
mod windows_search;

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::Manager;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemSearchResult {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_id: Option<String>,
    pub kind: String,
    pub title: String,
    pub subtitle: String,
    pub terms: String,
    pub priority: i32,
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub record_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_count: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub top_most: Option<bool>,
}

impl SystemSearchResult {
    fn new(kind: &str, title: String, subtitle: String, path: PathBuf, priority: i32) -> Self {
        let path_text = path.display().to_string();
        Self {
            id: format!("system:{kind}:{path_text}"),
            provider_id: Some(provider_id_for_kind(kind).to_string()),
            kind: kind.to_string(),
            title,
            subtitle,
            terms: format!("{path_text} {kind} local filesystem installed program"),
            priority,
            path: path_text.clone(),
            record_key: Some(record_key(kind, &path_text)),
            run_count: None,
            top_most: None,
        }
    }
}

fn provider_id_for_kind(kind: &str) -> &'static str {
    if kind == "app" {
        "apps"
    } else {
        "warmedCache"
    }
}

fn record_key(kind: &str, path: &str) -> String {
    format!("{}:{}", kind, path.trim().replace('/', r"\").to_lowercase())
}

pub type SearchIndexRuntimeState = index::SearchIndexRuntimeState;
pub type ProviderHealthContract = provider::ProviderHealthContract;
pub type EverythingSetupConsentRequest = everything_install::EverythingSetupConsentRequest;
pub type EverythingSetupResult = everything_install::EverythingSetupResult;

pub fn warm_search_index(app_handle: tauri::AppHandle) {
    index::warm_search_index(app_handle);
}

#[tauri::command]
pub async fn search_system(
    app_handle: tauri::AppHandle,
    query: String,
) -> Result<Vec<SystemSearchResult>, String> {
    let query = query.trim().to_string();
    if query.is_empty() {
        return Ok(Vec::new());
    }

    tauri::async_runtime::spawn_blocking(move || {
        let state = app_handle.state::<Mutex<SearchIndexRuntimeState>>();
        index::search_index(&app_handle, &state, &query)
    })
    .await
    .map_err(|error| format!("Search worker failed: {error}"))?
}

#[tauri::command]
pub fn get_search_provider_health(
    app_handle: tauri::AppHandle,
) -> Result<Vec<ProviderHealthContract>, String> {
    let settings = crate::settings::load_shell_settings_for_app(&app_handle)
        .unwrap_or_else(|_| crate::settings::ShellSettings::default());
    Ok(provider::current_provider_health(&settings))
}

#[tauri::command]
pub fn request_everything_setup(
    request: EverythingSetupConsentRequest,
) -> Result<EverythingSetupResult, String> {
    Ok(everything_install::request_everything_setup(request))
}

pub fn search_index_state() -> Mutex<SearchIndexRuntimeState> {
    Mutex::new(SearchIndexRuntimeState::default())
}
