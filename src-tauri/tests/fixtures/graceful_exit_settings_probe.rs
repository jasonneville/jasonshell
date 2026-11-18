//! Isolated fixture settings destination. Production settings_path consults this
//! test-only managed override; no installed or shared user settings are touched.
use std::path::PathBuf;
use tauri::{AppHandle, Manager};
pub(crate) struct FixtureSettingsPath(pub PathBuf);
pub(crate) fn override_path(app: &AppHandle) -> Option<PathBuf> {
    app.try_state::<FixtureSettingsPath>().map(|path|path.0.clone())
}
