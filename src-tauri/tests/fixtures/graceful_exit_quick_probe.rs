//! Read-only observation of the production job/worker retention ledger.
use super::*;
pub(crate) fn retained_runs() -> Result<usize, String> {
    Ok(owned_runs().lock().map_err(|_| "owned run probe poisoned")?.len())
}
