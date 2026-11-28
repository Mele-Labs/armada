//! The last lines a background command has written, read off its output file while it runs.
//!
//! **Kept nowhere.** The lines ride the session's activity row (`SessionActivity::tails`), in memory,
//! and go when the command ends, when the process is replaced or when the session ends. The path comes
//! from the call's result (`DroneEvent::OutputAt`) and the task from the harness's start line, in
//! either order; the tail starts once Fleet holds both. **Only a file the harness names for that
//! very task is read**: `…/tasks/<task>.output`, with no `..` in it.

use std::collections::{HashMap, HashSet};
use std::io::SeekFrom;
use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use tokio::io::{AsyncReadExt, AsyncSeekExt};

use crate::daemon::Fleet;

/// How often the file is read.
pub(crate) const TAIL_EVERY: Duration = Duration::from_secs(1);

/// The most lines served, and the longest one.
const LINES: usize = 3;
const LINE_AT_MOST: usize = 160;

/// How much of the file's end is read: enough for the last lines of a chatty command.
const WINDOW: u64 = 8 * 1024;

/// What the stream said of each background command, by the id of its call. In memory.
#[derive(Default)]
pub(crate) struct Tailing {
    files: HashMap<String, String>,
    tasks: HashMap<String, String>,
    live: HashSet<String>,
}

impl Tailing {
    /// The call's result said where the output goes.
    pub(crate) fn file_told(&mut self, call: &str, file: &str) {
        self.files.insert(call.to_string(), file.to_string());
    }

    /// The harness started the task for the call.
    pub(crate) fn task_told(&mut self, call: &str, task: &str) {
        self.tasks.insert(call.to_string(), task.to_string());
    }

    /// The file to tail for the call, once the path and the task are both held, and once only.
    pub(crate) fn begin(&mut self, call: &str) -> Option<String> {
        let task = self.tasks.get(call)?;
        let file = self.files.get(call)?;
        if !trusted(file, task) || !self.live.insert(call.to_string()) {
            return None;
        }
        Some(file.clone())
    }

    /// Whether the call's command is still tailed.
    pub(crate) fn is_live(&self, call: &str) -> bool {
        self.live.contains(call)
    }

    /// The task ended: the call it ran for, if one was tailed.
    pub(crate) fn ended(&mut self, task: &str) -> Option<String> {
        let call = self.tasks.iter().find(|(_, held)| held.as_str() == task).map(|(call, _)| call.clone())?;
        self.tasks.remove(&call);
        self.files.remove(&call);
        self.live.remove(&call).then_some(call)
    }
}

/// The path the harness gives a task's output: under a `tasks` folder, named for the task.
fn trusted(file: &str, task: &str) -> bool {
    file.starts_with('/') && !file.contains("..") && file.ends_with(&format!("/tasks/{task}.output"))
}

/// The newest lines of what a file's end holds: a carriage return starts a line over, as a progress
/// bar does; an escape sequence is dropped; an empty line is skipped. `whole` is false where the
/// bytes began part way into the file, so the first line may be cut and is left out.
pub(crate) fn last_lines(bytes: &[u8], whole: bool) -> Vec<String> {
    let text = String::from_utf8_lossy(bytes);
    let mut lines: Vec<&str> = text.split('\n').collect();
    if !whole && lines.len() > 1 {
        lines.remove(0);
    }
    let mut kept: Vec<String> = lines
        .into_iter()
        .filter_map(|line| {
            let shown = line.rsplit('\r').find(|part| !part.trim().is_empty()).unwrap_or("");
            let clean = without_escapes(shown);
            let clean = clean.trim_end();
            (!clean.trim().is_empty()).then(|| cut(clean))
        })
        .collect();
    let extra = kept.len().saturating_sub(LINES);
    kept.drain(..extra);
    kept
}

fn cut(line: &str) -> String {
    match line.chars().count() <= LINE_AT_MOST {
        true => line.to_string(),
        false => format!("{}…", line.chars().take(LINE_AT_MOST - 1).collect::<String>().trim_end()),
    }
}

