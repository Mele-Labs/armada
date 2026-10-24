//! What this Mac has, told to every Job at start. The commands its Jobs' shells reported missing
//! are kept machine-wide, and the browser the Playwright tool drives is named beside them.
//!
//! **No agent's word**: a name is kept off a result on the stream (`adapters::session_runs`), and
//! dropped when it resolves on the `PATH` a Job is started with, so installing it clears it.

use std::path::Path;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use adapters::browser_shell;
use adapters::session_runs::{missing_commands, Record};

use crate::daemon::Fleet;

/// What to use where a missing command has a stand-in.
const SUBSTITUTES: &[(&str, &str)] = &[
    ("timeout", "the Bash tool's `timeout` parameter"),
    ("gtimeout", "the Bash tool's `timeout` parameter"),
];

/// Whether `name` is an executable file in a folder of `path`.
pub(crate) fn on_path(name: &str, path: &str) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::env::split_paths(path).any(|folder| {
        std::fs::metadata(folder.join(name)).is_ok_and(|meta| meta.is_file() && meta.permissions().mode() & 0o111 != 0)
    })
}

/// A missing command as a line, with its stand-in where it has one.
pub(crate) fn missing_line(name: &str) -> String {
    match SUBSTITUTES.iter().find(|(held, _)| *held == name) {
        Some((_, instead)) => format!("`{name}` is not installed here. Use {instead}."),
        None => format!("`{name}` is not installed here."),
    }
}

/// The browser the Playwright tool drives, as a line.
pub(crate) fn browser_line(shell: Option<&Path>) -> String {
    match shell {
        Some(shell) => format!("The Playwright tool drives the headless shell at `{}`.", shell.display()),
        None => format!("The Playwright tool has no browser here. `{}` installs one.", browser_shell::INSTALL_COMMAND),
    }
}

impl<H, V, W> Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// Keep the commands a stream's results name as not found. Quiet on a failure: the next result
    /// that names one tries again.
    pub(crate) async fn missing_kept(&self, records: &[Record]) {
        let found = missing_commands(records);
        if !found.is_empty() {
            let _ = self.store().lock().await.merge_missing_commands(&found);
        }
    }

    /// The commands kept as missing that still do not resolve. One that does is forgotten.
    pub(crate) async fn missing_now(&self) -> Vec<String> {
        let mut store = self.store().lock().await;
        let mut missing = store.missing_commands().unwrap_or_default();
        missing.retain(|name| {
            let resolves = on_path(name, &self.host().path);
            if resolves {
                let _ = store.forget_missing_command(name);
            }
            !resolves
        });
        missing
    }
}
