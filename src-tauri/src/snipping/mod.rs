//! Opt-in snipping primitives. Intentionally not registered in normal shell startup.
pub mod geometry;
pub mod session;
pub mod save;
#[cfg(windows)]
pub mod coordinator;
#[cfg(windows)]
pub mod capture;
#[cfg(windows)]
pub mod clipboard;
#[cfg(windows)]
pub mod clipboard_process;
#[cfg(windows)]
pub mod windows;
#[cfg(windows)]
pub mod native_registry;
#[cfg(windows)]
pub mod native;
#[cfg(windows)]
pub mod foreground;
#[cfg(windows)]
pub mod composition;
#[cfg(windows)]
pub mod runtime;
