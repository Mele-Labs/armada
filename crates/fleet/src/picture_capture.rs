//! What takes a picture of a screen with no window on his screen, for the librarian's `pictures` part
//! (`feature_library::picture_taking`). **A seam so a test plants one**, as `judge` is.

use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// How long one picture may take.
const BUDGET: Duration = Duration::from_secs(60);

/// The size a page is drawn at.
const WIDTH: u32 = 1280;
const HEIGHT: u32 = 800;

/// What takes a picture with no window on his screen. Both are blocking.
pub trait Capture: Send + Sync {
    /// The page at `url`, as a PNG at `out`.
    fn page(&self, url: &str, out: &Path) -> Result<(), String>;
    /// The first step of the native walk `walk`, played by the native Bridge at `app`, as a PNG at `out`.
    fn walk(&self, app: &Path, walk: &str, out: &Path) -> Result<(), String>;
}

/// The shipped capture: Playwright's headless shell for a page, and the native Bridge's `--walk <name> --check --shots` for a walk.
pub struct Headless;

impl Capture for Headless {
    fn page(&self, url: &str, out: &Path) -> Result<(), String> {
        let shell = adapters::browser_shell::found().ok_or_else(|| format!("no headless browser on this Mac: {}", adapters::browser_shell::INSTALL_COMMAND))?;
        let profile = out.with_extension("profile");
        let mut command = Command::new(shell);
        command.args([
            "--headless".to_string(),
            "--hide-scrollbars".to_string(),
            "--no-first-run".to_string(),
            "--disable-gpu".to_string(),
            format!("--user-data-dir={}", profile.display()),
            format!("--window-size={WIDTH},{HEIGHT}"),
            "--virtual-time-budget=5000".to_string(),
            format!("--screenshot={}", out.display()),
            url.to_string(),
        ]);
        let ran = run_for(command);
        let _ = std::fs::remove_dir_all(&profile);
        ran
    }

    fn walk(&self, app: &Path, walk: &str, out: &Path) -> Result<(), String> {
        let shots = out.with_extension("shots");
        let mut command = Command::new(app);
        command.args(["--walk", walk, "--check", "--width", &WIDTH.to_string()]).arg("--shots").arg(&shots);
        let ran = run_for(command).and_then(|()| std::fs::rename(shots.join(format!("{walk}-01.png")), out).map_err(|why| format!("the walk left no picture: {why}")));
        let _ = std::fs::remove_dir_all(&shots);
        ran
    }
}

/// Run `command` to its end within [`BUDGET`], with nothing on its streams.
fn run_for(mut command: Command) -> Result<(), String> {
    let mut child = command.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn().map_err(|why| format!("it could not start: {why}"))?;
    let began = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) if status.success() => return Ok(()),
            Ok(Some(status)) => return Err(format!("it ended with {status}")),
            Ok(None) if began.elapsed() > BUDGET => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("it took longer than {} seconds", BUDGET.as_secs()));
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(100)),
            Err(why) => return Err(why.to_string()),
        }
    }
}
