//! Narrow in-process loader for packaged Parakeet TDT models.

use parakeet_rs::ParakeetTDT;
use std::fmt;
use std::path::Path;

const REQUIRED_FILES: [&str; 3] = [
    "encoder-model.int8.onnx",
    "decoder_joint-model.int8.onnx",
    "vocab.txt",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct SpeechModelError;

impl fmt::Display for SpeechModelError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("speech model is unavailable")
    }
}

impl std::error::Error for SpeechModelError {}

pub(crate) fn validate_parakeet_tdt_layout(model_directory: &Path) -> Result<(), SpeechModelError> {
    if !model_directory.is_dir()
        || REQUIRED_FILES
            .iter()
            .any(|file_name| !model_directory.join(file_name).is_file())
    {
        return Err(SpeechModelError);
    }

    Ok(())
}

pub(crate) fn load_parakeet_tdt(model_directory: &Path) -> Result<ParakeetTDT, SpeechModelError> {
    validate_parakeet_tdt_layout(model_directory)?;
    ParakeetTDT::from_pretrained(model_directory, None).map_err(|_| SpeechModelError)
}
