//! A terminal watch: no command, but the Job's terminal heard from the watch's start until a line
//! matches what it waits for. **Its reading is passed and finished on a match**, quoting the line;
//! a terminal that ends first, or is not open, is a failed reading.
//!
//! **A line is matched as the person's pane draws it** (`crate::session_running::plain`), so a
//! prompt's colours and a progress bar's redraws do not hide the words. A line still being written
//! is matched too, so a prompt that ends in no newline is heard.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::SessionRunEvent;
use core_model::ReadingState;
use regex::{Regex, RegexBuilder};
use store::{KeptWatchTaken, KeptWatchWhole};
use tokio::sync::broadcast::error::RecvError;

use crate::daemon::Fleet;
use crate::session_running::plain;

/// The most of a line still being written that is held, from its end.
const MOST_PENDING: usize = 16 * 1024;
/// The longest a matched line is quoted.
const QUOTED: usize = 300;
/// A pattern's compiled size at most, so one cannot hold Fleet's memory.
const PATTERN_SIZE: usize = 1 << 20;

/// What `text` matches: the words themselves, or with `regex` the pattern it spells.
pub(crate) fn wanted(text: &str, regex: bool) -> Result<Regex, String> {
    let pattern = if regex { text.to_string() } else { regex::escape(text) };
    RegexBuilder::new(&pattern)
        .size_limit(PATTERN_SIZE)
        .build()
        .map_err(|why| format!("`in_terminal` is not a pattern Fleet can read: {why}"))
}

/// `chunk` added to what is `pending`, and the first line in it that `wanted` matches, as drawn.
pub(crate) fn matched(pending: &mut String, chunk: &str, wanted: &Regex) -> Option<String> {
    pending.push_str(chunk);
    while let Some(at) = pending.find('\n') {
        let line: String = pending.drain(..=at).collect();
        let drawn = plain::last_lines(&line, 1);
        if wanted.is_match(&drawn) {
            return Some(drawn);
        }
    }
    if pending.len() > MOST_PENDING {
        let mut cut = pending.len() - MOST_PENDING;
        while !pending.is_char_boundary(cut) {
            cut += 1;
        }
        pending.drain(..cut);
    }
    let drawn = plain::last_lines(pending, 1);
    wanted.is_match(&drawn).then_some(drawn)
}

fn read(state: ReadingState, says: String) -> KeptWatchTaken {
    KeptWatchTaken::Read { state: state.as_wire().to_string(), finished: true, says, link: None, parts: Vec::new() }
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
    /// Hear `kept`'s Job's terminal until it prints what the watch waits for, it ends, or `stopped`.
    pub(crate) async fn watch_listened(self: &Arc<Self>, kept: KeptWatchWhole, mut stopped: tokio::sync::oneshot::Receiver<()>) {
        let id = kept.watch.id.clone();
        let Some(text) = kept.watch.in_terminal.clone() else { return };
        let wanted = match wanted(&text, kept.watch.regex) {
            Ok(wanted) => wanted,
            Err(why) => return self.watch_read(&id, KeptWatchTaken::Broken { why, output: None }).await,
        };
        let Some(mut heard) = self.session_runs().shell_heard(&kept.watch.session_id) else {
            let says = String::from("The Job's terminal is not open. type_in_terminal opens it.");
            return self.watch_read(&id, read(ReadingState::Failed, says)).await;
        };
        let mut pending = String::new();
        let taken = loop {
            tokio::select! {
                _ = &mut stopped => return,
                event = heard.recv() => match event {
                    Ok(SessionRunEvent::Output(chunk)) => {
                        if let Some(line) = matched(&mut pending, &chunk, &wanted) {
                            let line: String = line.trim().chars().take(QUOTED).collect();
                            break read(ReadingState::Passed, format!("The terminal printed \"{line}\"."));
                        }
                    }
                    // A listener that fell behind reads on, as a viewer does.
                    Err(RecvError::Lagged(_)) => {}
                    Ok(SessionRunEvent::Exited(_)) | Err(RecvError::Closed) => {
                        break read(ReadingState::Failed, format!("The Job's terminal ended before it printed {text}."));
                    }
                },
            }
        };
        self.watch_read(&id, taken).await;
    }
}
