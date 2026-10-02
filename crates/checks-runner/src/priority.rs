//! At what priority a Check runs: agent work yields the machine to the merge
//! line and to the person using it. `docs/concepts/manifest.md`, *At what
//! priority a Check runs*.
//!
//! **A QoS clamp, not `nice`.** Measured on the owner's 18-core M5 Pro under a
//! saturating load: `nice -n 10` left the process at scheduler priority 31 and
//! changed nothing; `taskpolicy -b` held agent work to the slowest cores even
//! on an idle machine (1.2s became 2.2s); `taskpolicy -c utility` cost nothing
//! idle and gave a normal-priority Check back its idle time under load.

use std::path::{Path, PathBuf};

/// Set to `normal` to run a Check at normal priority: the merge line sets it on
/// every Check it runs, and a person sets it to switch the lowering off.
pub const PRIORITY_ENV: &str = "ARMADA_CHECK_PRIORITY";

/// Where the clamp comes from. A system tool rather than a call, because the
/// clamp's spawn attribute is not public API.
const TASKPOLICY: &str = "/usr/sbin/taskpolicy";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Priority {
    /// Agent work: clamped to utility QoS, inherited by everything it starts.
    Low,
    /// The merge line's Checks, and every Check where the lowering is off.
    Normal,
}

impl Priority {
    /// [`Priority::named`] for this process's own environment.
    pub fn from_env() -> Priority {
        Priority::named(std::env::var(PRIORITY_ENV).ok().as_deref())
    }

    /// Low unless the value is `normal`: the lowering is the default, and only
    /// the one word turns it off.
    pub fn named(value: Option<&str>) -> Priority {
        match value {
            Some(value) if value.trim().eq_ignore_ascii_case("normal") => Priority::Normal,
            _ => Priority::Low,
        }
    }

    /// The program and arguments to spawn for `program args` at this priority.
    ///
    /// **Only a program that resolves is wrapped**, and by its full path. One
    /// that does not is spawned as written, so the operating system still says
    /// *not found* rather than `taskpolicy` exiting 66 — a code a real program
    /// may return, which `run`'s header says why to keep apart.
    pub(crate) fn command(
        self,
        program: String,
        args: Vec<String>,
        worktree: &Path,
    ) -> (String, Vec<String>) {
        let found = match self {
            Priority::Low if cfg!(target_os = "macos") && executable(Path::new(TASKPOLICY)) => {
                resolved(&program, worktree)
            }
            _ => None,
        };
        match found {
            Some(path) => {
                let mut wrapped = vec![
                    "-c".to_string(),
                    "utility".to_string(),
                    path.display().to_string(),
                ];
                wrapped.extend(args);
                (TASKPOLICY.to_string(), wrapped)
            }
            None => (program, args),
        }
    }
}

/// `program` as the spawn would find it: against `worktree` where it names a
/// path, on `PATH` where it is a bare name.
fn resolved(program: &str, worktree: &Path) -> Option<PathBuf> {
    if program.contains('/') {
        let path = worktree.join(program);
        return executable(&path).then_some(path);
    }
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .filter(|dir| !dir.as_os_str().is_empty())
        .map(|dir| dir.join(program))
        .find(|candidate| executable(candidate))
}

fn executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path)
        .is_ok_and(|held| held.is_file() && held.permissions().mode() & 0o111 != 0)
}
