use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PinnedTaskbarLauncher {
    pub id: String,
    pub name: String,
    pub shortcut_path: String,
    pub target_path: Option<String>,
    pub icon_data_url: String,
}

#[tauri::command]
pub fn list_pinned_taskbar_apps() -> Result<Vec<PinnedTaskbarLauncher>, String> {
    imp::list_pinned_taskbar_apps()
}

#[tauri::command]
pub fn launch_pinned_taskbar_app(shortcut_path: String) -> Result<(), String> {
    imp::launch_pinned_taskbar_app(shortcut_path)
}

pub fn run_pinned_taskbar_app_as_admin(shortcut_path: String) -> Result<(), String> {
    imp::run_pinned_taskbar_app_as_admin(shortcut_path)
}

pub fn open_pinned_shortcut_properties(shortcut_path: String) -> Result<(), String> {
    imp::open_pinned_shortcut_properties(shortcut_path)
}

pub fn reveal_pinned_shortcut(shortcut_path: String) -> Result<(), String> {
    imp::reveal_pinned_shortcut(shortcut_path)
}

pub fn reveal_pinned_shortcut_target(shortcut_path: String) -> Result<(), String> {
    imp::reveal_pinned_shortcut_target(shortcut_path)
}

pub fn handle_launch_pinned_taskbar_helper_args() -> Result<bool, String> {
    imp::handle_launch_pinned_taskbar_helper_args()
}

pub fn copy_pinned_shortcut_path(shortcut_path: String) -> Result<(), String> {
    imp::copy_pinned_shortcut_path(shortcut_path)
}

pub fn unpin_pinned_taskbar_app(shortcut_path: String) -> Result<(), String> {
    imp::unpin_pinned_taskbar_app(shortcut_path)
}

pub fn can_pin_task_window_to_taskbar(hwnd: &str) -> Result<bool, String> {
    imp::can_pin_task_window_to_taskbar(hwnd)
}

pub fn pin_task_window_to_taskbar(hwnd: String) -> Result<(), String> {
    imp::pin_task_window_to_taskbar(hwnd)
}

