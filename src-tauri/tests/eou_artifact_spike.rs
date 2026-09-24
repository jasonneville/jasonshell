use parakeet_rs::{ParakeetEOU, ParakeetEOUHandle};
use std::fs;
use std::path::PathBuf;

const SAMPLE_RATE: usize = 16_000;
const CHUNK_SAMPLES: usize = 2_560;
const ZERO_FLUSH_CHUNKS: usize = 3;

fn approved_model_dir() -> PathBuf {
    std::env::var_os("JASONSHELL_EOU_MODEL_DIR")
        .map(PathBuf::from)
        .expect("JASONSHELL_EOU_MODEL_DIR must name the approved local Phase 0 artifact")
}

fn approved_fixture_path() -> PathBuf {
    std::env::var_os("JASONSHELL_EOU_FIXTURE_PCM")
        .map(PathBuf::from)
        .expect("JASONSHELL_EOU_FIXTURE_PCM must name approved 16 kHz mono signed-16-bit PCM")
}

fn normalized_words(text: &str) -> String {
    text.chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn transcribe_stream(stream: &mut ParakeetEOU, samples: &[f32]) -> String {
    let mut transcript = String::new();

    for chunk in samples.chunks(CHUNK_SAMPLES) {
        let mut padded_chunk = vec![0.0_f32; CHUNK_SAMPLES];
        padded_chunk[..chunk.len()].copy_from_slice(chunk);
        transcript.push_str(
            &stream
                .transcribe(&padded_chunk, false)
                .expect("EOU stream must accept ordered 160 ms chunks"),
        );
    }

    let silence_chunk = vec![0.0_f32; CHUNK_SAMPLES];
    for _ in 0..ZERO_FLUSH_CHUNKS {
        transcript.push_str(
            &stream
                .transcribe(&silence_chunk, false)
                .expect("EOU stream must accept documented zero-chunk flush"),
        );
    }

    transcript
}

#[test]
#[ignore = "requires the owner-approved local EOU artifact; run with --ignored"]
fn approved_eou_artifact_loads_and_accepts_documented_stream_flush() {
    let model_dir = approved_model_dir();
    for required_file in ["encoder.onnx", "decoder_joint.onnx", "tokenizer.json"] {
        assert!(
            model_dir.join(required_file).is_file(),
            "missing required EOU artifact file: {required_file}"
        );
    }

    let handle = ParakeetEOUHandle::from_pretrained(&model_dir, None)
        .expect("approved EOU artifact must load offline");
    let mut stream = ParakeetEOU::from_shared(&handle);
    let silence_chunk = vec![0.0_f32; CHUNK_SAMPLES];
    let mut outputs = Vec::new();

    for _ in 0..(SAMPLE_RATE / CHUNK_SAMPLES + 1) {
        outputs.push(
            stream
                .transcribe(&silence_chunk, false)
                .expect("EOU stream must accept ordered 160 ms chunks"),
        );
    }

    for _ in 0..ZERO_FLUSH_CHUNKS {
        outputs.push(
            stream
                .transcribe(&silence_chunk, false)
                .expect("EOU stream must accept documented zero-chunk flush"),
        );
    }

    assert!(
        outputs.iter().all(String::is_empty),
        "silent Phase 0 fixture must not emit transcript content"
    );
}

#[test]
#[ignore = "requires owner-approved local EOU artifact and public licensed fixture; run with --ignored"]
fn approved_eou_artifact_transcribes_public_fixture_without_duplicate_tail() {
    let model_dir = approved_model_dir();
    let fixture_bytes = fs::read(approved_fixture_path())
        .expect("approved fixture PCM must be readable without network access");
    assert!(
        fixture_bytes.len() % 2 == 0,
        "approved fixture must be signed-16-bit PCM"
    );
    let samples = fixture_bytes
        .chunks_exact(2)
        .map(|bytes| i16::from_le_bytes([bytes[0], bytes[1]]) as f32 / i16::MAX as f32)
        .collect::<Vec<_>>();

    let handle = ParakeetEOUHandle::from_pretrained(&model_dir, None)
        .expect("approved EOU artifact must load offline");
    let mut stream = ParakeetEOU::from_shared(&handle);
    let transcript = transcribe_stream(&mut stream, &samples);

    assert!(
        normalized_words(&transcript) == "a golden fortune and a happy life",
        "fixture normalized transcript must match once without duplicate or dropped tail"
    );
}
