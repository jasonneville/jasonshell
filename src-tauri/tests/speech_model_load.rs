#[path = "../src/speech_model.rs"]
mod speech_model;

use std::path::{Path, PathBuf};

const MODEL_DIRECTORY: &str = "parakeet-tdt-0.6b-v2-int8";

fn packaged_model_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("resources")
        .join("speech-models")
        .join(MODEL_DIRECTORY)
}

#[test]
fn packaged_model_layout_has_required_parakeet_tdt_files() {
    let result = speech_model::validate_parakeet_tdt_layout(&packaged_model_path());

    assert!(result.is_ok(), "packaged model layout must be valid");
}

#[test]
fn missing_model_directory_returns_user_safe_error() {
    let missing = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("resources")
        .join("speech-models")
        .join("missing-model");

    let error = speech_model::validate_parakeet_tdt_layout(&missing)
        .expect_err("missing model directory must fail closed");

    assert_eq!(error.to_string(), "speech model is unavailable");
    assert!(!error.to_string().contains("missing-model"));
}

#[test]
#[ignore = "requires 650 MB packaged Parakeet resource"]
fn packaged_parakeet_model_loads() {
    let result = speech_model::load_parakeet_tdt(&packaged_model_path());

    assert!(result.is_ok(), "packaged Parakeet model must load on CPU");
}