#[cfg(target_os = "windows")]
mod imp {
    use super::PinnedTaskbarLauncher;
    use base64::engine::general_purpose::STANDARD as BASE64;
    use base64::Engine;
    use png::{BitDepth, ColorType, Encoder};
    use std::ffi::OsStr;
    use std::fs;
    use std::mem::size_of;
    use std::os::windows::ffi::OsStrExt;
    use std::path::{Path, PathBuf};
    use std::thread;
    use windows::core::{Interface, PCWSTR};
    use windows::Win32::Foundation::HWND;
    use windows::Win32::Graphics::Gdi::{
        CreateCompatibleDC, DeleteDC, DeleteObject, GetDIBits, GetObjectW, BITMAP, BITMAPINFO,
        BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HBITMAP, HDC,
    };
    use windows::Win32::Storage::FileSystem::{FILE_FLAGS_AND_ATTRIBUTES, WIN32_FIND_DATAW};
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CoUninitialize, IPersistFile, CLSCTX_INPROC_SERVER,
        COINIT_APARTMENTTHREADED, STGM_READ,
    };
    use windows::Win32::UI::Shell::{
        ExtractIconExW, IShellLinkW, SHGetFileInfoW, ShellExecuteW, SHFILEINFOW, SHGFI_ICON,
        SHGFI_SMALLICON,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        DestroyIcon, GetIconInfo, HICON, ICONINFO, SW_SHOWNORMAL,
    };

    const SE_ERR_ACCESSDENIED: isize = 5;

    pub fn list_pinned_taskbar_apps() -> Result<Vec<PinnedTaskbarLauncher>, String> {
        run_in_sta(|| {
            let taskbar_dir = pinned_taskbar_dir()?;
            let mut entries = fs::read_dir(&taskbar_dir)
                .map_err(|error| format!("Failed to read pinned taskbar shortcuts: {error}"))?
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .filter(|path| has_lnk_extension(path))
                .collect::<Vec<_>>();

            entries.sort_by_cached_key(|path| {
                path.file_name()
                    .map(|name| name.to_string_lossy().to_lowercase())
                    .unwrap_or_default()
            });

            let mut launchers = Vec::new();

            for shortcut_path in entries {
                let icon_data_url = extract_icon_data_url(&shortcut_path)
                    .unwrap_or_else(|_| fallback_launcher_icon_data_url());

                launchers.push(PinnedTaskbarLauncher {
                    id: shortcut_path.to_string_lossy().into_owned(),
                    name: launcher_name(&shortcut_path),
                    shortcut_path: shortcut_path.to_string_lossy().into_owned(),
                    target_path: resolved_shortcut_target_path(&shortcut_path),
                    icon_data_url,
                });
            }

            Ok(launchers)
        })
    }

    pub fn launch_pinned_taskbar_app(shortcut_path: String) -> Result<(), String> {
        let shortcut_path = validate_shortcut_path(&shortcut_path)?;
        match shell_execute_shortcut(shortcut_path.clone(), None, "launch pinned shortcut") {
            Ok(SE_ERR_ACCESSDENIED) => launch_pinned_taskbar_app_as_admin(shortcut_path),
            Ok(_) => Ok(()),
            Err(error) => Err(error),
        }
    }

    pub fn run_pinned_taskbar_app_as_admin(shortcut_path: String) -> Result<(), String> {
        let shortcut_path = validate_shortcut_path(&shortcut_path)?;
        launch_pinned_taskbar_app_as_admin(shortcut_path)
    }

    fn launch_pinned_taskbar_app_as_admin(shortcut_path: PathBuf) -> Result<(), String> {
        match shell_execute_shortcut(shortcut_path, Some("runas"), "launch pinned shortcut as administrator") {
            Ok(_) => Ok(()),
            Err(error) => Err(error),
        }
    }

    pub fn open_pinned_shortcut_properties(shortcut_path: String) -> Result<(), String> {
        let shortcut_path = validate_shortcut_path(&shortcut_path)?;
        shell_execute_shortcut(
            shortcut_path,
            Some("properties"),
            "open pinned shortcut properties",
        )
        .map(|_| ())
    }

    pub fn reveal_pinned_shortcut(shortcut_path: String) -> Result<(), String> {
        let shortcut_path = validate_shortcut_path(&shortcut_path)?;
        reveal_path_in_explorer(&shortcut_path, "pinned shortcut")
    }

    pub fn reveal_pinned_shortcut_target(shortcut_path: String) -> Result<(), String> {
        let shortcut_path = validate_shortcut_path(&shortcut_path)?;
        run_in_sta({
            let shortcut_path = shortcut_path.clone();
            move || {
                let shell_link = load_shell_link(&shortcut_path)?;
                let target_path = resolved_shortcut_target(&shell_link)?.ok_or_else(|| {
                    format!(
                        "Pinned shortcut target is unavailable: {}",
                        shortcut_path.display()
                    )
                })?;
                if !target_path.exists() {
                    return Err(format!(
                        "Pinned shortcut target no longer exists: {}",
                        target_path.display()
                    ));
                }
                reveal_path_in_explorer(&target_path, "pinned shortcut target")
            }
        })
    }

    pub fn copy_pinned_shortcut_path(shortcut_path: String) -> Result<(), String> {
        let shortcut_path = validate_shortcut_path(&shortcut_path)?;
        copy_text_to_clipboard(&shortcut_path.to_string_lossy())
    }

    pub fn unpin_pinned_taskbar_app(shortcut_path: String) -> Result<(), String> {
        let shortcut_path = validate_shortcut_path(&shortcut_path)?;
        fs::remove_file(&shortcut_path).map_err(|error| {
            format!(
                "Failed to unpin taskbar shortcut {}: {error}",
                shortcut_path.display()
            )
        })
    }

    pub fn can_pin_task_window_to_taskbar(hwnd: &str) -> Result<bool, String> {
        let target_path = crate::task_windows::task_window_process_path(hwnd)?;
        Ok(!taskbar_target_already_pinned(&target_path)?)
    }

    pub fn pin_task_window_to_taskbar(hwnd: String) -> Result<(), String> {
        let target_path = crate::task_windows::task_window_process_path(&hwnd)?;
        if taskbar_target_already_pinned(&target_path)? {
            return Ok(());
        }
        create_taskbar_shortcut(&target_path)
    }

    fn taskbar_target_already_pinned(target_path: &Path) -> Result<bool, String> {
        let target_identity = normalized_path_identity(target_path);
        for entry in fs::read_dir(pinned_taskbar_dir()?)
            .map_err(|error| format!("Failed to read pinned taskbar shortcuts: {error}"))?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| has_lnk_extension(path))
        {
            let Some(pinned_target) = resolved_shortcut_target_path(&entry) else {
                continue;
            };
            if normalized_path_identity(Path::new(&pinned_target)) == target_identity {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn create_taskbar_shortcut(target_path: &Path) -> Result<(), String> {
        let taskbar_dir = pinned_taskbar_dir()?;
        fs::create_dir_all(&taskbar_dir)
            .map_err(|error| format!("Failed to create pinned taskbar directory: {error}"))?;
        let shortcut_path = next_available_shortcut_path(&taskbar_dir, target_path);
        run_in_sta({
            let target_path = target_path.to_path_buf();
            let shortcut_path = shortcut_path.clone();
            move || {
                let shell_link: IShellLinkW = unsafe {
                    CoCreateInstance(
                        &windows::Win32::UI::Shell::ShellLink,
                        None,
                        CLSCTX_INPROC_SERVER,
                    )
                }
                .map_err(|error| format!("Failed to create ShellLink COM object: {error}"))?;
                let target_wide = to_wide(&target_path);
                unsafe {
                    shell_link
                        .SetPath(PCWSTR(target_wide.as_ptr()))
                        .map_err(|error| {
                            format!(
                                "Failed to set taskbar shortcut target {}: {error}",
                                target_path.display()
                            )
                        })?;
                }
                let persist_file: IPersistFile = shell_link
                    .cast()
                    .map_err(|error| format!("Failed to bind ShellLink persistence: {error}"))?;
                let shortcut_wide = to_wide(&shortcut_path);
                unsafe {
                    persist_file
                        .Save(PCWSTR(shortcut_wide.as_ptr()), true)
                        .map_err(|error| {
                            format!(
                                "Failed to save taskbar shortcut {}: {error}",
                                shortcut_path.display()
                            )
                        })?;
                }
                Ok(())
            }
        })
    }

    fn next_available_shortcut_path(taskbar_dir: &Path, target_path: &Path) -> PathBuf {
        let base_name = target_path
            .file_stem()
            .and_then(OsStr::to_str)
            .map(sanitize_shortcut_name)
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| "Pinned App".to_string());
        let first = taskbar_dir.join(format!("{base_name}.lnk"));
        if !first.exists() {
            return first;
        }
        for index in 2..=99 {
            let candidate = taskbar_dir.join(format!("{base_name} ({index}).lnk"));
            if !candidate.exists() {
                return candidate;
            }
        }
        taskbar_dir.join(format!("{base_name} ({}).lnk", std::process::id()))
    }

    fn sanitize_shortcut_name(name: &str) -> String {
        name.chars()
            .map(|ch| match ch {
                '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
                _ => ch,
            })
            .collect::<String>()
            .trim()
            .to_string()
    }

    fn normalized_path_identity(path: &Path) -> String {
        fs::canonicalize(path)
            .unwrap_or_else(|_| path.to_path_buf())
            .to_string_lossy()
            .replace('/', "\\")
            .to_ascii_lowercase()
    }

    fn shell_execute_shortcut(
        shortcut_path: PathBuf,
        verb: Option<&str>,
        context: &str,
    ) -> Result<isize, String> {
        run_in_sta({
            let shortcut_path = shortcut_path.clone();
            let verb = verb.map(str::to_string);
            let context = context.to_string();
            move || {
                let shortcut_wide = to_wide(&shortcut_path);
                let verb_wide = verb.as_deref().map(|value| to_wide(OsStr::new(value)));
                let result = unsafe {
                    ShellExecuteW(
                        Some(HWND::default()),
                        verb_wide
                            .as_ref()
                            .map(|value| PCWSTR(value.as_ptr()))
                            .unwrap_or(PCWSTR::null()),
                        PCWSTR(shortcut_wide.as_ptr()),
                        None,
                        None,
                        SW_SHOWNORMAL,
                    )
                };
                let code = result.0 as isize;

                if code <= 32 {
                    if verb.is_none() && code == SE_ERR_ACCESSDENIED {
                        return Ok(code);
                    }

                    return Err(format!(
                        "ShellExecuteW failed to {context} {} with code {code}",
                        shortcut_path.display()
                    ));
                }

                Ok(code)
            }
        })
    }

    pub fn handle_launch_pinned_taskbar_helper_args() -> Result<bool, String> {
        Ok(false)
    }

    fn reveal_path_in_explorer(path: &Path, context: &str) -> Result<(), String> {
        std::process::Command::new("explorer.exe")
            .arg(format!("/select,{}", path.display()))
            .spawn()
            .map_err(|error| format!("Failed to reveal {context}: {error}"))?;

        Ok(())
    }

    fn copy_text_to_clipboard(text: &str) -> Result<(), String> {
        std::process::Command::new("powershell.exe")
            .args([
                "-NoProfile",
                "-Command",
                "Set-Clipboard -Value $env:JASONSHELL_CLIPBOARD_TEXT",
            ])
            .env("JASONSHELL_CLIPBOARD_TEXT", text)
            .spawn()
            .map_err(|error| format!("Failed to copy launcher path: {error}"))?;

        Ok(())
    }

    fn run_in_sta<T, F>(operation: F) -> Result<T, String>
    where
        T: Send + 'static,
        F: FnOnce() -> Result<T, String> + Send + 'static,
    {
        thread::spawn(move || {
            unsafe {
                CoInitializeEx(None, COINIT_APARTMENTTHREADED)
                    .ok()
                    .map_err(|error| format!("Failed to initialize COM apartment: {error}"))?;
            }

            let result = operation();

            unsafe {
                CoUninitialize();
            }

            result
        })
        .join()
        .map_err(|_| "Pinned taskbar operation panicked".to_string())?
    }

    fn pinned_taskbar_dir() -> Result<PathBuf, String> {
        let Some(appdata) = std::env::var_os("APPDATA") else {
            return Err("APPDATA is unavailable".to_string());
        };

        Ok(PathBuf::from(appdata).join(
            Path::new("Microsoft")
                .join("Internet Explorer")
                .join("Quick Launch")
                .join("User Pinned")
                .join("TaskBar"),
        ))
    }

    fn validate_shortcut_path(shortcut_path: &str) -> Result<PathBuf, String> {
        let requested_path = PathBuf::from(shortcut_path);

        if !has_lnk_extension(&requested_path) {
            return Err("Only pinned .lnk shortcuts may be launched".to_string());
        }

        let canonical_dir = fs::canonicalize(pinned_taskbar_dir()?)
            .map_err(|error| format!("Failed to resolve pinned taskbar directory: {error}"))?;
        let canonical_shortcut = fs::canonicalize(&requested_path)
            .map_err(|error| format!("Failed to resolve pinned shortcut path: {error}"))?;
        let Some(parent) = canonical_shortcut.parent() else {
            return Err("Pinned shortcut parent directory is unavailable".to_string());
        };

        if parent != canonical_dir {
            return Err("Pinned shortcut path is outside the taskbar pin directory".to_string());
        }

        Ok(canonical_shortcut)
    }

    fn launcher_name(shortcut_path: &Path) -> String {
        shortcut_path
            .file_stem()
            .and_then(OsStr::to_str)
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .unwrap_or("Pinned App")
            .to_string()
    }

    fn has_lnk_extension(path: &Path) -> bool {
        path.extension()
            .and_then(OsStr::to_str)
            .map(|extension| extension.eq_ignore_ascii_case("lnk"))
            .unwrap_or(false)
    }

    fn load_shell_link(shortcut_path: &Path) -> Result<IShellLinkW, String> {
        let shell_link: IShellLinkW = unsafe {
            CoCreateInstance(
                &windows::Win32::UI::Shell::ShellLink,
                None,
                CLSCTX_INPROC_SERVER,
            )
        }
        .map_err(|error| format!("Failed to create ShellLink COM object: {error}"))?;
        let persist_file: IPersistFile = shell_link
            .cast()
            .map_err(|error| format!("Failed to bind ShellLink persistence: {error}"))?;
        let shortcut_wide = to_wide(shortcut_path);

        unsafe {
            persist_file
                .Load(PCWSTR(shortcut_wide.as_ptr()), STGM_READ)
                .map_err(|error| {
                    format!(
                        "Failed to load shortcut {}: {error}",
                        shortcut_path.display()
                    )
                })?;
        }

        Ok(shell_link)
    }

    fn extract_icon_data_url(shortcut_path: &Path) -> Result<String, String> {
        let shell_link = load_shell_link(shortcut_path)?;

        if let Some((icon_path, icon_index)) = explicit_icon_location(&shell_link)? {
            if let Ok(icon_data_url) = extract_icon_at_location(&icon_path, icon_index) {
                return Ok(icon_data_url);
            }
        }

        if let Some(target_path) = resolved_shortcut_target(&shell_link)? {
            if let Ok(icon_data_url) = extract_file_icon_data_url(&target_path) {
                return Ok(icon_data_url);
            }
        }

        extract_file_icon_data_url(shortcut_path)
    }

    fn resolved_shortcut_target_path(shortcut_path: &Path) -> Option<String> {
        let shell_link = load_shell_link(shortcut_path).ok()?;
        let target = resolved_shortcut_target(&shell_link).ok()??;
        Some(target.to_string_lossy().into_owned())
    }

    fn explicit_icon_location(shell_link: &IShellLinkW) -> Result<Option<(PathBuf, i32)>, String> {
        let mut icon_path = vec![0_u16; 260];
        let mut icon_index = 0;
        unsafe {
            shell_link
                .GetIconLocation(&mut icon_path, &mut icon_index)
                .map_err(|error| format!("Failed to read shortcut icon location: {error}"))?;
        }

        let path = trim_wide_buffer(&icon_path);
        if path.is_empty() {
            return Ok(None);
        }

        Ok(Some((PathBuf::from(path), icon_index)))
    }

    fn resolved_shortcut_target(shell_link: &IShellLinkW) -> Result<Option<PathBuf>, String> {
        let mut target_path = vec![0_u16; 260];
        let mut find_data = WIN32_FIND_DATAW::default();
        unsafe {
            shell_link
                .GetPath(&mut target_path, &mut find_data, 0)
                .map_err(|error| format!("Failed to read shortcut target path: {error}"))?;
        }

        let path = trim_wide_buffer(&target_path);
        if path.is_empty() {
            return Ok(None);
        }

        Ok(Some(PathBuf::from(path)))
    }

    fn extract_file_icon_data_url(path: &Path) -> Result<String, String> {
        let path_wide = to_wide(path);
        let mut icon_info = SHFILEINFOW::default();
        let icon_result = unsafe {
            SHGetFileInfoW(
                PCWSTR(path_wide.as_ptr()),
                FILE_FLAGS_AND_ATTRIBUTES(0),
                Some(&mut icon_info),
                size_of::<SHFILEINFOW>() as u32,
                SHGFI_ICON | SHGFI_SMALLICON,
            )
        };

        if icon_result == 0 || icon_info.hIcon.0.is_null() {
            return Err(format!("Failed to extract icon for {}", path.display()));
        }

        let png_result = icon_to_png_bytes(icon_info.hIcon);

        unsafe {
            let _ = DestroyIcon(icon_info.hIcon);
        }

        Ok(format!(
            "data:image/png;base64,{}",
            BASE64.encode(png_result?)
        ))
    }

    fn fallback_launcher_icon_data_url() -> String {
        "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAFgwJ/lTMmZQAAAABJRU5ErkJggg=="
            .to_string()
    }

    fn extract_icon_at_location(path: &Path, icon_index: i32) -> Result<String, String> {
        let path_wide = to_wide(path);
        let mut small_icon = HICON::default();
        let count = unsafe {
            ExtractIconExW(
                PCWSTR(path_wide.as_ptr()),
                icon_index,
                None,
                Some(&mut small_icon),
                1,
            )
        };

        if count == 0 || small_icon.0.is_null() {
            return Err(format!(
                "Failed to extract icon index {icon_index} from {}",
                path.display()
            ));
        }

        let png_result = icon_to_png_bytes(small_icon);

        unsafe {
            let _ = DestroyIcon(small_icon);
        }

        Ok(format!(
            "data:image/png;base64,{}",
            BASE64.encode(png_result?)
        ))
    }

    fn icon_to_png_bytes(icon_handle: HICON) -> Result<Vec<u8>, String> {
        let (width, height, pixels) = icon_to_rgba(icon_handle)?;
        let mut png_bytes = Vec::new();
        let mut encoder = Encoder::new(&mut png_bytes, width, height);
        encoder.set_color(ColorType::Rgba);
        encoder.set_depth(BitDepth::Eight);

        let mut writer = encoder
            .write_header()
            .map_err(|error| format!("Failed to start PNG encoding: {error}"))?;
        writer
            .write_image_data(&pixels)
            .map_err(|error| format!("Failed to encode launcher icon: {error}"))?;

        drop(writer);

        Ok(png_bytes)
    }

    fn icon_to_rgba(icon_handle: HICON) -> Result<(u32, u32, Vec<u8>), String> {
        let mut icon = ICONINFO::default();

        unsafe {
            GetIconInfo(icon_handle, &mut icon)
                .map_err(|error| format!("Failed to read icon metadata: {error}"))?;
        }

        let conversion_result = (|| {
            if icon.hbmColor.0.is_null() {
                return Err("Launcher icon does not expose a color bitmap".to_string());
            }

            let mut bitmap = BITMAP::default();
            let object_size = unsafe {
                GetObjectW(
                    icon.hbmColor.into(),
                    size_of::<BITMAP>() as i32,
                    Some((&mut bitmap as *mut BITMAP).cast()),
                )
            };

            if object_size == 0 {
                return Err("Failed to inspect launcher icon bitmap".to_string());
            }

            let width = bitmap.bmWidth as i32;
            let height = bitmap.bmHeight as i32;

            if width <= 0 || height <= 0 {
                return Err("Launcher icon bitmap dimensions are invalid".to_string());
            }

            let mut pixels = vec![0_u8; (width * height * 4) as usize];
            let mut bitmap_info = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: width,
                    biHeight: -height,
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    ..Default::default()
                },
                ..Default::default()
            };
            let dc = unsafe { CreateCompatibleDC(Some(HDC::default())) };

            if dc.0.is_null() {
                return Err("Failed to create icon bitmap device context".to_string());
            }

            let scanlines = unsafe {
                GetDIBits(
                    dc,
                    icon.hbmColor,
                    0,
                    height as u32,
                    Some(pixels.as_mut_ptr().cast()),
                    &mut bitmap_info,
                    DIB_RGB_COLORS,
                )
            };

            unsafe {
                let _ = DeleteDC(dc);
            }

            if scanlines == 0 {
                return Err("Failed to read launcher icon pixels".to_string());
            }

            for pixel in pixels.chunks_exact_mut(4) {
                pixel.swap(0, 2);
            }

            Ok((width as u32, height as u32, pixels))
        })();

        unsafe {
            delete_bitmap(icon.hbmColor);
            delete_bitmap(icon.hbmMask);
        }

        conversion_result
    }

    unsafe fn delete_bitmap(bitmap: HBITMAP) {
        if !bitmap.0.is_null() {
            let _ = DeleteObject(bitmap.into());
        }
    }

    fn trim_wide_buffer(buffer: &[u16]) -> String {
        let end = buffer
            .iter()
            .position(|value| *value == 0)
            .unwrap_or(buffer.len());
        String::from_utf16_lossy(&buffer[..end]).trim().to_string()
    }

    fn to_wide(value: impl AsRef<OsStr>) -> Vec<u16> {
        value
            .as_ref()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect()
    }

    #[cfg(test)]
    mod tests {
        use super::{
            fallback_launcher_icon_data_url, has_lnk_extension, launcher_name,
            sanitize_shortcut_name, trim_wide_buffer,
        };
        use std::path::Path;

        #[test]
        fn detects_only_lnk_shortcuts() {
            assert!(has_lnk_extension(Path::new("Visual Studio Code.lnk")));
            assert!(has_lnk_extension(Path::new("VISUAL STUDIO CODE.LNK")));
            assert!(!has_lnk_extension(Path::new("Visual Studio Code.url")));
        }

        #[test]
        fn uses_file_stem_for_launcher_name() {
            assert_eq!(
                launcher_name(Path::new("C:\\Pins\\Visual Studio Code.lnk")),
                "Visual Studio Code"
            );
        }

        #[test]
        fn trims_wide_icon_buffers() {
            let buffer = [
                b'C' as u16,
                b':' as u16,
                b'\\' as u16,
                b'I' as u16,
                b'c' as u16,
                b'o' as u16,
                b'n' as u16,
                0,
                b'X' as u16,
            ];

            assert_eq!(trim_wide_buffer(&buffer), "C:\\Icon");
        }

        #[test]
        fn fallback_launcher_icon_is_image_data_url() {
            assert!(fallback_launcher_icon_data_url().starts_with("data:image/png;base64,"));
        }

        #[test]
        fn sanitizes_shortcut_names() {
            assert_eq!(sanitize_shortcut_name("Bad:Name*"), "Bad_Name_");
        }
    }
}

