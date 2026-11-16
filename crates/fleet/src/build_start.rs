//! Around the start of `scripts/restart-build`: the checkout cleared so its `main` can fast-forward,
//! and the wrapper watched until it takes the status file over.
//!
//! **A wrapper that exits before it writes a stage never took over**, so nothing else says it
//! stopped and Fleet's own `running` mark would stand until it ages out. Its output goes to
//! `restart-build.start.log` and its last line becomes the failure. A file, not a pipe, because
//! the wrapper outlives this Fleet.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use ipc::{CheckoutBlocked, ClearBlocked};
use tokio::process::Child;

/// The wrapper's own stdout and stderr, from its latest start.
pub(crate) const START_LOG: &str = "restart-build.start.log";
/// A `git pull` of `main` still running after this is stopped.
const PULL_WITHIN: Duration = Duration::from_secs(2 * 60);

/// What stops `main` at `root` fast-forwarding. Runs git, so call it off the async runtime.
pub(crate) fn blocked_in(root: &str) -> Option<CheckoutBlocked> {
    adapters::main_blocked(root).map(|blocked| CheckoutBlocked { behind: blocked.behind, files: blocked.files })
}

/// Move the blocking files out of the way as `how` says, then fast-forward `main`. **Nothing to
/// clear is no work**: a checkout off `main`, level with it, or with no file in the way is left for
/// the wrapper to report. Errors with git's line. Runs git, so call it off the async runtime.
pub(crate) fn cleared(root: &Path, how: ClearBlocked, now: SystemTime) -> Result<(), String> {
    let root = root.to_string_lossy();
    let Some(blocked) = adapters::main_blocked(&root) else {
        return Ok(());
    };
    match how {
        ClearBlocked::Stash => {
            let millis = i64::try_from(now.duration_since(UNIX_EPOCH).unwrap_or_default().as_millis()).unwrap_or(0);
            let at = crate::clock::rfc3339_utc(millis);
            // To the second: `2026-11-16T09:30:12Z`.
            let at = format!("{}Z", at.get(..19).unwrap_or(&at));
            adapters::stash_files(&root, &blocked.files, &format!("armada: before updating main {at}"))?;
        }
        ClearBlocked::Delete => adapters::delete_files(&root, &blocked.files)?,
    }
    adapters::pull_main(&root, PULL_WITHIN)
}

/// Wait on the wrapper in the background. Where it exits non-zero and `status` still holds `mark`,
/// Fleet's own line, write the failure the wrapper never did: `build`, the recorded commit, and the
/// last line it printed.
pub(crate) fn watch(mut child: Child, support: PathBuf, status: PathBuf, mark: String, build: &'static str) {
    drop(tokio::spawn(async move {
        let Ok(exit) = child.wait().await else {
            return;
        };
        if exit.success() || std::fs::read_to_string(&status).ok().as_deref() != Some(mark.as_str()) {
            return;
        }
        let said = std::fs::read_to_string(support.join(START_LOG)).unwrap_or_default();
        let line = said.lines().map(str::trim).rfind(|line| !line.is_empty()).map(str::to_string).unwrap_or_else(|| match exit.code() {
            Some(code) => format!("restart-build exited {code}"),
            None => String::from("restart-build was ended by a signal"),
        });
        let commit = crate::building::commit_in(&support).unwrap_or_default();
        let _ = std::fs::write(&status, format!("failed\n{build}\n{commit}\n{line}\n"));
    }));
}
