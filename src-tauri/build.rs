fn main() {
    tauri_build::build();
    // Tauri embeds the application manifest for its main binary, not Cargo examples.
    #[cfg(target_os = "windows")]
    {
        println!("cargo:rustc-link-arg-examples=/MANIFEST:EMBED");
        println!("cargo:rustc-link-arg-examples=/MANIFESTDEPENDENCY:type='win32' name='Microsoft.Windows.Common-Controls' version='6.0.0.0' processorArchitecture='*' publicKeyToken='6595b64144ccf1df' language='*'");
    }
}