#[cfg(not(target_os = "windows"))]
mod imp {
    use super::PinnedTaskbarLauncher;

    pub fn list_pinned_taskbar_apps() -> Result<Vec<PinnedTaskbarLauncher>, String> {
        Ok(Vec::new())
    }

    pub fn launch_pinned_taskbar_app(_shortcut_path: String) -> Result<(), String> {
        Err("Pinned taskbar launchers are only supported on Windows".to_string())
    }

    pub fn run_pinned_taskbar_app_as_admin(_shortcut_path: String) -> Result<(), String> {
        Err("Pinned taskbar launchers are only supported on Windows".to_string())
    }

    pub fn open_pinned_shortcut_properties(_shortcut_path: String) -> Result<(), String> {
        Err("Pinned taskbar launcher properties are only supported on Windows".to_string())
    }

    pub fn reveal_pinned_shortcut(_shortcut_path: String) -> Result<(), String> {
        Err("Pinned taskbar launcher reveal is only supported on Windows".to_string())
    }

    pub fn reveal_pinned_shortcut_target(_shortcut_path: String) -> Result<(), String> {
        Err("Pinned taskbar launcher target reveal is only supported on Windows".to_string())
    }

    pub fn copy_pinned_shortcut_path(_shortcut_path: String) -> Result<(), String> {
        Err("Pinned taskbar launcher path copy is only supported on Windows".to_string())
    }

    pub fn unpin_pinned_taskbar_app(_shortcut_path: String) -> Result<(), String> {
        Err("Pinned taskbar launcher unpin is only supported on Windows".to_string())
    }

    pub fn can_pin_task_window_to_taskbar(_hwnd: &str) -> Result<bool, String> {
        Err("Taskbar pinning is only supported on Windows".to_string())
    }

    pub fn pin_task_window_to_taskbar(_hwnd: String) -> Result<(), String> {
        Err("Taskbar pinning is only supported on Windows".to_string())
    }
}
