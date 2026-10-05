//! Metadata-only opt-in subprocess protocol. No desktop, visible HWND or shell action.
use crate::snipping::{clipboard::{self, Failpoints}, clipboard_process::{Outcome, ProcessOwner}, geometry::{CroppedImage, MAX_STAGING_BYTES}};
use serde_json::{json, Value};
use std::{io::{BufRead, Read, Write}, sync::mpsc, time::{Duration, Instant}};

fn report(value: Value) -> Result<(), &'static str> {
    let line = value.to_string();
    if line.len() > 4096 { return Err("driver report budget"); }
    println!("{line}"); std::io::stdout().flush().map_err(|_| "driver report failed")
}
fn synthetic() -> CroppedImage { CroppedImage { width: 2, height: 2,
    pixels: vec![11,22,33,255,44,55,66,255,77,88,99,255,111,122,133,255] } }

pub fn run(args: &[String]) -> Result<(), String> {
    // Fail closed on duplicate/unknown options before apartment or publication.
    let valid = (args.len() == 3 || args.len() == 4)
        && args[0] == "--consent-synthetic-clipboard-test" && args[1] == "--clipboard-test-case"
        && (args.len() == 3 || args[3] == "--hold-until-reader");
    if !valid { return Err("invalid opt-in clipboard driver arguments".into()); }
    let case = args[2].as_str();
    if !matches!(case, "invalid-image" | "budget" | "allocation" | "ole-set-rejected" | "real-busy" | "success" | "flush-after-commit" | "repeat-cleanup-20" | "helper-hang-operation" | "helper-hang-flush" | "helper-hang-shutdown" | "durable-redundant-flush-timeout") {
        return Err("unknown clipboard case".into());
    }
    if (case == "flush-after-commit") != (args.len() == 4) { return Err("postcommit observation requires bounded hold".into()); }
    let mut publisher = ProcessOwner::spawn(&std::env::current_exe().map_err(|_| "helper executable unavailable")?)?;
    if case == "durable-redundant-flush-timeout" {
        if !matches!(publisher.publish(synthetic(), 16, Default::default()), Outcome::Committed { durable: true }) {
            return Err("initial native publication did not establish durability".into());
        }
        publisher.arm_redundant_flush_response_fault_for_test()?;
        let started = Instant::now();
        let outcome = publisher.flush();
        let operation_ms = started.elapsed().as_millis();
        let fault_used = publisher.redundant_flush_response_fault_used_for_test();
        let shutdown = publisher.shutdown();
        let mut value = outcome.metadata();
        value["case"] = json!(case); value["initialCommitted"] = json!(true); value["initialDurable"] = json!(true);
        value["writerDrained"] = json!(shutdown.writer_drained); value["helperExited"] = json!(shutdown.exited);
        value["operationMs"] = json!(operation_ms); value["faultUsed"] = json!(fault_used);
        value["shortCircuited"] = json!(!fault_used);
        value["failpoint"] = json!("redundant-flush-response-unavailable-after-confirmed-durable-publication");
        report(value)?;
        if !matches!(outcome, Outcome::Committed { durable: true }) || !shutdown.exited || !shutdown.writer_drained || operation_ms > 3000 {
            return Err("known durability or helper cleanup regression".into());
        }
        return Ok(());
    }
    if case.starts_with("helper-hang-") {
        let started = Instant::now();
        let fail = Failpoints { stall_before_set: case == "helper-hang-operation", stall_after_commit: case == "helper-hang-flush",
            stall_shutdown: case == "helper-hang-shutdown", flush: case == "helper-hang-shutdown", ..Default::default() };
        let outcome = publisher.publish(synthetic(), 16, fail);
        let operation_ms = started.elapsed().as_millis();
        let shutdown_started = Instant::now();
        let shutdown = publisher.shutdown();
        let mut value = if case == "helper-hang-shutdown" { shutdown.outcome.metadata() } else { outcome.metadata() };
        value["case"] = json!(case); value["operationMs"] = json!(operation_ms);
        value["shutdownMs"] = json!(shutdown_started.elapsed().as_millis());
        value["helperExited"] = json!(shutdown.exited); value["forcedShutdown"] = json!(shutdown.forced);
        value["writerDrained"] = json!(shutdown.writer_drained);
        value["failpoint"] = json!("permanent-helper-stall-at-native-boundary-not-an-OS-COM-hang-claim");
        report(value)?;
        if !shutdown.exited || !shutdown.writer_drained || operation_ms > 3000 || shutdown_started.elapsed() > Duration::from_secs(1) { return Err("isolated helper deadline/cleanup failed".into()); }
        return Ok(());
    }
    if case == "repeat-cleanup-20" {
        let publish = |publisher: &mut ProcessOwner| -> Result<(), String> {
            if !matches!(publisher.publish(synthetic(), 16, Default::default()), Outcome::Committed { durable: true }) {
                return Err("repeat publication committed but not durable".into());
            }
            Ok(())
        };
        publish(&mut publisher)?;
        clipboard::pump();
        let before = publisher.metrics()?;
        let mut successful = 0;
        let mut failed = 0;
        for index in 0..20 {
            publish(&mut publisher)?; successful += 1;
            let fail = Failpoints { allocation: index % 2 == 0, set: index % 2 != 0, ..Default::default() };
            if !matches!(publisher.publish(synthetic(), 16, fail), Outcome::Rejected(_)) { return Err("injected rejection unexpectedly committed".into()); }
            failed += 1;
            let counts = publisher.metrics()?;
            for field in ["liveObjects", "liveGlobalAllocations", "liveStagedBytes"] {
                if counts[field] != 0 { return Err("helper owned resource retained after iteration".into()); }
            }
        }
        clipboard::pump();
        let after = publisher.metrics()?;
        report(json!({ "case": case, "iterations":20, "successfulPublications": successful, "failedPublications": failed, "before": before, "after": after }))?;
        return Ok(());
    }
    let mut image = synthetic();
    if case == "invalid-image" { image.pixels.pop(); }
    let retained = if case == "budget" { MAX_STAGING_BYTES } else { image.pixels.capacity() };
    let fail = Failpoints { allocation: case == "allocation", set: case == "ole-set-rejected", flush: case == "flush-after-commit", ..Default::default() };
    match publisher.publish(image, retained, fail) {
        Outcome::Rejected(code) => {
            report(json!({ "case":case, "status":"rejected", "code":code, "committed":false, "durable":false }))?;
            if matches!(case, "success" | "flush-after-commit") { return Err("expected native commit rejected".into()); }
            if case == "real-busy" && code != "clipboard-busy" { return Err("real busy did not exercise native busy outcome".into()); }
        }
        Outcome::Committed { durable: true } => {
            report(json!({ "case":case, "status":"committed", "committed":true, "durable":true }))?;
            if case != "success" { return Err("expected failure path not exercised".into()); }
        }
        Outcome::Committed { durable: false } => {
            report(json!({ "case":case, "status":"committed-warning", "code":"clipboard-committed-not-durable", "committed":true, "durable":false }))?;
            if case != "flush-after-commit" { return Err("unexpected postcommit durability failure".into()); }
            // Reader thread owns stdin only, never COM/image resources. Publication and
            // message servicing remain on the creating STA without timeout abandonment.
            let (tx, rx) = mpsc::sync_channel(1);
            let reader = std::thread::spawn(move || {
                let mut line = String::new();
                let result = std::io::BufReader::new(std::io::stdin()).take(64).read_line(&mut line);
                let _ = tx.send(result.is_ok() && line == "release-after-observation\n");
            });
            let deadline = Instant::now() + Duration::from_secs(5);
            let released = loop {
                clipboard::pump();
                if let Ok(value) = rx.try_recv() { break value; }
                if Instant::now() >= deadline { break false; }
                std::thread::sleep(Duration::from_millis(5));
            };
            if released { let _ = reader.join(); }
            // Even on observation timeout recover materialization before apartment teardown.
            if !matches!(publisher.flush(), Outcome::Committed { durable: true }) { return Err("helper flush did not establish durability".into()); }
            if !released { return Err("reader observation timed out after committed publication; data materialized during recovery".into()); }
            report(json!({ "case":case, "status":"committed", "committed":true, "durable":true }))?;
        }
        outcome => { let mut value = outcome.metadata(); value["case"] = json!(case); report(value)?; return Err("helper publication unknown or durability lost".into()); }
    }
    Ok(())
}
