//! Where a built commit stands against `origin/main`, and the fetch that keeps
//! `origin/main` current. Read by Fleet's `get_fleet_build`.
//!
//! **Counts, not a verdict.** `origin/main...commit` gives both sides of the
//! symmetric difference at once: what the build holds that `origin/main` lacks
//! and what `origin/main` holds that the build lacks. A preview is ahead by its
//! merged branches and behind by whatever landed after it was cut.

use std::io::Read;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use crate::delivery::run_in;

const POLL: Duration = Duration::from_millis(50);

/// `(ahead, behind)` of `commit` against `origin/main`, or `None` where either
/// is not in the repository at `root` or git would not say.
pub fn position_against_main(root: &str, commit: &str) -> Option<(u32, u32)> {
    // `^{commit}` so a ref that is not a commit, and an empty string, are not counted.
    let named = format!("{commit}^{{commit}}");
    let held = run_in(root, "git", &["rev-parse", "--verify", "--quiet", &named]).ok()?;
    if !held.status.success() {
        return None;
    }
    let range = format!("origin/main...{commit}");
    let counted = run_in(root, "git", &["rev-list", "--left-right", "--count", &range]).ok()?;
    if !counted.status.success() {
        return None;
    }
    let said = String::from_utf8_lossy(&counted.stdout);
    let (behind, ahead) = said.trim().split_once('\t')?;
    Some((ahead.trim().parse().ok()?, behind.trim().parse().ok()?))
}

/// `git fetch origin main` in `root`, stopped if it is still running after
/// `within`. **No prompt**: Fleet has no terminal to answer one on.
pub fn fetch_main(root: &str, within: Duration) -> Result<(), String> {
    let mut child = Command::new("git")
        .args(["fetch", "--quiet", "origin", "main"])
        .current_dir(root)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .env("GIT_TERMINAL_PROMPT", "0")
        .spawn()
        .map_err(|why| why.to_string())?;
    let mut stderr = child.stderr.take().expect("stderr is piped");
    // Read on its own thread so a chatty git cannot fill the pipe and stall.
    let reading = std::thread::spawn(move || {
        let mut said = String::new();
        let _ = stderr.read_to_string(&mut said);
        said
    });
    let deadline = Instant::now() + within;
    loop {
        match child.try_wait() {
            Ok(Some(status)) if status.success() => return Ok(()),
            Ok(Some(_)) => return Err(reading.join().unwrap_or_default().trim().to_string()),
            Ok(None) if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("git fetch took longer than {}s", within.as_secs()));
            }
            Ok(None) => std::thread::sleep(POLL),
            Err(why) => return Err(why.to_string()),
        }
    }
}
