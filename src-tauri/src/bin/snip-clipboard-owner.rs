//! Bundled opt-in helper executable; never part of shell startup.
#[path = "../snipping/mod.rs"]
mod snipping;
#[cfg(windows)]
fn main() {
    if std::env::args().skip(1).collect::<Vec<_>>() != ["--internal-snip-clipboard-owner"] { std::process::exit(2); }
    if snipping::clipboard_process::helper_main().is_err() { std::process::exit(1); }
}
#[cfg(not(windows))]
fn main() { std::process::exit(2); }
