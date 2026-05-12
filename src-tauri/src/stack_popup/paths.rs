use std::env;
use std::fs;
use std::path::{Path, PathBuf};

pub(crate) fn normalize_existing_dir(path: &str) -> Result<String, String> {
    let resolved = normalize_existing_path(path)?;
    if !Path::new(&resolved).is_dir() {
        return Err("Stack path is not a folder".to_string());
    }
    Ok(resolved)
}

pub(crate) fn normalize_existing_path(path: &str) -> Result<String, String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err("Stack path is empty".to_string());
    }

    let candidate = normalize_stack_path_candidate(trimmed);

    let pathbuf =
        resolve_stack_alias_path(&candidate).unwrap_or_else(|| PathBuf::from(candidate.clone()));
    fs::canonicalize(&pathbuf)
        .map(|path| stack_display_path_string(&path.to_string_lossy()))
        .map_err(|error| format!("Failed to resolve stack path: {error}"))
}

pub(crate) fn stack_display_path_string(value: &str) -> String {
    let trimmed = value.trim();
    if let Some(rest) = trimmed.strip_prefix("\\\\?\\UNC\\") {
        format!(r"\\{rest}")
    } else if let Some(rest) = trimmed.strip_prefix("\\??\\UNC\\") {
        format!(r"\\{rest}")
    } else if let Some(rest) = trimmed.strip_prefix("\\\\?\\") {
        rest.to_string()
    } else if let Some(rest) = trimmed.strip_prefix("\\??\\") {
        rest.to_string()
    } else {
        trimmed.to_string()
    }
}

pub(crate) fn paths_match_for_unpin(pin_path: &str, requested_path: &str) -> bool {
    if let (Ok(pin), Ok(requested)) = (
        normalize_existing_path(pin_path),
        normalize_existing_path(requested_path),
    ) {
        return pin.eq_ignore_ascii_case(&requested);
    }

    raw_path_key(pin_path) == raw_path_key(requested_path)
}

fn raw_path_key(path: &str) -> String {
    normalize_stack_path_candidate(path)
        .trim_end_matches(['\\', '/'])
        .replace('/', "\\")
        .to_lowercase()
}

pub(crate) fn normalize_stack_path_candidate(path: &str) -> String {
    let trimmed = path.trim().trim_matches('"');
    let mut candidate = file_uri_to_path(trimmed).unwrap_or_else(|| trimmed.to_string());

    // Strip common extended-path artifacts produced when constructing file:// URIs from
    // Windows canonical paths (which can include the "\\?\\" prefix). After
    // converting slashes, this can manifest as a leading "?/" or "?\\". Remove it.
    if candidate.starts_with("?\\") || candidate.starts_with("?/") {
        candidate = candidate[2..].to_string();
    }
    stack_display_path_string(&candidate)
}

fn file_uri_to_path(value: &str) -> Option<String> {
    if !value
        .get(..7)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("file://"))
    {
        return None;
    }

    let rest = &value[7..];
    let (host, path) = if rest.starts_with('/') {
        ("", rest)
    } else {
        match rest.find('/') {
            Some(index) => (&rest[..index], &rest[index..]),
            None => (rest, ""),
        }
    };
    let host = percent_decode(host);
    let mut path = percent_decode(path);

    if !host.is_empty() && !host.eq_ignore_ascii_case("localhost") {
        path = path.trim_start_matches('/').replace('/', "\\");
        return Some(if path.is_empty() {
            format!(r"\\{host}")
        } else {
            format!(r"\\{host}\{path}")
        });
    }

    #[cfg(windows)]
    {
        while path.starts_with('/') {
            path.remove(0);
        }
        Some(path.replace('/', "\\"))
    }
    #[cfg(not(windows))]
    {
        Some(path)
    }
}

fn percent_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            if let Ok(hex) = std::str::from_utf8(&bytes[index + 1..index + 3]) {
                if let Ok(byte) = u8::from_str_radix(hex, 16) {
                    decoded.push(byte);
                    index += 3;
                    continue;
                }
            }
        }
        decoded.push(bytes[index]);
        index += 1;
    }
    String::from_utf8_lossy(&decoded).into_owned()
}

pub(crate) fn resolve_stack_alias_path(path: &str) -> Option<PathBuf> {
    let profile = user_profile_dir()?;
    resolve_stack_alias_with_profile(path, &profile)
}

pub(crate) fn user_profile_dir() -> Option<PathBuf> {
    env::var_os("USERPROFILE")
        .or_else(|| env::var_os("HOME"))
        .map(PathBuf::from)
}

pub(crate) fn resolve_stack_alias_with_profile(path: &str, profile: &Path) -> Option<PathBuf> {
    let alias = path.strip_prefix("shell:")?;
    if alias.eq_ignore_ascii_case("profile") {
        return Some(profile.to_path_buf());
    }
    if alias.eq_ignore_ascii_case("desktop") {
        return Some(profile.join("Desktop"));
    }
    if alias.eq_ignore_ascii_case("personal") || alias.eq_ignore_ascii_case("documents") {
        return Some(profile.join("Documents"));
    }
    if alias.eq_ignore_ascii_case("downloads") {
        return Some(profile.join("Downloads"));
    }
    None
}

pub(crate) fn validate_child_name(name: &str) -> Result<&str, String> {
    if name.trim().is_empty() {
        return Err("Name cannot be empty".to_string());
    }
    if name.contains('\\') || name.contains('/') {
        return Err("Name cannot contain path separators".to_string());
    }
    if name.ends_with('.') || name.ends_with(' ') {
        return Err("Name cannot end with a dot or space".to_string());
    }
    if name.chars().any(|ch| ch.is_control()) {
        return Err("Name cannot contain control characters".to_string());
    }
    if name
        .chars()
        .any(|ch| matches!(ch, '<' | '>' | ':' | '"' | '|' | '?' | '*'))
    {
        return Err("Name contains characters Windows does not allow".to_string());
    }
    let basename = name.split('.').next().unwrap_or(name).to_ascii_uppercase();
    if matches!(basename.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || reserved_numbered_device_name(&basename, "COM")
        || reserved_numbered_device_name(&basename, "LPT")
    {
        return Err("Name is reserved by Windows".to_string());
    }
    Ok(name)
}

fn reserved_numbered_device_name(name: &str, prefix: &str) -> bool {
    name.len() == 4
        && name.starts_with(prefix)
        && name
            .as_bytes()
            .get(3)
            .is_some_and(|digit| (b'1'..=b'9').contains(digit))
}
