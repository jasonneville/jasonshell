use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioState {
    pub master_volume_percent: f32,
    pub output_devices: Vec<AudioDeviceInfo>,
    pub input_devices: Vec<AudioDeviceInfo>,
    pub default_output_device_id: Option<String>,
    pub default_input_device_id: Option<String>,
    pub sessions: Vec<AudioSessionInfo>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioMasterState {
    pub volume_percent: f32,
    pub muted: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioDeviceInfo {
    pub id: String,
    pub name: String,
    pub flow: String,
    pub is_default: bool,
    pub state: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioSessionInfo {
    pub id: String,
    pub name: String,
    pub process_id: Option<u32>,
    pub volume_percent: f32,
    pub muted: bool,
    pub state: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetAudioVolumeRequest {
    pub volume_percent: f32,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetAppVolumeRequest {
    pub session_id: String,
    pub volume_percent: f32,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetAudioDeviceRequest {
    pub device_id: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetDefaultAudioDeviceRequest {
    pub device_id: String,
    pub flow: String,
}

#[tauri::command]
pub fn get_audio_state() -> Result<AudioState, String> {
    imp::get_audio_state()
}

#[tauri::command]
pub fn list_audio_devices() -> Result<Vec<AudioDeviceInfo>, String> {
    imp::list_audio_devices()
}

#[tauri::command]
pub fn list_audio_sessions() -> Result<Vec<AudioSessionInfo>, String> {
    imp::list_audio_sessions()
}

#[tauri::command]
pub fn set_master_volume(request: SetAudioVolumeRequest) -> Result<AudioState, String> {
    let scalar = percent_to_scalar(request.volume_percent)?;
    imp::set_master_volume_scalar(scalar)?;
    imp::get_audio_state()
}

#[tauri::command]
pub fn set_master_volume_percent(volume_percent: f32) -> Result<(), String> {
    let scalar = percent_to_scalar(volume_percent)?;
    imp::set_master_volume_scalar(scalar)
}

#[tauri::command]
pub fn set_master_mute(muted: bool) -> Result<(), String> {
    imp::set_master_mute(muted)
}

#[tauri::command]
pub fn set_app_session_volume_percent(
    session_id: String,
    volume_percent: f32,
) -> Result<(), String> {
    let scalar = percent_to_scalar(volume_percent)?;
    imp::set_app_session_volume_scalar(session_id, scalar)
}

#[tauri::command]
pub fn set_app_volume(request: SetAppVolumeRequest) -> Result<AudioState, String> {
    let scalar = percent_to_scalar(request.volume_percent)?;
    imp::set_app_session_volume_scalar(request.session_id, scalar)?;
    imp::get_audio_state()
}

#[tauri::command]
pub fn set_app_session_mute(session_id: String, muted: bool) -> Result<(), String> {
    imp::set_app_session_mute(session_id, muted)
}

#[tauri::command]
pub fn set_default_audio_device(request: SetDefaultAudioDeviceRequest) -> Result<(), String> {
    imp::set_default_audio_device(request)
}

#[tauri::command]
pub fn set_default_audio_input_device(
    request: SetAudioDeviceRequest,
) -> Result<AudioState, String> {
    imp::set_default_audio_device(SetDefaultAudioDeviceRequest {
        device_id: request.device_id,
        flow: "input".to_string(),
    })?;
    imp::get_audio_state()
}

#[tauri::command]
pub fn set_default_audio_output_device(
    request: SetAudioDeviceRequest,
) -> Result<AudioState, String> {
    imp::set_default_audio_device(SetDefaultAudioDeviceRequest {
        device_id: request.device_id,
        flow: "output".to_string(),
    })?;
    imp::get_audio_state()
}

fn percent_to_scalar(volume_percent: f32) -> Result<f32, String> {
    if !volume_percent.is_finite() {
        return Err("audio volume percent must be finite".to_string());
    }
    if !(0.0..=100.0).contains(&volume_percent) {
        return Err("audio volume percent must be between 0 and 100".to_string());
    }
    Ok(volume_percent / 100.0)
}

fn scalar_to_percent(scalar: f32) -> f32 {
    (scalar.clamp(0.0, 1.0) * 100.0).round()
}

fn normalize_audio_flow(flow: &str) -> Result<&'static str, String> {
    match flow.trim().to_ascii_lowercase().as_str() {
        "output" | "render" | "speaker" | "speakers" => Ok("output"),
        "input" | "capture" | "microphone" | "mic" => Ok("input"),
        _ => Err("audio device flow must be output or input".to_string()),
    }
}

#[cfg(target_os = "windows")]
mod imp {
    use super::{
        normalize_audio_flow, scalar_to_percent, AudioDeviceInfo, AudioMasterState,
        AudioSessionInfo, AudioState, SetDefaultAudioDeviceRequest,
    };
    use std::ffi::{c_void, OsStr};
    use std::os::windows::ffi::OsStrExt;
    use std::thread;
    use windows::core::{Interface, PCWSTR, PWSTR};
    use windows::Win32::Devices::FunctionDiscovery::PKEY_Device_FriendlyName;
    use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
    use windows::Win32::Media::Audio::{
        eCapture, eCommunications, eConsole, eMultimedia, eRender, AudioSessionState,
        AudioSessionStateActive, AudioSessionStateExpired, AudioSessionStateInactive, EDataFlow,
        ERole, IAudioSessionControl2, IAudioSessionManager2, IMMDevice, IMMDeviceEnumerator,
        ISimpleAudioVolume, MMDeviceEnumerator, DEVICE_STATE_ACTIVE,
    };
    use windows::Win32::System::Com::StructuredStorage::{
        PropVariantClear, PropVariantToStringAlloc,
    };
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CoTaskMemFree, CoUninitialize, CLSCTX_ALL,
        COINIT_MULTITHREADED, STGM_READ,
    };

    const CLSID_POLICY_CONFIG_CLIENT: windows_core::GUID =
        windows_core::GUID::from_u128(0x870af99c_171d_4f9e_af0d_e63df40c2bc9);

    windows_core::imp::define_interface!(
        IPolicyConfig,
        IPolicyConfig_Vtbl,
        0xf8679f50_850a_41cf_9c72_430f290290c8
    );
    windows_core::imp::interface_hierarchy!(IPolicyConfig, windows_core::IUnknown);

    impl IPolicyConfig {
        unsafe fn set_default_endpoint(
            &self,
            device_id: PCWSTR,
            role: ERole,
        ) -> windows_core::Result<()> {
            unsafe {
                (windows_core::Interface::vtable(self).SetDefaultEndpoint)(
                    windows_core::Interface::as_raw(self),
                    device_id,
                    role,
                )
                .ok()
            }
        }
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    pub struct IPolicyConfig_Vtbl {
        pub base__: windows_core::IUnknown_Vtbl,
        pub GetMixFormat: unsafe extern "system" fn(
            *mut c_void,
            PCWSTR,
            *mut *mut c_void,
        ) -> windows_core::HRESULT,
        pub GetDeviceFormat: unsafe extern "system" fn(
            *mut c_void,
            PCWSTR,
            windows_core::BOOL,
            *mut *mut c_void,
        ) -> windows_core::HRESULT,
        pub ResetDeviceFormat:
            unsafe extern "system" fn(*mut c_void, PCWSTR) -> windows_core::HRESULT,
        pub SetDeviceFormat: unsafe extern "system" fn(
            *mut c_void,
            PCWSTR,
            *const c_void,
            *const c_void,
        ) -> windows_core::HRESULT,
        pub GetProcessingPeriod: unsafe extern "system" fn(
            *mut c_void,
            PCWSTR,
            windows_core::BOOL,
            *mut i64,
            *mut i64,
        ) -> windows_core::HRESULT,
        pub SetProcessingPeriod:
            unsafe extern "system" fn(*mut c_void, PCWSTR, *const i64) -> windows_core::HRESULT,
        pub GetShareMode:
            unsafe extern "system" fn(*mut c_void, PCWSTR, *mut c_void) -> windows_core::HRESULT,
        pub SetShareMode:
            unsafe extern "system" fn(*mut c_void, PCWSTR, *const c_void) -> windows_core::HRESULT,
        pub GetPropertyValue: unsafe extern "system" fn(
            *mut c_void,
            PCWSTR,
            *const windows_core::GUID,
            *mut c_void,
        ) -> windows_core::HRESULT,
        pub SetPropertyValue: unsafe extern "system" fn(
            *mut c_void,
            PCWSTR,
            *const windows_core::GUID,
            *const c_void,
        ) -> windows_core::HRESULT,
        pub SetDefaultEndpoint:
            unsafe extern "system" fn(*mut c_void, PCWSTR, ERole) -> windows_core::HRESULT,
        pub SetEndpointVisibility: unsafe extern "system" fn(
            *mut c_void,
            PCWSTR,
            windows_core::BOOL,
        ) -> windows_core::HRESULT,
    }

    pub fn get_audio_state() -> Result<AudioState, String> {
        run_in_mta(|| {
            let enumerator = device_enumerator()?;
            let master = master_state(&enumerator)?;
            let devices = devices(&enumerator)?;
            let sessions = render_sessions(&enumerator)?;
            let default_output_device_id = default_device_id(&enumerator, eRender);
            let default_input_device_id = default_device_id(&enumerator, eCapture);
            let mut output_devices = Vec::new();
            let mut input_devices = Vec::new();
            for device in devices {
                if device.flow == "input" {
                    input_devices.push(device);
                } else {
                    output_devices.push(device);
                }
            }
            Ok(AudioState {
                master_volume_percent: master.volume_percent,
                output_devices,
                input_devices,
                default_output_device_id,
                default_input_device_id,
                sessions,
            })
        })
    }

    pub fn list_audio_devices() -> Result<Vec<AudioDeviceInfo>, String> {
        run_in_mta(|| devices(&device_enumerator()?))
    }

    pub fn list_audio_sessions() -> Result<Vec<AudioSessionInfo>, String> {
        run_in_mta(|| render_sessions(&device_enumerator()?))
    }

    pub fn set_master_volume_scalar(scalar: f32) -> Result<(), String> {
        run_in_mta(move || {
            let enumerator = device_enumerator()?;
            let endpoint = default_endpoint_volume(&enumerator, eRender)?;
            unsafe {
                endpoint
                    .SetMasterVolumeLevelScalar(scalar, std::ptr::null())
                    .map_err(|error| format!("Failed to set master volume: {error}"))
            }
        })
    }

    pub fn set_master_mute(muted: bool) -> Result<(), String> {
        run_in_mta(move || {
            let enumerator = device_enumerator()?;
            let endpoint = default_endpoint_volume(&enumerator, eRender)?;
            unsafe {
                endpoint
                    .SetMute(muted, std::ptr::null())
                    .map_err(|error| format!("Failed to set master mute: {error}"))
            }
        })
    }

    pub fn set_app_session_volume_scalar(session_id: String, scalar: f32) -> Result<(), String> {
        run_in_mta(move || {
            let volume = find_render_session_volume(&device_enumerator()?, &session_id)?;
            unsafe {
                volume
                    .SetMasterVolume(scalar, std::ptr::null())
                    .map_err(|error| format!("Failed to set audio session volume: {error}"))
            }
        })
    }

    pub fn set_app_session_mute(session_id: String, muted: bool) -> Result<(), String> {
        run_in_mta(move || {
            let volume = find_render_session_volume(&device_enumerator()?, &session_id)?;
            unsafe {
                volume
                    .SetMute(muted, std::ptr::null())
                    .map_err(|error| format!("Failed to set audio session mute: {error}"))
            }
        })
    }

    pub fn set_default_audio_device(request: SetDefaultAudioDeviceRequest) -> Result<(), String> {
        let flow = normalize_audio_flow(&request.flow)?;
        run_in_mta(move || {
            let enumerator = device_enumerator()?;
            let device_flow = if flow == "input" { eCapture } else { eRender };
            let available_devices = devices_for_flow(&enumerator, device_flow, flow)?;
            if !available_devices
                .iter()
                .any(|device| device.id == request.device_id)
            {
                return Err(format!("Failed to find active audio {flow} device"));
            }
            set_default_endpoint_for_all_roles(&request.device_id)
        })
    }

    fn run_in_mta<T, F>(operation: F) -> Result<T, String>
    where
        T: Send + 'static,
        F: FnOnce() -> Result<T, String> + Send + 'static,
    {
        thread::spawn(move || {
            unsafe {
                CoInitializeEx(None, COINIT_MULTITHREADED)
                    .ok()
                    .map_err(|error| {
                        format!("Failed to initialize audio COM apartment: {error}")
                    })?;
            }
            let result = operation();
            unsafe {
                CoUninitialize();
            }
            result
        })
        .join()
        .map_err(|_| "Audio operation panicked".to_string())?
    }

    fn device_enumerator() -> Result<IMMDeviceEnumerator, String> {
        unsafe {
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)
                .map_err(|error| format!("Failed to create audio device enumerator: {error}"))
        }
    }

    fn master_state(enumerator: &IMMDeviceEnumerator) -> Result<AudioMasterState, String> {
        let endpoint = default_endpoint_volume(enumerator, eRender)?;
        let volume = unsafe {
            endpoint
                .GetMasterVolumeLevelScalar()
                .map_err(|error| format!("Failed to read master volume: {error}"))?
        };
        let muted = unsafe {
            endpoint
                .GetMute()
                .map_err(|error| format!("Failed to read master mute: {error}"))?
                .as_bool()
        };
        Ok(AudioMasterState {
            volume_percent: scalar_to_percent(volume),
            muted,
        })
    }

    fn default_endpoint_volume(
        enumerator: &IMMDeviceEnumerator,
        flow: EDataFlow,
    ) -> Result<IAudioEndpointVolume, String> {
        let device = unsafe {
            enumerator
                .GetDefaultAudioEndpoint(flow, eConsole)
                .map_err(|error| format!("Failed to get default audio endpoint: {error}"))?
        };
        unsafe {
            device.Activate(CLSCTX_ALL, None).map_err(|error| {
                format!("Failed to activate default audio endpoint volume: {error}")
            })
        }
    }

    fn devices(enumerator: &IMMDeviceEnumerator) -> Result<Vec<AudioDeviceInfo>, String> {
        let mut output = devices_for_flow(enumerator, eRender, "output")?;
        output.extend(devices_for_flow(enumerator, eCapture, "input")?);
        Ok(output)
    }

    fn devices_for_flow(
        enumerator: &IMMDeviceEnumerator,
        flow: EDataFlow,
        flow_label: &str,
    ) -> Result<Vec<AudioDeviceInfo>, String> {
        let default_id = default_device_id(enumerator, flow);
        let collection = unsafe {
            enumerator
                .EnumAudioEndpoints(flow, DEVICE_STATE_ACTIVE)
                .map_err(|error| format!("Failed to enumerate {flow_label} devices: {error}"))?
        };
        let count = unsafe {
            collection
                .GetCount()
                .map_err(|error| format!("Failed to count {flow_label} devices: {error}"))?
        };
        let mut devices = Vec::with_capacity(count as usize);
        for index in 0..count {
            let device = unsafe {
                collection
                    .Item(index)
                    .map_err(|error| format!("Failed to read {flow_label} device: {error}"))?
            };
            let id = device_id(&device)?;
            let state = unsafe {
                device
                    .GetState()
                    .map_err(|error| format!("Failed to read audio device state: {error}"))?
            };
            let name = device_friendly_name(&device).unwrap_or_else(|| id.clone());
            devices.push(AudioDeviceInfo {
                id: id.clone(),
                name,
                flow: flow_label.to_string(),
                is_default: default_id.as_deref() == Some(id.as_str()),
                state: device_state_label(state.0),
            });
        }
        Ok(devices)
    }

    fn default_device_id(enumerator: &IMMDeviceEnumerator, flow: EDataFlow) -> Option<String> {
        let device = unsafe { enumerator.GetDefaultAudioEndpoint(flow, eConsole).ok()? };
        device_id(&device).ok()
    }

    fn render_sessions(enumerator: &IMMDeviceEnumerator) -> Result<Vec<AudioSessionInfo>, String> {
        let manager = default_session_manager(enumerator)?;
        let session_enumerator = unsafe {
            manager
                .GetSessionEnumerator()
                .map_err(|error| format!("Failed to enumerate audio sessions: {error}"))?
        };
        let count = unsafe {
            session_enumerator
                .GetCount()
                .map_err(|error| format!("Failed to count audio sessions: {error}"))?
        };
        let mut sessions = Vec::with_capacity(count as usize);
        for index in 0..count {
            let session = unsafe {
                session_enumerator
                    .GetSession(index)
                    .map_err(|error| format!("Failed to read audio session: {error}"))?
            };
            let session2 = session
                .cast::<IAudioSessionControl2>()
                .map_err(|error| format!("Failed to inspect audio session metadata: {error}"))?;
            let volume = session
                .cast::<ISimpleAudioVolume>()
                .map_err(|error| format!("Failed to inspect audio session volume: {error}"))?;
            let session_id =
                unsafe { pwstr_to_string_and_free(session2.GetSessionInstanceIdentifier().ok()) }
                    .or_else(|| unsafe {
                        pwstr_to_string_and_free(session2.GetSessionIdentifier().ok())
                    })
                    .ok_or_else(|| "Audio session id is unavailable".to_string())?;
            let display_name = unsafe { pwstr_to_string_and_free(session.GetDisplayName().ok()) }
                .filter(|name| !name.trim().is_empty());
            let process_id = unsafe { session2.GetProcessId().ok() }.filter(|pid| *pid != 0);
            let state = unsafe { session.GetState().unwrap_or(AudioSessionStateInactive) };
            let volume_percent = unsafe {
                volume
                    .GetMasterVolume()
                    .map(scalar_to_percent)
                    .map_err(|error| format!("Failed to read audio session volume: {error}"))?
            };
            let muted = unsafe {
                volume
                    .GetMute()
                    .map_err(|error| format!("Failed to read audio session mute: {error}"))?
                    .as_bool()
            };
            let name = display_name.unwrap_or_else(|| {
                process_id
                    .map(|pid| format!("Process {pid}"))
                    .unwrap_or_else(|| "System sounds".to_string())
            });
            sessions.push(AudioSessionInfo {
                id: session_id,
                name,
                process_id,
                volume_percent,
                muted,
                state: session_state_label(state),
            });
        }
        Ok(sessions)
    }

    fn default_session_manager(
        enumerator: &IMMDeviceEnumerator,
    ) -> Result<IAudioSessionManager2, String> {
        let device = unsafe {
            enumerator
                .GetDefaultAudioEndpoint(eRender, eConsole)
                .map_err(|error| format!("Failed to get default render endpoint: {error}"))?
        };
        unsafe {
            device
                .Activate(CLSCTX_ALL, None)
                .map_err(|error| format!("Failed to activate audio session manager: {error}"))
        }
    }

    fn find_render_session_volume(
        enumerator: &IMMDeviceEnumerator,
        requested_session_id: &str,
    ) -> Result<ISimpleAudioVolume, String> {
        let manager = default_session_manager(enumerator)?;
        let session_enumerator = unsafe {
            manager
                .GetSessionEnumerator()
                .map_err(|error| format!("Failed to enumerate audio sessions: {error}"))?
        };
        let count = unsafe {
            session_enumerator
                .GetCount()
                .map_err(|error| format!("Failed to count audio sessions: {error}"))?
        };
        for index in 0..count {
            let session = unsafe {
                session_enumerator
                    .GetSession(index)
                    .map_err(|error| format!("Failed to read audio session: {error}"))?
            };
            let session2 = session
                .cast::<IAudioSessionControl2>()
                .map_err(|error| format!("Failed to inspect audio session metadata: {error}"))?;
            let instance_id =
                unsafe { pwstr_to_string_and_free(session2.GetSessionInstanceIdentifier().ok()) };
            let stable_id =
                unsafe { pwstr_to_string_and_free(session2.GetSessionIdentifier().ok()) };
            if instance_id.as_deref() == Some(requested_session_id)
                || stable_id.as_deref() == Some(requested_session_id)
            {
                return session
                    .cast::<ISimpleAudioVolume>()
                    .map_err(|error| format!("Failed to bind audio session volume: {error}"));
            }
        }
        Err("Audio session is no longer available".to_string())
    }

    fn set_default_endpoint_for_all_roles(device_id: &str) -> Result<(), String> {
        let policy: IPolicyConfig = unsafe {
            CoCreateInstance(&CLSID_POLICY_CONFIG_CLIENT, None, CLSCTX_ALL)
                .map_err(|error| format!("Failed to create Windows audio policy client: {error}"))?
        };
        let device_id = to_wide(device_id);
        for role in [eConsole, eMultimedia, eCommunications] {
            unsafe {
                policy
                    .set_default_endpoint(PCWSTR(device_id.as_ptr()), role)
                    .map_err(|error| {
                        format!("Failed to set Windows default audio endpoint: {error}")
                    })?;
            }
        }
        Ok(())
    }

    fn device_id(device: &IMMDevice) -> Result<String, String> {
        unsafe {
            pwstr_to_string_and_free(device.GetId().ok())
                .ok_or_else(|| "Audio device id is unavailable".to_string())
        }
    }

    fn device_friendly_name(device: &IMMDevice) -> Option<String> {
        let store = unsafe { device.OpenPropertyStore(STGM_READ).ok()? };
        let mut value = unsafe { store.GetValue(&PKEY_Device_FriendlyName).ok()? };
        let name = unsafe { pwstr_to_string_and_free(PropVariantToStringAlloc(&value).ok()) };
        unsafe {
            let _ = PropVariantClear(&mut value);
        }
        name.filter(|value| !value.trim().is_empty())
    }

    unsafe fn pwstr_to_string_and_free(value: Option<PWSTR>) -> Option<String> {
        let value = value?;
        if value.0.is_null() {
            return None;
        }
        let mut length = 0_usize;
        while *value.0.add(length) != 0 {
            length += 1;
        }
        let text = String::from_utf16_lossy(std::slice::from_raw_parts(value.0, length));
        CoTaskMemFree(Some(value.0.cast()));
        Some(text)
    }

    fn session_state_label(state: AudioSessionState) -> String {
        if state == AudioSessionStateActive {
            "active"
        } else if state == AudioSessionStateExpired {
            "expired"
        } else if state == AudioSessionStateInactive {
            "inactive"
        } else {
            "unknown"
        }
        .to_string()
    }

    fn device_state_label(state: u32) -> String {
        if state == DEVICE_STATE_ACTIVE.0 {
            "active".to_string()
        } else {
            format!("state-{state}")
        }
    }

    fn to_wide(value: &str) -> Vec<u16> {
        OsStr::new(value)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect()
    }
}

#[cfg(not(target_os = "windows"))]
mod imp {
    use super::{AudioDeviceInfo, AudioSessionInfo, AudioState, SetDefaultAudioDeviceRequest};

    pub fn get_audio_state() -> Result<AudioState, String> {
        Err("Audio controls are only supported on Windows".to_string())
    }

    pub fn list_audio_devices() -> Result<Vec<AudioDeviceInfo>, String> {
        Ok(Vec::new())
    }

    pub fn list_audio_sessions() -> Result<Vec<AudioSessionInfo>, String> {
        Ok(Vec::new())
    }

    pub fn set_master_volume_scalar(_scalar: f32) -> Result<(), String> {
        Err("Audio controls are only supported on Windows".to_string())
    }

    pub fn set_master_mute(_muted: bool) -> Result<(), String> {
        Err("Audio controls are only supported on Windows".to_string())
    }

    pub fn set_app_session_volume_scalar(_session_id: String, _scalar: f32) -> Result<(), String> {
        Err("Audio controls are only supported on Windows".to_string())
    }

    pub fn set_app_session_mute(_session_id: String, _muted: bool) -> Result<(), String> {
        Err("Audio controls are only supported on Windows".to_string())
    }

    pub fn set_default_audio_device(_request: SetDefaultAudioDeviceRequest) -> Result<(), String> {
        Err("Audio controls are only supported on Windows".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::{normalize_audio_flow, percent_to_scalar, scalar_to_percent};

    #[test]
    fn audio_percent_helpers_use_bounded_scalar_values() {
        assert_eq!(percent_to_scalar(0.0).unwrap(), 0.0);
        assert_eq!(percent_to_scalar(50.0).unwrap(), 0.5);
        assert_eq!(percent_to_scalar(100.0).unwrap(), 1.0);
        assert!(percent_to_scalar(-1.0).is_err());
        assert!(percent_to_scalar(101.0).is_err());
        assert!(percent_to_scalar(f32::NAN).is_err());
        assert_eq!(scalar_to_percent(0.42), 42.0);
        assert_eq!(scalar_to_percent(2.0), 100.0);
    }

    #[test]
    fn audio_flow_aliases_normalize_to_supported_directions() {
        assert_eq!(normalize_audio_flow("output").unwrap(), "output");
        assert_eq!(normalize_audio_flow("render").unwrap(), "output");
        assert_eq!(normalize_audio_flow("mic").unwrap(), "input");
        assert_eq!(normalize_audio_flow("capture").unwrap(), "input");
        assert!(normalize_audio_flow("bluetooth").is_err());
    }
}
