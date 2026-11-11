// Native launcher: cmd.exe cannot carry windows-sys' long feature arguments.
use std::{env, ffi::OsStr, path::Path, process::{self, Command}};

fn main() {
    let mut args: Vec<_> = env::args_os().skip(1).collect();
    let build_script = args.windows(2).any(|pair| {
        pair[0] == OsStr::new("--crate-name") && pair[1] == OsStr::new("build_script_build")
    });
    let blaze = args.first().and_then(|arg| Path::new(arg).file_name())
        .and_then(OsStr::to_str)
        .is_some_and(|name| name.eq_ignore_ascii_case("ra-jit-rustc.exe")
            || name.eq_ignore_ascii_case("ra-jit-rustc"));
    if build_script && blaze {
        args.remove(0);
    }
    let Some(command) = args.first() else {
        eprintln!("missing compiler command");
        process::exit(1);
    };
    match Command::new(command).args(&args[1..]).status() {
        Ok(status) => process::exit(status.code().unwrap_or(1)),
        Err(error) => {
            eprintln!("failed to launch compiler: {error}");
            process::exit(1);
        }
    }
}
