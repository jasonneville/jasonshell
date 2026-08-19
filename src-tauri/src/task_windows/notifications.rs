use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};
use windows::UI::Notifications::Management::{
    UserNotificationListener, UserNotificationListenerAccessStatus,
};
use windows::UI::Notifications::NotificationKinds;

const NOTIFICATION_POLL_INTERVAL: Duration = Duration::from_secs(1);
const MAX_RETAINED_NOTIFICATION_IDS: usize = 4_096;
const APP_INSTALL_PATH_TTL: Duration = Duration::from_secs(60);
const APP_INSTALL_PATH_NEGATIVE_TTL: Duration = Duration::from_secs(10);
const MAX_APP_INSTALL_PATH_CACHE_ENTRIES: usize = 256;

#[derive(Default)]
struct NotificationState {
    // Retain delivered toast IDs across Action Center dismissal so the count is
    // notifications received since this app was last focused, not current history.
    counted_notification_ids: HashSet<(String, u32)>,
    counted_notification_order: VecDeque<(String, u32)>,
    counts_by_app_id: HashMap<String, u32>,
    install_path_cache: HashMap<String, (Option<PathBuf>, Instant)>,
    install_path_cache_order: VecDeque<String>,
}

static NOTIFICATION_STATE: OnceLock<Mutex<NotificationState>> = OnceLock::new();

fn state() -> &'static Mutex<NotificationState> {
    NOTIFICATION_STATE.get_or_init(|| Mutex::new(NotificationState::default()))
}

pub(super) fn start_notification_tracking() {
    super::diagnostics::initialize_package_identity_diagnostics();
    super::diagnostics::note_listener_start();
    thread::spawn(|| {
        let Ok(listener) = UserNotificationListener::Current() else {
            super::diagnostics::note_listener_status(super::ToastListenerStatus::Unavailable);
            return;
        };
        let Ok(access) = listener
            .RequestAccessAsync()
            .and_then(|operation| operation.get())
        else {
            super::diagnostics::note_listener_status(super::ToastListenerStatus::Error);
            return;
        };
        if access == UserNotificationListenerAccessStatus::Denied {
            super::diagnostics::note_denied_request();
            return;
        }
        if access != UserNotificationListenerAccessStatus::Allowed {
            super::diagnostics::note_listener_status(super::ToastListenerStatus::Unavailable);
            return;
        }
        super::diagnostics::note_listener_status(super::ToastListenerStatus::Allowed);

        loop {
            let Ok(notifications) = listener
                .GetNotificationsAsync(NotificationKinds::Toast)
                .and_then(|operation| operation.get())
            else {
                super::diagnostics::note_listener_poll_failure("GetNotificationsAsync failed");
                thread::sleep(NOTIFICATION_POLL_INTERVAL);
                continue;
            };
            super::diagnostics::note_listener_poll_success();

            for notification in notifications {
                let (Ok(id), Ok(app_info)) = (notification.Id(), notification.AppInfo()) else {
                    continue;
                };
                let Ok(app_id) = app_info.AppUserModelId() else {
                    super::diagnostics::note_unresolved_app_id("<missing-app-id>");
                    continue;
                };
                super::diagnostics::note_resolved_app_id(&app_id.to_string_lossy());
                record_notification(&app_id.to_string_lossy(), id);
            }
            thread::sleep(NOTIFICATION_POLL_INTERVAL);
        }
    });
}

pub(super) fn notification_count_for_process_path(
    process_path: Option<&Path>,
    _app: Option<&tauri::AppHandle>,
) -> u32 {
    let Some(process_path) = process_path else {
        return 0;
    };
    let app_ids = match state().lock() {
        Ok(state) => state.counts_by_app_id.clone(),
        Err(_) => return 0,
    };
    app_ids
        .into_iter()
        .filter_map(|(app_id, count)| app_install_path_cached(&app_id).map(|path| (path, count)))
        .filter(|(install_path, _)| process_path.starts_with(install_path))
        .map(|(_, count)| count)
        .max()
        .unwrap_or(0)
}

