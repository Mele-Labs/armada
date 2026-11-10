//! The Operator's journal of the week: one markdown file per ISO week in Fleet's own home, a heading
//! per local day and a line per entry with its local time. **Fleet writes it, and every reading call
//! is handed this week's and last week's**, so the Operator wakes knowing what the week held.
//!
//! **Append only, and it never fails what it records**: a raise, a ruling or a message goes on
//! whether or not its line was written, and a line that would not write is a warning in Fleet's log.

use std::io::Write as _;
use std::path::PathBuf;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};

use crate::clock::LocalReading;
use crate::daemon::Fleet;

pub(super) const WEEKDAYS: [&str; 7] = ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"];
const WEEK: i64 = 7 * 86_400_000;

/// The file of the week `at` falls in, as `2026-W46.md`.
pub(crate) fn week_file(at: &LocalReading) -> String {
    format!("{}-W{:02}.md", at.week_year, at.week)
}

/// The heading of the day `at` falls in.
fn day_heading(at: &LocalReading) -> String {
    format!("## {} {:04}-{:02}-{:02}", WEEKDAYS[usize::try_from(at.weekday - 1).unwrap_or(0).min(6)], at.year, at.month, at.day)
}

/// `entry` on one line, as the journal keeps it.
fn one_line(entry: &str) -> String {
    entry.split_whitespace().collect::<Vec<_>>().join(" ")
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
    pub(super) fn journal_dir(&self) -> PathBuf {
        PathBuf::from(&self.host().operator_journal_dir)
    }

    /// Append `entry` to this week's journal, under today's heading. **Never fails**: a line that
    /// will not write is warned of and the rest goes on.
    pub(crate) async fn operator_journaled(&self, entry: &str) {
        let entry = one_line(entry);
        if entry.is_empty() {
            return;
        }
        let at = crate::clock::local_reading(self.now().epoch_millis().unwrap_or(0)).await;
        let path = self.journal_dir().join(week_file(&at));
        // One writer at a time, so two entries never both begin a day.
        let _turn = self.operator_seat().journal.lock().await;
        let written = (|| -> std::io::Result<()> {
            std::fs::create_dir_all(self.journal_dir())?;
            let before = std::fs::read_to_string(&path).unwrap_or_default();
            let heading = day_heading(&at);
            let mut out = String::new();
            if before.is_empty() {
                out.push_str(&format!("# The Operator's journal, {}-W{:02}\n", at.week_year, at.week));
            }
            if !before.lines().any(|line| line == heading) {
                out.push_str(&format!("\n{heading}\n\n"));
            }
            out.push_str(&format!("- {:02}:{:02} {entry}\n", at.hour, at.minute));
            std::fs::OpenOptions::new().create(true).append(true).open(&path)?.write_all(out.as_bytes())
        })();
        if let Err(why) = written {
            self.operator_warned("a line of the Operator's journal was not written", why);
        }
    }

    /// This week's journal and last week's, each as a file name and its text, where written.
    pub(crate) async fn operator_journal_files(&self) -> Vec<(String, String)> {
        let now = self.now().epoch_millis().unwrap_or(0);
        let mut files = Vec::new();
        for at in [now - WEEK, now] {
            let name = week_file(&crate::clock::local_reading(at).await);
            if let Ok(text) = std::fs::read_to_string(self.journal_dir().join(&name)) {
                files.push((format!("journal-{name}"), text));
            }
        }
        files
    }
}
