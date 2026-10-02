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
/// `limit`.
pub fn check(
    armada: &str,
    cwd: &Path,
    name: &str,
    log: &Path,
    limit: Duration,
) -> Result<CheckRan, Stopped> {
    // The merge line's Checks keep normal priority; every other caller's are
    // lowered beneath them. `checks_runner::Priority`.
    let normal = [(checks_runner::PRIORITY_ENV, "normal")];
    let limited = run_limited(&[armada, "check", name], cwd, log, limit, &normal)?;
    Ok(CheckRan {
        passed: limited.ran.success() && !limited.timed_out,
        timed_out: limited.timed_out,
        output: limited.ran.combined(),
    })
}

pub struct CheckRan {
    pub passed: bool,
    pub timed_out: bool,
    pub output: String,
}

#[cfg(test)]
mod tests {
    use std::os::unix::fs::PermissionsExt as _;
    use std::time::Duration;

    use super::check;
    use crate::tests::TempDir;

    /// The merge line's Checks are what everyone else's yield to, so it says
    /// so explicitly rather than leaving it to whatever its caller inherited.
    #[test]
    fn the_merge_line_runs_its_checks_at_normal_priority() {
        let dir = TempDir::new();
        dir.write("armada", "#!/bin/sh\necho \"$ARMADA_CHECK_PRIORITY\"\n");
        let stub = dir.path().join("armada");
        std::fs::set_permissions(&stub, std::fs::Permissions::from_mode(0o755)).expect("chmod");
        let Ok(ran) = check(
            stub.to_str().expect("a UTF-8 path"),
            dir.path(),
            "test",
            &dir.path().join("check.log"),
            Duration::from_secs(30),
        ) else {
            panic!("the stub runs");
        };
        assert_eq!(ran.output.trim(), "normal");
    }
}
