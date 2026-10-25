//! A run that ended: the Session that started it is told how, as Fleet tells it anything.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};

use super::plain;
use crate::daemon::Fleet;
use crate::session_host::waiting::TAKEN_OFF;
use crate::session_host::RunEnd;

/// Lines of the end of the output the Session is told.
const TOLD_LINES: usize = 40;

/// How a run ended, as the task that waited for it saw it.
pub(crate) struct Ended {
    pub session: String,
    pub command: String,
    /// `None` where the group was stopped or a signal ended it.
    pub code: Option<i32>,
    pub stopped: bool,
    /// What the run kept of its output, escapes intact.
    pub output: String,
}

/// `text` in a fence that no run of backticks inside it can close.
pub(crate) fn fenced(text: &str) -> String {
    let (mut longest, mut run) = (0, 0);
    for c in text.chars() {
        run = if c == '`' { run + 1 } else { 0 };
        longest = longest.max(run);
    }
    let fence = "`".repeat((longest + 1).max(3));
    format!("{fence}\n{text}\n{fence}")
}

/// The last lines the run printed, as the screen drew them.
fn tail_of(ended: &Ended) -> String {
    plain::last_lines(&ended.output, TOLD_LINES)
}

/// What the Session reads. `taken_off` is whether its waiting list lost an item to the exit.
pub(crate) fn told(ended: &Ended, taken_off: bool) -> String {
    let how = match (ended.code, ended.stopped) {
        (Some(code), _) => format!("Exit {code}."),
        (None, true) => "Stopped.".to_string(),
        (None, false) => "Ended by a signal.".to_string(),
    };
    let command = ended.command.trim();
    let mut text = if command.contains(['\n', '`']) {
        format!("Owner ran this in Bridge:\n{}\n{how}", fenced(command))
    } else {
        format!("Owner ran `{command}` in Bridge. {how}")
    };
    let tail = tail_of(ended);
    if tail.is_empty() {
        text.push_str(" No output.");
    } else {
        text.push_str(" Last lines:\n");
        text.push_str(&fenced(&tail));
    }
    if taken_off {
        text.push('\n');
        text.push_str(TAKEN_OFF);
    }
    text
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
    /// A clean exit takes the matching `run` item off the Session's list first, so Bridge has
    /// nothing to settle, and a clean sign-in signs in every Job. **A Session that has ended is not
    /// told**, which `told_by_fleet` answers.
    pub(crate) async fn session_run_ended(self: Arc<Self>, ended: Ended) {
        let taken_off =
            ended.code == Some(0) && self.run_item_settled(&ended.session, &ended.command).await;
        if ended.code == Some(0) && ended.command.trim() == adapters::SIGN_IN_COMMAND {
            self.signed_in_everywhere(&ended.session).await;
        }
        let ran = RunEnd {
            command: ended.command.clone(),
            code: ended.code,
            tail: tail_of(&ended),
        };
        let _ = self
            .told_of_a_run(&ended.session, &told(&ended, taken_off), ran)
            .await;
    }
}
