//! The three calls this line makes through `$ARMADA_LAND_ARMADA`, always a
//! subprocess and never `declared::covering`/`declared::execute` in-process.
//!
//! **A deliberate reversal, from an earlier design note.** Calling in-process
//! against the gate worktree's own `armada.yml` would be the more direct
//! reading for a real repository's own gate — but `scripts/test_land.py`'s
//! stub `armada` answers these three by reading `checks.json` and
//! `checks/<name>.sh` out of the *current directory*, faking a
//! Manifest-driven repository without one actually existing. An in-process
//! call would parse the test repository's real (absent) `armada.yml` and see
//! none of that. Keeping this a subprocess through the swappable
//! `$ARMADA_LAND_ARMADA` is what keeps the existing Python suite's fixtures
//! exercising this port unmodified — the stronger evidence, by the task's
//! own reading.

use std::path::Path;
use std::time::Duration;

use super::shell::{run, run_limited};
use super::stop::Stopped;
use crate::cli::CHANGED;
use crate::say::NARROWED_TO;

/// `$ARMADA_LAND_ARMADA covers`, fed the changed paths on stdin — the Checks
/// they hit, in the Manifest's own order.
pub fn covers(armada: &str, cwd: &Path, paths: &[String]) -> Result<Vec<String>, Stopped> {
    let mut stdin = paths.join("\n");
    stdin.push('\n');
    let ran = run(&[armada, "covers"], cwd, Some(&stdin), None)?;
    if !ran.success() {
        return Err(Stopped::stopped(format!(
            "`{armada} covers` refused: {}",
            ran.stderr().trim()
        )));
    }
    Ok(ran
        .stdout()
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect())
}

/// `$ARMADA_LAND_ARMADA run <name>` — one of `setup.requires`, run before a
/// Check.
pub fn run_command(armada: &str, cwd: &Path, name: &str, log: &Path) -> Result<(), Stopped> {
    let ran = run(&[armada, "run", name], cwd, None, Some(log))?;
    if !ran.success() {
        return Err(Stopped::stopped(format!(
            "`{armada} run {name}` failed preparing the gate; see {}",
            log.display()
        )));
    }
    Ok(())
}

/// `$ARMADA_LAND_ARMADA check <name>` — one Check the combination hits,
/// logged whole so a red turn's caller can point at it, and killed past
/// `limit`. With `changed`, `check <name> --changed` over those paths, which
/// narrows it where its `narrow` can.
pub fn check(
    armada: &str,
    cwd: &Path,
    name: &str,
    changed: Option<&[String]>,
    log: &Path,
    limit: Duration,
) -> Result<CheckRan, Stopped> {
    let limited = match changed {
        Some(paths) => {
            let mut stdin = paths.join("\n");
            stdin.push('\n');
            let argv = [armada, "check", name, CHANGED];
            run_limited(&argv, cwd, Some(&stdin), log, limit)?
        }
        None => run_limited(&[armada, "check", name], cwd, None, log, limit)?,
    };
    let output = limited.ran.stdout();
    Ok(CheckRan {
        passed: limited.ran.success() && !limited.timed_out,
        timed_out: limited.timed_out,
        narrowed: output
            .lines()
            .find_map(|line| line.strip_prefix(NARROWED_TO))
            .map(|to| to.trim().to_string()),
        output: limited.ran.combined(),
    })
}

pub struct CheckRan {
    pub passed: bool,
    pub timed_out: bool,
    /// What `--changed` narrowed it to, as `armada check` said it. `None` is
    /// a whole run.
    pub narrowed: Option<String>,
    pub output: String,
}
