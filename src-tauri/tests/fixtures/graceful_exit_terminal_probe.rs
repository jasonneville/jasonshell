//! Read-only test affordance: duplicate exact production-owned ConPTY child
//! handles BEFORE drain, so exit assertions do not trust a recycled PID.
use super::*;
use std::os::windows::io::{BorrowedHandle, OwnedHandle};

pub(crate) fn process_handles(app: &AppHandle) -> Result<Vec<(String, OwnedHandle)>, String> {
    let state = app.state::<Mutex<StackPopupRuntimeState>>();
    let runtime = state.lock().map_err(|_| "terminal probe ownership poisoned")?;
    runtime.terminal_sessions.sessions.values().map(|session| {
        let raw = session.child.as_raw_handle().ok_or("ConPTY child process handle unavailable")?;
        // The registry lock retains the actual owning Child throughout duplication.
        let handle = unsafe { BorrowedHandle::borrow_raw(raw) }.try_clone_to_owned()
            .map_err(|_| "could not duplicate exact terminal process handle")?;
        Ok((session.target_label.clone(), handle))
    }).collect()
}

pub(crate) fn retained_counts(app: &AppHandle) -> Result<(usize, usize), String> {
    let state = app.state::<Mutex<StackPopupRuntimeState>>();
    let runtime = state.lock().map_err(|_| "terminal probe ownership poisoned")?;
    Ok((runtime.terminal_sessions.sessions.len(), runtime.terminal_sessions.draining.len()))
}
