//! All volume metadata runs in the same executable before Tauri startup.
use super::{
    native_runtime::{now, RuntimeInner},
    probe_executor::{execute_command, ProbeError},
    system_info::*,
};
use std::{
    process::Command,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use windows::{
    core::PCWSTR,
    Win32::Storage::FileSystem::{
        GetDiskFreeSpaceExW, GetDriveTypeW, GetLogicalDrives, GetVolumeInformationW,
    },
};

pub fn roots() -> Vec<String> {
    let mask = unsafe { GetLogicalDrives() };
    (0..26)
        .filter(|bit| mask & (1 << bit) != 0)
        .map(|bit| format!("{}:\\", (b'A' + bit) as char))
        .collect()
}
fn unavailable(root: String, reason: SampleReason) -> StorageRoot {
    StorageRoot {
        root,
        kind: "unknown".into(),
        filesystem: Sample::unavailable("GetVolumeInformationW", None, reason),
        total_bytes: Sample::unavailable("GetDiskFreeSpaceExW", Some("bytes"), reason),
        total_free_bytes: Sample::unavailable("GetDiskFreeSpaceExW", Some("bytes"), reason),
        caller_available_bytes: Sample::unavailable("GetDiskFreeSpaceExW", Some("bytes"), reason),
    }
}
pub fn helper(args: &[String]) -> i32 {
    if args.len() != 2
        || args[0] != "--internal-system-info-volume"
        || !valid_probe_root(&args[1])
        || !roots().contains(&args[1])
    {
        return 2;
    }
    let root = &args[1];
    let wide: Vec<_> = root.encode_utf16().chain(Some(0)).collect();
    let kind = match unsafe { GetDriveTypeW(PCWSTR(wide.as_ptr())) } {
        2 => "removable",
        3 => "fixed",
        4 => "mapped",
        _ => "unknown",
    };
    let mut result = unavailable(root.clone(), SampleReason::QueryFailed);
    result.kind = kind.into();
    if kind != "unknown" {
        let mut available = 0;
        let mut total = 0;
        let mut free = 0;
        if unsafe {
            GetDiskFreeSpaceExW(
                PCWSTR(wide.as_ptr()),
                Some(&mut available),
                Some(&mut total),
                Some(&mut free),
            )
        }
        .is_ok()
        {
            let sample = |value| {
                safe_number(value)
                    .map(|value| Sample::ready(value, "GetDiskFreeSpaceExW", Some("bytes"), now()))
                    .unwrap_or_else(|| {
                        Sample::unavailable(
                            "GetDiskFreeSpaceExW",
                            Some("bytes"),
                            SampleReason::LimitReached,
                        )
                    })
            };
            // Windows reports caller-visible total capacity here: quotas can
            // reduce it. This is not necessarily the physical volume capacity.
            result.total_bytes = sample(total);
            result.total_free_bytes = sample(free);
            result.caller_available_bytes = sample(available);
        }
        let mut filesystem = [0u16; 64];
        if unsafe {
            GetVolumeInformationW(
                PCWSTR(wide.as_ptr()),
                None,
                None,
                None,
                None,
                Some(&mut filesystem),
            )
        }
        .is_ok()
        {
            let end = filesystem
                .iter()
                .position(|c| *c == 0)
                .unwrap_or(filesystem.len());
            result.filesystem = Sample::ready(
                String::from_utf16_lossy(&filesystem[..end]),
                "GetVolumeInformationW",
                None,
                now(),
            );
        }
    }
    match serde_json::to_string(&result) {
        Ok(json) if json.len() <= 8192 => {
            println!("{json}");
            0
        }
        _ => 1,
    }
}
pub fn collect(inner: &Arc<Mutex<RuntimeInner>>, origin: Instant, work: WorkToken) -> Storage {
    let mut output = Vec::new();
    for root in roots().into_iter().take(32) {
        let token = {
            let Ok(mut state) = inner.lock() else {
                break;
            };
            let context = state.coordinator.context();
            if !context.visible
                || context.active_tab != ProcessManagerTab::SystemInfo
                || context.open_generation != work.generation
                || context.tab_revision != work.revision
            {
                break;
            }
            state.probes.reserve(
                &root,
                work.generation,
                work.revision,
                origin.elapsed().as_millis() as u64,
            )
        };
        let Some(token) = token else {
            output.push(unavailable(root, SampleReason::LimitReached));
            continue;
        };
        let result = std::env::current_exe()
            .map_err(|_| ProbeError::Spawn)
            .and_then(|exe| {
                execute_command(
                    Command::new(exe)
                        .arg("--internal-system-info-volume")
                        .arg(&root),
                    || {
                        inner
                            .lock()
                            .map(|state| {
                                let c = state.coordinator.context();
                                !c.visible
                                    || c.active_tab != ProcessManagerTab::SystemInfo
                                    || c.open_generation != work.generation
                                    || c.tab_revision != work.revision
                            })
                            .unwrap_or(true)
                    },
                    Duration::from_millis(1500),
                    8192,
                )
            });
        // execute_command never returns with a live/unreaped child.
        let publish = inner
            .lock()
            .map(|mut state| {
                let publish = state
                    .probes
                    .may_publish(&token, work.generation, work.revision);
                state.probes.reaped(&token);
                publish
            })
            .unwrap_or(false);
        if !publish {
            break;
        }
        let sample = match result {
            Ok(bytes) => serde_json::from_slice::<StorageRoot>(&bytes)
                .ok()
                .filter(|value| value.root == root)
                .unwrap_or_else(|| unavailable(root.clone(), SampleReason::QueryFailed)),
            Err(ProbeError::TimedOut) => unavailable(root.clone(), SampleReason::TimedOut),
            Err(ProbeError::Cancelled) => break,
            Err(_) => unavailable(root.clone(), SampleReason::QueryFailed),
        };
        output.push(sample);
    }
    Storage {
        state: if output
            .iter()
            .all(|root| matches!(root.total_bytes.state, SampleState::Ready))
        {
            SampleState::Ready
        } else {
            SampleState::Partial
        },
        sampled_at_ms: Some(now()),
        roots: output,
    }
}