fn without_escapes(line: &str) -> String {
    let mut clean = String::with_capacity(line.len());
    let mut chars = line.chars().peekable();
    while let Some(one) = chars.next() {
        match one {
            '\u{1b}' if chars.peek() == Some(&'[') => {
                chars.next();
                for next in chars.by_ref() {
                    if ('@'..='~').contains(&next) {
                        break;
                    }
                }
            }
            '\u{1b}' => {}
            other if other.is_control() && other != '\t' => {}
            other => clean.push(other),
        }
    }
    clean
}

/// The end of the file, as the lines it holds. Empty where it cannot be read yet.
async fn tail_of(file: &str) -> Vec<String> {
    let Ok(mut open) = tokio::fs::File::open(file).await else { return Vec::new() };
    let Ok(len) = open.metadata().await.map(|meta| meta.len()) else { return Vec::new() };
    let from = len.saturating_sub(WINDOW);
    if open.seek(SeekFrom::Start(from)).await.is_err() {
        return Vec::new();
    }
    let mut bytes = Vec::new();
    if open.take(WINDOW).read_to_end(&mut bytes).await.is_err() {
        return Vec::new();
    }
    last_lines(&bytes, from == 0)
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
    /// Start tailing the call's output once its path and its task are both held.
    pub(super) fn tail_wanted(self: &Arc<Self>, id: &str, call: &str) {
        let Some(file) = self.hosts().of(id).state().tailing.begin(call) else { return };
        let (fleet, id, call) = (Arc::clone(self), id.to_string(), call.to_string());
        tokio::spawn(async move {
            let mut last: Vec<String> = Vec::new();
            while fleet.hosts().of(&id).state().tailing.is_live(&call) {
                let lines = tail_of(&file).await;
                if lines != last {
                    let step = fleet.hosts().of(&id).state().activity.tail_set(&call, lines.clone(), fleet.hosts().activity_every());
                    fleet.publish_step(id.clone(), step);
                    last = lines;
                }
                tokio::time::sleep(TAIL_EVERY).await;
            }
        });
    }

    /// The task ended: its lines go from the row. Whether the row changed, which the caller publishes.
    pub(super) fn tail_over(&self, id: &str, task: &str) -> bool {
        let runtime = self.hosts().of(id);
        let call = runtime.state().tailing.ended(task);
        match call {
            Some(call) => runtime.state().activity.tail_gone(&call),
            None => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_newest_three_lines_of_a_chatty_command() {
        let got = last_lines(b"one\ntwo\nthree\nfour\nfive\n", true);
        assert_eq!(got, ["three", "four", "five"]);
    }

    #[test]
    fn a_progress_bar_shows_its_latest_state_and_an_escape_is_dropped() {
        let got = last_lines(b"10%\r50%\r\x1b[32m99%\x1b[0m\n", true);
        assert_eq!(got, ["99%"]);
    }

    #[test]
    fn a_window_that_began_part_way_drops_its_cut_first_line() {
        assert_eq!(last_lines(b"rest of a cut line\nwhole\n", false), ["whole"]);
        assert_eq!(last_lines(b"whole\n", true), ["whole"]);
    }

    #[test]
    fn a_long_line_is_cut_and_empty_ones_are_skipped() {
        let long = "x".repeat(400);
        let got = last_lines(format!("\n\n{long}\n\n").as_bytes(), true);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].chars().count(), LINE_AT_MOST);
        assert!(got[0].ends_with('…'));
    }

    #[test]
    fn only_the_tasks_own_output_file_is_read() {
        assert!(trusted("/private/tmp/claude-501/p/s/tasks/bhj.output", "bhj"));
        assert!(!trusted("/private/tmp/claude-501/p/s/tasks/other.output", "bhj"));
        assert!(!trusted("/private/tmp/../etc/tasks/bhj.output", "bhj"));
        assert!(!trusted("tasks/bhj.output", "bhj"));
    }

    #[test]
    fn a_tail_begins_once_whichever_of_the_path_and_the_task_came_first() {
        let file = "/tmp/x/tasks/t1.output";
        let mut tailing = Tailing::default();
        tailing.file_told("c1", file);
        assert_eq!(tailing.begin("c1"), None);
        tailing.task_told("c1", "t1");
        assert_eq!(tailing.begin("c1").as_deref(), Some(file));
        assert_eq!(tailing.begin("c1"), None, "once");
        assert_eq!(tailing.ended("t1").as_deref(), Some("c1"));
        assert!(!tailing.is_live("c1"));
    }
}
