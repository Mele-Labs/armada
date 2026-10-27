//! A failed or broken reading's whole output, kept as a file so the person can send it on.
//!
//! **Under Fleet's state, apart from the Job's scratch folder**, which the agent may write in: a log
//! is what the command printed, and nothing but Fleet writes it. One folder a watch, one file a
//! reading, and a reading the store forgets takes its file with it. Deleting the watch, or the Job
//! ending, removes the folder.
//!
//! **As much as the run captured**: `checks_runner` keeps the last [`CAPTURE_LIMIT`] of each
//! stream, and the file's first line says so when either was longer.

use std::path::{Path, PathBuf};

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use checks_runner::{Attempt, CAPTURE_LIMIT};
use store::KeptWatchTaken;

use crate::daemon::Fleet;

/// The folder every Job's watch logs are under, beside its scratch folder.
const LOGS_DIR: &str = "watch-logs";

/// What a run printed, both streams, each under its name. `None` where it printed nothing.
pub(crate) fn log_text(attempt: &Attempt) -> Option<String> {
    let (stdout, stderr) = (attempt.output.stdout.trim_end(), attempt.output.stderr.trim_end());
    if stdout.trim().is_empty() && stderr.trim().is_empty() {
        return None;
    }
    let mut text = String::new();
    if attempt.output.truncated {
        text.push_str(&format!("Cut: Fleet keeps the last {} KiB of each stream.\n\n", CAPTURE_LIMIT / 1024));
    }
    text.push_str(&format!("stdout:\n{stdout}\n\nstderr:\n{stderr}\n"));
    Some(text)
}

/// Whether `taken` failed: it broke, or it read `failed` overall or in a part.
pub(crate) fn failed(taken: &KeptWatchTaken) -> bool {
    match taken {
        KeptWatchTaken::Broken { .. } => true,
        KeptWatchTaken::Read { state, parts, .. } => state == "failed" || parts.iter().any(|(_, state)| state == "failed"),
    }
}

/// `at` as a file name: no colon, which Finder shows as a slash.
fn file_name(at: &str) -> String {
    format!("{}.log", at.replace(':', "-"))
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
    fn watch_logs_of(&self, job: &str) -> PathBuf {
        Path::new(&self.host().attachments_dir).join(LOGS_DIR).join(job)
    }

    /// Write `text` as the log of `watch`'s reading at `at`, and say where. `None` where it could not
    /// be written, so the reading carries no path to a file that is not there.
    pub(crate) fn watch_log_kept(&self, job: &str, watch: &str, at: &str, text: &str) -> Option<String> {
        let folder = self.watch_logs_of(job).join(watch);
        std::fs::create_dir_all(&folder).ok()?;
        let file = folder.join(file_name(at));
        std::fs::write(&file, text).ok()?;
        Some(file.to_string_lossy().into_owned())
    }

    /// Remove `watch`'s log files that no kept reading names.
    pub(crate) fn watch_logs_pruned(&self, job: &str, watch: &str, kept: &[&str]) {
        let Ok(files) = std::fs::read_dir(self.watch_logs_of(job).join(watch)) else { return };
        for file in files.flatten() {
            let path = file.path();
            if !kept.iter().any(|one| Path::new(one) == path) {
                let _ = std::fs::remove_file(path);
            }
        }
    }

    /// Remove every log `watch` kept.
    pub(crate) fn watch_log_removed(&self, job: &str, watch: &str) {
        let _ = std::fs::remove_dir_all(self.watch_logs_of(job).join(watch));
    }

    /// Remove every log `job`'s watches kept: the Job has ended.
    pub(crate) fn watch_logs_removed(&self, job: &str) {
        let _ = std::fs::remove_dir_all(self.watch_logs_of(job));
    }
}
