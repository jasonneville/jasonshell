mod apps;
mod files;
mod index;
mod scoring;
mod windows_search;

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemSearchResult {
    pub id: String,
    pub kind: String,
    pub title: String,
    pub subtitle: String,
    pub terms: String,
    pub priority: i32,
    pub path: String,
}

impl SystemSearchResult {
    fn new(kind: &str, title: String, subtitle: String, path: PathBuf, priority: i32) -> Self {
        let path_text = path.display().to_string();
        Self {
            id: format!("system:{kind}:{path_text}"),
            kind: kind.to_string(),
            title,
            subtitle,
            terms: format!("{path_text} {kind} local filesystem installed program"),
            priority,
            path: path_text,
        }
    }
}

pub type SearchIndexRuntimeState = index::SearchIndexRuntimeState;

pub fn warm_search_index(app_handle: tauri::AppHandle) {
    index::warm_search_index(app_handle);
}

#[tauri::command]
pub fn search_system(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, Mutex<SearchIndexRuntimeState>>,
    query: String,
) -> Result<Vec<SystemSearchResult>, String> {
    let query = query.trim();
    if query.len() < 2 {
        return Ok(Vec::new());
    }

    index::search_index(&app_handle, &state, query)
}

pub fn search_index_state() -> Mutex<SearchIndexRuntimeState> {
    Mutex::new(SearchIndexRuntimeState::default())
}