pub(super) fn clear_notifications_for_process_path(process_path: &Path) {
    let app_ids = match state().lock() {
        Ok(state) => state.counts_by_app_id.keys().cloned().collect::<Vec<_>>(),
        Err(_) => return,
    };
    let matching_ids = app_ids
        .into_iter()
        .filter(|app_id| {
            app_install_path_cached(app_id).is_some_and(|path| process_path.starts_with(path))
        })
        .collect::<Vec<_>>();
    if let Ok(mut state) = state().lock() {
        for app_id in matching_ids {
            state.counts_by_app_id.remove(&app_id);
        }
    }
}

fn app_install_path_cached(app_id: &str) -> Option<PathBuf> {
    if let Ok(state) = state().lock() {
        if let Some((cached, at)) = state.install_path_cache.get(app_id) {
            let ttl = if cached.is_some() {
                APP_INSTALL_PATH_TTL
            } else {
                APP_INSTALL_PATH_NEGATIVE_TTL
            };
            if at.elapsed() < ttl {
                return cached.clone();
            }
        }
    }
    let app_id = windows::core::HSTRING::from(app_id);
    let resolved = windows::ApplicationModel::AppInfo::GetFromAppUserModelId(&app_id)
        .ok()
        .and_then(|app_info| app_info.Package().ok())
        .and_then(|package| package.InstalledPath().ok())
        .map(|path| PathBuf::from(path.to_string_lossy()));
    if let Ok(mut state) = state().lock() {
        let app_id = app_id.to_string_lossy().to_owned();
        state
            .install_path_cache
            .insert(app_id.clone(), (resolved.clone(), Instant::now()));
        state
            .install_path_cache_order
            .retain(|cached_id| cached_id != &app_id);
        state.install_path_cache_order.push_back(app_id);
        while state.install_path_cache_order.len() > MAX_APP_INSTALL_PATH_CACHE_ENTRIES {
            if let Some(expired_app_id) = state.install_path_cache_order.pop_front() {
                state.install_path_cache.remove(&expired_app_id);
            }
        }
    }
    if resolved.is_some() {
        super::diagnostics::note_resolved_app_id(&app_id.to_string_lossy());
    } else {
        super::diagnostics::note_unresolved_app_id(&app_id.to_string_lossy());
    }
    resolved
}

fn record_notification(app_id: &str, notification_id: u32) {
    let Ok(mut state) = state().lock() else {
        return;
    };
    let notification_key = (app_id.to_string(), notification_id);
    if !state
        .counted_notification_ids
        .insert(notification_key.clone())
    {
        return;
    }
    state.counted_notification_order.push_back(notification_key);
    if state.counted_notification_order.len() > MAX_RETAINED_NOTIFICATION_IDS {
        if let Some(expired_key) = state.counted_notification_order.pop_front() {
            state.counted_notification_ids.remove(&expired_key);
        }
    }
    let count = state
        .counts_by_app_id
        .entry(app_id.to_string())
        .or_default();
    *count = count.saturating_add(1);
}

#[cfg(test)]
pub(super) fn clear_all_notification_state() {
    if let Ok(mut state) = state().lock() {
        *state = NotificationState::default();
    }
}

#[cfg(test)]
pub(super) fn notification_count_for_app_id(app_id: &str) -> u32 {
    state()
        .lock()
        .ok()
        .and_then(|state| state.counts_by_app_id.get(app_id).copied())
        .unwrap_or(0)
}

#[cfg(test)]
pub(super) fn clear_notifications_for_app_id(app_id: &str) {
    if let Ok(mut state) = state().lock() {
        state.counts_by_app_id.remove(app_id);
    }
}

#[cfg(test)]
pub(super) fn record_notification_for_test(app_id: &str, notification_id: u32) {
    record_notification(app_id, notification_id);
}
