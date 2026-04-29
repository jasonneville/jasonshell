use super::provider::{
    provider_health, ProviderHealthContract, ProviderHealthState, ProviderReasonCode,
    SearchProviderId,
};
use serde::{Deserialize, Serialize};
use std::env;
use std::path::PathBuf;
use std::process::Command;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct EverythingInstallationStatus {
    pub installed_exe_path: Option<PathBuf>,
    pub process_running: bool,
    pub service_running: bool,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) enum EverythingSetupAction {
    LaunchInstalled,
    DownloadInstaller,
    RunBundledInstaller,
    OpenOfficialDownload,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) enum EverythingSetupStatus {
    Declined,
    Launched,
    Installed,
    Blocked,
    Failed,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EverythingSetupConsentRequest {
    pub action: EverythingSetupAction,
    pub consent: bool,
    pub official_url: String,
    pub artifact_name: Option<String>,
    pub version: Option<String>,
    pub sha256: Option<String>,
    pub license_approved: bool,
    pub provenance_approved: bool,
    pub requires_admin: bool,
    pub explains_filename_exposure: bool,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EverythingSetupResult {
    pub status: EverythingSetupStatus,
    pub health: ProviderHealthContract,
    pub reason_code: Option<String>,
    pub message: String,
}

pub(crate) fn detect_everything_installation() -> EverythingInstallationStatus {
    EverythingInstallationStatus {
        installed_exe_path: installed_everything_candidates()
            .into_iter()
            .find(|path| path.is_file()),
        process_running: everything_process_running(),
        service_running: everything_service_running(),
    }
}

pub(crate) fn request_everything_setup(
    request: EverythingSetupConsentRequest,
) -> EverythingSetupResult {
    let gate = validate_setup_consent(&request);
    if gate != EverythingSetupStatus::Launched {
        return setup_result(
            gate,
            Some(if gate == EverythingSetupStatus::Declined {
                "userDeclined"
            } else {
                "checksumBlocked"
            }),
            "Everything setup request did not pass consent and provenance gates",
        );
    }

    match request.action {
        EverythingSetupAction::LaunchInstalled => {
            let status = detect_everything_installation();
            let Some(path) = status.installed_exe_path else {
                return setup_result(
                    EverythingSetupStatus::Failed,
                    Some("notInstalled"),
                    "Everything executable was not found",
                );
            };
            match Command::new(&path).arg("-startup").spawn() {
                Ok(_) => setup_result(
                    EverythingSetupStatus::Launched,
                    None,
                    format!("launched {}", path.display()),
                ),
                Err(error) => setup_result(
                    EverythingSetupStatus::Failed,
                    Some("launchFailed"),
                    format!("failed to launch Everything: {error}"),
                ),
            }
        }
        EverythingSetupAction::DownloadInstaller | EverythingSetupAction::RunBundledInstaller => {
            setup_result(
                EverythingSetupStatus::Blocked,
                Some("checksumBlocked"),
                "managed installer execution is blocked until an approved artifact is present",
            )
        }
        EverythingSetupAction::OpenOfficialDownload => setup_result(
            EverythingSetupStatus::Launched,
            None,
            "official download URL passed consent gate",
        ),
    }
}

pub(crate) fn validate_setup_consent(
    request: &EverythingSetupConsentRequest,
) -> EverythingSetupStatus {
    if !request.consent {
        return EverythingSetupStatus::Declined;
    }
    if !is_official_voidtools_url(&request.official_url) || !request.explains_filename_exposure {
        return EverythingSetupStatus::Blocked;
    }

    if matches!(
        request.action,
        EverythingSetupAction::LaunchInstalled
            | EverythingSetupAction::DownloadInstaller
            | EverythingSetupAction::RunBundledInstaller
    ) && (!non_empty(request.version.as_deref())
        || !is_sha256(request.sha256.as_deref())
        || !request.license_approved
        || !request.provenance_approved)
    {
        return EverythingSetupStatus::Blocked;
    }

    EverythingSetupStatus::Launched
}

fn is_official_voidtools_url(value: &str) -> bool {
    value
        .strip_prefix("https://www.voidtools.com/")
        .is_some_and(|suffix| !suffix.trim().is_empty())
}

fn is_sha256(value: Option<&str>) -> bool {
    value.is_some_and(|value| {
        value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
    })
}

fn non_empty(value: Option<&str>) -> bool {
    value.is_some_and(|value| !value.trim().is_empty())
}

fn setup_result(
    status: EverythingSetupStatus,
    reason_code: Option<&str>,
    message: impl Into<String>,
) -> EverythingSetupResult {
    let health = match status {
        EverythingSetupStatus::Launched | EverythingSetupStatus::Installed => provider_health(
            SearchProviderId::Everything,
            ProviderHealthState::Degraded,
            Some(ProviderReasonCode::FallbackActive),
            "Everything setup action launched; provider health must be refreshed",
            false,
        ),
        EverythingSetupStatus::Declined => provider_health(
            SearchProviderId::Everything,
            ProviderHealthState::Degraded,
            Some(ProviderReasonCode::FallbackActive),
            "User declined Everything setup; fallback search remains active",
            true,
        ),
        EverythingSetupStatus::Blocked | EverythingSetupStatus::Failed => provider_health(
            SearchProviderId::Everything,
            ProviderHealthState::Degraded,
            Some(ProviderReasonCode::FallbackActive),
            "Everything setup did not run; fallback search remains active",
            true,
        ),
    };

    EverythingSetupResult {
        status,
        health,
        reason_code: reason_code.map(str::to_string),
        message: message.into(),
    }
}

fn installed_everything_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    for var in ["ProgramFiles", "ProgramFiles(x86)"] {
        if let Some(root) = env::var_os(var).map(PathBuf::from) {
            candidates.push(root.join(r"Everything\Everything.exe"));
        }
    }
    if let Some(profile) = env::var_os("USERPROFILE").map(PathBuf::from) {
        candidates.push(profile.join(r"scoop\apps\everything\current\Everything.exe"));
        candidates.push(profile.join(r"AppData\Local\Programs\Everything\Everything.exe"));
    }
    candidates.push(PathBuf::from(r"C:\Program Files\Everything\Everything.exe"));
    candidates
}

#[cfg(target_os = "windows")]
fn everything_process_running() -> bool {
    use std::mem::size_of;
    use windows::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
    use windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
        TH32CS_SNAPPROCESS,
    };

    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
    let Ok(snapshot) = snapshot else {
        return false;
    };
    if snapshot == INVALID_HANDLE_VALUE {
        return false;
    }

    let mut entry = PROCESSENTRY32W {
        dwSize: size_of::<PROCESSENTRY32W>() as u32,
        ..PROCESSENTRY32W::default()
    };
    let mut found = false;
    let mut ok = unsafe { Process32FirstW(snapshot, &mut entry).is_ok() };
    while ok {
        let name = process_entry_name(&entry);
        if name.eq_ignore_ascii_case("Everything.exe") {
            found = true;
            break;
        }
        ok = unsafe { Process32NextW(snapshot, &mut entry).is_ok() };
    }
    unsafe {
        let _ = CloseHandle(snapshot);
    }
    found
}

