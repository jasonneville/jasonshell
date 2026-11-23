//! Process creation flags shared by helper-process spawns.
//!
//! Release builds use the Windows GUI subsystem, so console-subsystem children
//! (powershell, sc.exe, shutdown.exe, task children, git) would each flash a
//! console window unless they are created with `CREATE_NO_WINDOW`. GUI launches
//! (explorer, editors) and the "Open terminal here" launch must NOT use this.

pub(crate) const CREATE_NO_WINDOW: u32 = 0x0800_0000;

pub(crate) fn no_console_window(command: &mut std::process::Command) -> &mut std::process::Command {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command
}
