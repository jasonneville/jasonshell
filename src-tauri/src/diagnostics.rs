use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::VecDeque;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::State;

const DEFAULT_DIAGNOSTIC_CAPACITY: usize = 200;
const REDACTED: &str = "[REDACTED]";

#[derive(Default)]
pub struct DiagnosticsRuntimeState {
    entries: VecDeque<DiagnosticEntry>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DiagnosticLevel {
    Debug,
    Info,
    Warn,
    Error,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct IncomingDiagnosticEntry {
    pub level: DiagnosticLevel,
    pub source: String,
    pub message: String,
    #[serde(default)]
    pub fields: Map<String, Value>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticEntry {
    pub timestamp_epoch_ms: u128,
    pub level: DiagnosticLevel,
    pub source: String,
    pub message: String,
    pub fields: Map<String, Value>,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticExport {
    pub generated_at_epoch_ms: u128,
    pub entries: Vec<DiagnosticEntry>,
}

pub fn diagnostics_state() -> Mutex<DiagnosticsRuntimeState> {
    Mutex::new(DiagnosticsRuntimeState::default())
}

#[tauri::command]
pub fn record_diagnostic(
    state: State<'_, Mutex<DiagnosticsRuntimeState>>,
    entry: IncomingDiagnosticEntry,
) -> Result<(), String> {
    let mut guard = state
        .lock()
        .map_err(|_| "diagnostics state is unavailable".to_string())?;
    record_diagnostic_entry(&mut guard, entry, current_epoch_ms());
    Ok(())
}

#[tauri::command]
pub fn export_diagnostics(
    state: State<'_, Mutex<DiagnosticsRuntimeState>>,
) -> Result<DiagnosticExport, String> {
    let guard = state
        .lock()
        .map_err(|_| "diagnostics state is unavailable".to_string())?;
    Ok(DiagnosticExport {
        generated_at_epoch_ms: current_epoch_ms(),
        entries: guard.entries.iter().cloned().collect(),
    })
}

fn record_diagnostic_entry(
    state: &mut DiagnosticsRuntimeState,
    entry: IncomingDiagnosticEntry,
    timestamp_epoch_ms: u128,
) {
    state.entries.push_back(DiagnosticEntry {
        timestamp_epoch_ms,
        level: entry.level,
        source: redact_text(&entry.source),
        message: redact_text(&entry.message),
        fields: redact_fields(entry.fields),
    });

    while state.entries.len() > DEFAULT_DIAGNOSTIC_CAPACITY {
        state.entries.pop_front();
    }
}

fn redact_fields(fields: Map<String, Value>) -> Map<String, Value> {
    fields
        .into_iter()
        .map(|(key, value)| {
            let value = if is_secret_key(&key) {
                Value::String(REDACTED.to_string())
            } else {
                redact_value(value)
            };
            (key, value)
        })
        .collect()
}

fn redact_value(value: Value) -> Value {
    match value {
        Value::String(text) => Value::String(redact_text(&text)),
        Value::Array(items) => Value::Array(items.into_iter().map(redact_value).collect()),
        Value::Object(fields) => Value::Object(redact_fields(fields)),
        value => value,
    }
}

fn redact_text(text: &str) -> String {
    let mut redacted = Vec::new();
    for token in text.split_whitespace() {
        if token.to_ascii_lowercase().starts_with("bearer") {
            redacted.push("Bearer");
        } else if redacted.last().copied() == Some("Bearer") {
            redacted.push(REDACTED);
        } else {
            redacted.push(token);
        }
    }
    redacted.join(" ")
}

fn is_secret_key(key: &str) -> bool {
    let key = key.to_ascii_lowercase();
    [
        "token",
        "secret",
        "password",
        "credential",
        "api_key",
        "apikey",
        "authorization",
        "cookie",
    ]
    .iter()
    .any(|needle| key.contains(needle))
}

fn current_epoch_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn incoming(message: &str) -> IncomingDiagnosticEntry {
        IncomingDiagnosticEntry {
            level: DiagnosticLevel::Warn,
            source: "settings".to_string(),
            message: message.to_string(),
            fields: Map::new(),
        }
    }

    #[test]
    fn diagnostic_ring_buffer_is_bounded() {
        let mut state = DiagnosticsRuntimeState::default();
        for index in 0..(DEFAULT_DIAGNOSTIC_CAPACITY + 3) {
            record_diagnostic_entry(
                &mut state,
                incoming(&format!("entry {index}")),
                index as u128,
            );
        }

        assert_eq!(state.entries.len(), DEFAULT_DIAGNOSTIC_CAPACITY);
        assert_eq!(state.entries.front().unwrap().message, "entry 3");
    }

    #[test]
    fn diagnostics_redact_secret_fields_and_bearer_tokens() {
        let mut fields = Map::new();
        fields.insert("apiToken".to_string(), Value::String("abc".to_string()));
        fields.insert(
            "path".to_string(),
            Value::String("Bearer should-not-export".to_string()),
        );
        let entry = IncomingDiagnosticEntry {
            level: DiagnosticLevel::Error,
            source: "auth".to_string(),
            message: "failed with Bearer secret-token".to_string(),
            fields,
        };
        let mut state = DiagnosticsRuntimeState::default();

        record_diagnostic_entry(&mut state, entry, 1);

        let exported = state.entries.pop_front().unwrap();
        assert_eq!(exported.message, "failed with Bearer [REDACTED]");
        assert_eq!(
            exported.fields["apiToken"],
            Value::String(REDACTED.to_string())
        );
        assert_eq!(
            exported.fields["path"],
            Value::String("Bearer [REDACTED]".to_string())
        );
    }
}
