//! Standalone test-owned helper. Never linked into the application.
use std::io::{self, Write};
fn main() {
    // Self-expire even if a broken parent stops reading a full pipe.
    std::thread::spawn(|| { std::thread::sleep(std::time::Duration::from_secs(5)); std::process::exit(99); });
    match std::env::args().nth(1).as_deref() {
        Some("stall") => std::thread::sleep(std::time::Duration::from_secs(5)),
        Some("flood") => { let chunk = [b'x';8192]; for _ in 0..256 { if io::stdout().write_all(&chunk).is_err() { break; } } }
        Some("exact") => io::stdout().write_all(&[b'x';8192]).unwrap(),
        Some("one-over") => io::stdout().write_all(&[b'x';8193]).unwrap(),
        Some("empty") => {},
        Some("malformed") => print!("{{not-json"),
        Some("nonzero") => std::process::exit(17),
        Some("valid") => print!(r#"{{"root":"C:\\","kind":"fixed","filesystem":{{"state":"ready","value":"NTFS","reason":null,"source":"fixture","unit":null,"sampledAtMs":123}},"totalBytes":{{"state":"ready","value":1000,"reason":null,"source":"fixture","unit":"bytes","sampledAtMs":123}},"totalFreeBytes":{{"state":"ready","value":700,"reason":null,"source":"fixture","unit":"bytes","sampledAtMs":123}},"callerAvailableBytes":{{"state":"ready","value":300,"reason":null,"source":"fixture","unit":"bytes","sampledAtMs":123}}}}"#),
        _ => std::process::exit(2),
    }
}
