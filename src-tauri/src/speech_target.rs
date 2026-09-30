//! Foreground-window snapshots for safe dictation paste delivery.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct SpeechPasteTarget {
    #[cfg(target_os = "windows")]
    hwnd: isize,
    #[cfg(target_os = "windows")]
    process_id: u32,
    #[cfg(target_os = "windows")]
    thread_id: u32,
    #[cfg(target_os = "windows")]
    focus_hwnd: isize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SpeechPasteFailure {
    TargetUnavailable,
    TargetChanged,
    FocusDenied,
    InputRejected,
}

impl SpeechPasteFailure {
    pub(crate) const fn code(self) -> &'static str {
        match self {
            Self::TargetUnavailable => "paste-target-unavailable",
            Self::TargetChanged => "paste-target-changed",
            Self::FocusDenied => "paste-focus-denied",
            Self::InputRejected => "paste-input-rejected",
        }
    }
}

#[cfg(target_os = "windows")]
fn target_identity(hwnd: windows::Win32::Foundation::HWND) -> Option<SpeechPasteTarget> {
    use windows::Win32::UI::WindowsAndMessaging::{
        GetGUIThreadInfo, GetWindowThreadProcessId, IsWindow, GUITHREADINFO,
    };

    if hwnd.0.is_null() || !unsafe { IsWindow(Some(hwnd)).as_bool() } {
        return None;
    }
    let mut process_id = 0;
    let thread_id = unsafe { GetWindowThreadProcessId(hwnd, Some(&mut process_id)) };
    if thread_id == 0 || process_id == 0 {
        return None;
    }
    let mut thread_info = GUITHREADINFO {
        cbSize: std::mem::size_of::<GUITHREADINFO>() as u32,
        ..Default::default()
    };
    if unsafe { GetGUIThreadInfo(thread_id, &mut thread_info) }.is_err()
        || thread_info.hwndFocus.0.is_null()
    {
        return None;
    }
    Some(SpeechPasteTarget {
        hwnd: hwnd.0 as isize,
        process_id,
        thread_id,
        focus_hwnd: thread_info.hwndFocus.0 as isize,
    })
}

/// Snapshot the current foreground window before a bar button takes focus.
pub(crate) fn capture_speech_paste_target() -> Result<SpeechPasteTarget, SpeechPasteFailure> {
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;
        return target_identity(unsafe { GetForegroundWindow() })
            .ok_or(SpeechPasteFailure::TargetUnavailable);
    }
    #[cfg(not(target_os = "windows"))]
    Err(SpeechPasteFailure::TargetUnavailable)
}

/// Restore only the captured window and focused control, revalidate both, then emit Ctrl+V.
///
/// A changed or unavailable target fails closed rather than pasting into the
/// current foreground application.
pub(crate) fn paste_to_captured_target(
    target: SpeechPasteTarget,
) -> Result<(), SpeechPasteFailure> {
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;
        let hwnd = windows::Win32::Foundation::HWND(target.hwnd as *mut _);
        if target_identity(hwnd) != Some(target)
            || target_identity(unsafe { GetForegroundWindow() }) != Some(target)
        {
            return Err(SpeechPasteFailure::TargetChanged);
        }
    }
    prepare_captured_target(target)?;
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::UI::Input::KeyboardAndMouse::{
            SendInput, INPUT, INPUT_KEYBOARD, KEYEVENTF_KEYUP, VIRTUAL_KEY, VK_CONTROL,
        };
        let mut inputs: [INPUT; 4] = unsafe { std::mem::zeroed() };
        for input in &mut inputs {
            input.r#type = INPUT_KEYBOARD;
        }
        inputs[0].Anonymous.ki.wVk = VK_CONTROL;
        inputs[1].Anonymous.ki.wVk = VIRTUAL_KEY(0x56);
        inputs[2].Anonymous.ki.wVk = VIRTUAL_KEY(0x56);
        inputs[2].Anonymous.ki.dwFlags = KEYEVENTF_KEYUP;
        inputs[3].Anonymous.ki.wVk = VK_CONTROL;
        inputs[3].Anonymous.ki.dwFlags = KEYEVENTF_KEYUP;
        if unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) } != inputs.len() as u32
        {
            return Err(SpeechPasteFailure::InputRejected);
        }
        Ok(())
    }
    #[cfg(not(target_os = "windows"))]
    Err(SpeechPasteFailure::TargetUnavailable)
}

/// Potentially blocking focus restoration happens outside the injection gate.
pub(crate) fn prepare_captured_target(target: SpeechPasteTarget) -> Result<(), SpeechPasteFailure> {
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, SetForegroundWindow};
        let hwnd = windows::Win32::Foundation::HWND(target.hwnd as *mut _);
        if target_identity(hwnd) != Some(target) {
            return Err(SpeechPasteFailure::TargetChanged);
        }
        if !unsafe { SetForegroundWindow(hwnd).as_bool() } {
            return Err(SpeechPasteFailure::FocusDenied);
        }
        if target_identity(unsafe { GetForegroundWindow() }) != Some(target) {
            return Err(SpeechPasteFailure::TargetChanged);
        }
        return Ok(());
    }
    #[cfg(not(target_os = "windows"))]
    Err(SpeechPasteFailure::TargetUnavailable)
}

/// Last immediate identity check and SendInput; caller holds shutdown delivery gate.
pub(crate) fn inject_to_prepared_target(
    target: SpeechPasteTarget,
) -> Result<(), SpeechPasteFailure> {
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::UI::Input::KeyboardAndMouse::{
            SendInput, INPUT, INPUT_KEYBOARD, KEYEVENTF_KEYUP, VIRTUAL_KEY, VK_CONTROL,
        };
        use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;

        let hwnd = windows::Win32::Foundation::HWND(target.hwnd as *mut _);
        if target_identity(hwnd) != Some(target) {
            return Err(SpeechPasteFailure::TargetChanged);
        }
        if target_identity(unsafe { GetForegroundWindow() }) != Some(target) {
            return Err(SpeechPasteFailure::TargetChanged);
        }
        let mut inputs: [INPUT; 4] = unsafe { std::mem::zeroed() };
        for input in &mut inputs {
            input.r#type = INPUT_KEYBOARD;
        }
        inputs[0].Anonymous.ki.wVk = VK_CONTROL;
        inputs[1].Anonymous.ki.wVk = VIRTUAL_KEY(0x56);
        inputs[2].Anonymous.ki.wVk = VIRTUAL_KEY(0x56);
        inputs[2].Anonymous.ki.dwFlags = KEYEVENTF_KEYUP;
        inputs[3].Anonymous.ki.wVk = VK_CONTROL;
        inputs[3].Anonymous.ki.dwFlags = KEYEVENTF_KEYUP;
        if unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) } != inputs.len() as u32
        {
            return Err(SpeechPasteFailure::InputRejected);
        }
        return Ok(());
    }
    #[cfg(not(target_os = "windows"))]
    Err(SpeechPasteFailure::TargetUnavailable)
}
