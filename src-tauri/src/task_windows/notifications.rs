use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::Duration;
use windows::UI::Notifications::Management::{
    UserNotificationListener, UserNotificationListenerAccessStatus,
};
use windows::UI::Notifications::NotificationKinds;

const NOTIFICATION_POLL_INTERVAL: Duration = Duration::from_secs(1);
const MAX_RETAINED_NOTIFICATION_IDS: usize = 4_096;

#[derive(Default)]
struct NotificationState {
    // Retain delivered toast IDs across Action Center dismissal so the count is
    // notifications received since this app was last focused, not current history.
    counted_notification_ids: HashSet<(String, u32)>,
    counted_notification_order: VecDeque<(String, u32)>,
    counts_by_app_id: HashMap<String, u32>,
}

static NOTIFICATION_STATE: OnceLock<Mutex<NotificationState>> = OnceLock::new();

fn state() -> &'static Mutex<NotificationState> {
    NOTIFICATION_STATE.get_or_init(|| Mutex::new(NotificationState::default()))
}

pub(super) fn start_notification_tracking() {
    thread::spawn(|| {
        let Ok(listener) = UserNotificationListener::Current() else {
            return;
        };
        let Ok(access) = listener
            .RequestAccessAsync()
            .and_then(|operation| operation.get())
        else {
            return;
        };
        if access != UserNotificationListenerAccessStatus::Allowed {
            return;
        }

        loop {
            let Ok(notifications) = listener
                .GetNotificationsAsync(NotificationKinds::Toast)
                .and_then(|operation| operation.get())
            else {
                thread::sleep(NOTIFICATION_POLL_INTERVAL);
                continue;
            };

            for notification in notifications {
                let (Ok(id), Ok(app_info)) = (notification.Id(), notification.AppInfo()) else {
                    continue;
                };
                let Ok(app_id) = app_info.AppUserModelId() else {
                    continue;
                };
                record_notification(&app_id.to_string_lossy(), id);
            }
            thread::sleep(NOTIFICATION_POLL_INTERVAL);
        }
    });
}

pub(super) fn notification_count_for_process_path(process_path: Option<&Path>) -> u32 {
    let Some(process_path) = process_path else {
        return 0;
    };
    let app_ids = match state().lock() {
        Ok(state) => state.counts_by_app_id.clone(),
        Err(_) => return 0,
    };
    app_ids
        .into_iter()
        .filter_map(|(app_id, count)| app_install_path(&app_id).map(|path| (path, count)))
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
            app_install_path(app_id).is_some_and(|path| process_path.starts_with(path))
        })
        .collect::<Vec<_>>();
    if let Ok(mut state) = state().lock() {
        for app_id in matching_ids {
            state.counts_by_app_id.remove(&app_id);
        }
    }
}

fn app_install_path(app_id: &str) -> Option<PathBuf> {
    let app_id = windows::core::HSTRING::from(app_id);
    windows::ApplicationModel::AppInfo::GetFromAppUserModelId(&app_id)
        .ok()?
        .Package()
        .ok()?
        .InstalledPath()
        .ok()
        .map(|path| PathBuf::from(path.to_string_lossy()))
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
