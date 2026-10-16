//! At what priority a Check runs: agent work yields the machine to the merge
//! line and to the person using it. `docs/concepts/manifest.md`, *At what
//! priority a Check runs*.
//!
//! **A QoS clamp, not `nice`.** Measured on the owner's 18-core M5 Pro under a
//! saturating load: `nice -n 10` left the process at scheduler priority 31 and
//! changed nothing; `taskpolicy -b` held agent work to the slowest cores even
//! on an idle machine (1.2s became 2.2s); `taskpolicy -c utility` cost nothing
//! idle and gave a normal-priority Check back its idle time under load.

use std::ffi::OsStr;
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
    pub fn command(
        self,
        program: String,
        args: Vec<String>,
        worktree: &Path,
    ) -> (String, Vec<String>) {
        let path = std::env::var_os("PATH");
        self.command_searching(program, args, worktree, path.as_deref())
    }

    /// [`Priority::command`] for a child whose environment is not this
    /// process's: a bare name is looked up on the `PATH` the child will have.
    pub fn command_searching(
        self,
        program: String,
        args: Vec<String>,
        worktree: &Path,
        path: Option<&OsStr>,
    ) -> (String, Vec<String>) {
        let found = match self {
            Priority::Low if cfg!(target_os = "macos") && executable(Path::new(TASKPOLICY)) => {
                resolved(&program, worktree, path)
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

impl Priority {
    /// A `std::process::Command` for `program` at this priority, for the caller
    /// to add arguments, a directory and an environment to as it would to any
    /// other: **the clamp is the program, the arguments follow it**.
    ///
    /// For a spawn that has no worktree to resolve against, which is every
    /// background `git`, `gh` and `ps` Fleet runs for itself. A bare name is
    /// looked up on this process's `PATH`.
    pub fn spawning(self, program: &str) -> std::process::Command {
        let (spawned, leading) = self.command(program.to_string(), Vec::new(), Path::new("/"));
        let mut command = std::process::Command::new(spawned);
        command.args(leading);
        command
    }

    /// [`Priority::spawning`] for a spawn that is awaited.
    pub fn spawning_async(self, program: &str) -> tokio::process::Command {
        tokio::process::Command::from(self.spawning(program))
    }
}

/// `program` as the spawn would find it: against `worktree` where it names a
/// path, on `path` where it is a bare name.
fn resolved(program: &str, worktree: &Path, path: Option<&OsStr>) -> Option<PathBuf> {
    if program.contains('/') {
        let path = worktree.join(program);
        return executable(&path).then_some(path);
    }
    std::env::split_paths(path?)
        .filter(|dir| !dir.as_os_str().is_empty())
        .map(|dir| dir.join(program))
        .find(|candidate| executable(candidate))
}

fn executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path)
        .is_ok_and(|held| held.is_file() && held.permissions().mode() & 0o111 != 0)
}
