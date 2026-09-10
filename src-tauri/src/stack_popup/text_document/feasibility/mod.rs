//! Developer-only P02 experiments. Not a production editor command/provider.
//! All content-bearing storage is bounded and encrypted; no source text is logged.

pub(crate) mod backing;
pub(crate) mod contract;
pub(crate) mod decode;
pub(crate) mod hash;
pub(crate) mod index;
pub(crate) mod lease;
#[cfg(windows)]
pub(crate) mod native;
pub(crate) mod scheduler;
#[cfg(windows)]
pub(crate) mod session;
#[cfg(windows)]
pub(crate) mod source;
#[cfg(all(windows, debug_assertions))]
pub(crate) use session::install_native_probe;
#[cfg(all(test, windows))]
mod tests;

use self::contract::{ProtocolError, ProtocolErrorCode};
pub(crate) type Result<T, E = ProtocolError> = std::result::Result<T, E>;
pub(crate) fn failure(code: ProtocolErrorCode, message: impl Into<String>) -> ProtocolError {
    ProtocolError {
        code,
        message: message.into(),
        retryable: false,
        field: None,
        operation_id: None,
        job_id: None,
        expected_revision: None,
        actual_revision: None,
    }
}

pub(crate) fn io_failure(error: std::io::Error) -> ProtocolError {
    let code = match error.raw_os_error() {
        Some(5) => ProtocolErrorCode::Readonly,
        Some(32 | 33) => ProtocolErrorCode::SharingViolation,
        Some(112 | 39) => ProtocolErrorCode::ResourceLimit,
        Some(995) => ProtocolErrorCode::Cancelled,
        _ => ProtocolErrorCode::IoFailure,
    };
    failure(
        code,
        format!("storage I/O failed (os={:?})", error.raw_os_error()),
    )
}

pub(crate) fn checked_add(a: u64, b: u64) -> Result<u64> {
    a.checked_add(b)
        .ok_or_else(|| failure(ProtocolErrorCode::ResourceLimit, "byte counter overflow"))
}
