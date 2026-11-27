//! The test binary started as the app, so the tests that play a walk or a shot check need no second
//! build of this crate. gpui's Mac platform must be made on the process's main thread, which a test
//! thread is not, so each such test starts this binary again with `AS_APP` set, and the hook below
//! runs `main` before libtest's own main takes the command line. `docs/practices/ci.md`, *The
//! native Bridge compiles once*.

use std::process::{Command, ExitCode};

mod shot_check;
mod walks;

/// Set on the copy a test starts: that copy is the app, given the app's own flags.
const AS_APP: &str = "BRIDGE_NATIVE_AS_APP";

// SAFETY: it runs before `main`, on the main thread, and only in a copy a test started with `AS_APP`.
// macOS hands std its arguments and environment there, and the process exits before libtest runs.
#[allow(unsafe_code)]
#[ctor::ctor]
unsafe fn run_as_app() {
    if std::env::var_os(AS_APP).is_none() {
        return;
    }
    let code = crate::main();
    std::io::Write::flush(&mut std::io::stdout()).ok();
    std::process::exit(if code == ExitCode::SUCCESS { 0 } else { 1 });
}

/// This binary, to be run as the app with the flags a test adds.
fn app() -> Command {
    let mut app = Command::new(std::env::current_exe().expect("the test binary's path"));
    app.env(AS_APP, "1");
    app
}