#[cfg(not(target_os = "windows"))]
fn everything_process_running() -> bool {
    false
}

#[cfg(target_os = "windows")]
fn process_entry_name(
    entry: &windows::Win32::System::Diagnostics::ToolHelp::PROCESSENTRY32W,
) -> String {
    let end = entry
        .szExeFile
        .iter()
        .position(|value| *value == 0)
        .unwrap_or(entry.szExeFile.len());
    String::from_utf16_lossy(&entry.szExeFile[..end])
}

fn everything_service_running() -> bool {
    let Ok(output) = Command::new("sc.exe")
        .args(["query", "Everything"])
        .output()
    else {
        return false;
    };
    if !output.status.success() {
        return false;
    }
    String::from_utf8_lossy(&output.stdout)
        .to_ascii_uppercase()
        .contains("RUNNING")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_supply_chain_gate_blocks_setup_execution() {
        let request = consent_request(true);
        let request = EverythingSetupConsentRequest {
            license_approved: false,
            ..request
        };

        assert_eq!(
            validate_setup_consent(&request),
            EverythingSetupStatus::Blocked
        );
    }

    #[test]
    fn no_consent_never_launches_everything() {
        let request = consent_request(false);

        assert_eq!(
            validate_setup_consent(&request),
            EverythingSetupStatus::Declined
        );
    }

    #[test]
    fn consent_requires_checksum_provenance_license_and_privacy_notice() {
        let mut request = consent_request(true);
        assert_eq!(
            validate_setup_consent(&request),
            EverythingSetupStatus::Launched
        );

        request.sha256 = None;
        assert_eq!(
            validate_setup_consent(&request),
            EverythingSetupStatus::Blocked
        );

        request = consent_request(true);
        request.provenance_approved = false;
        assert_eq!(
            validate_setup_consent(&request),
            EverythingSetupStatus::Blocked
        );

        request = consent_request(true);
        request.explains_filename_exposure = false;
        assert_eq!(
            validate_setup_consent(&request),
            EverythingSetupStatus::Blocked
        );
    }

    #[test]
    fn official_download_open_does_not_require_artifact_metadata() {
        let request = EverythingSetupConsentRequest {
            action: EverythingSetupAction::OpenOfficialDownload,
            consent: true,
            official_url: "https://www.voidtools.com/downloads/".to_string(),
            artifact_name: None,
            version: None,
            sha256: None,
            license_approved: false,
            provenance_approved: false,
            requires_admin: false,
            explains_filename_exposure: true,
        };

        assert_eq!(
            validate_setup_consent(&request),
            EverythingSetupStatus::Launched
        );
    }

    fn consent_request(consent: bool) -> EverythingSetupConsentRequest {
        EverythingSetupConsentRequest {
            action: EverythingSetupAction::LaunchInstalled,
            consent,
            official_url: "https://www.voidtools.com/downloads/".to_string(),
            artifact_name: Some("Everything-1.4.1.1032.x64-Setup.exe".to_string()),
            version: Some("1.4.1.1032".to_string()),
            sha256: Some("a".repeat(64)),
            license_approved: true,
            provenance_approved: true,
            requires_admin: false,
            explains_filename_exposure: true,
        }
    }
}
