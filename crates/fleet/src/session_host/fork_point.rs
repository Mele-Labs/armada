//! Where a fork at a message cuts: the transcript line a thread row stands for.
//! Since 23.79. `docs/concepts/session.md`, *A fork at a message*; the flag is
//! `docs/spikes/030-*`.
//!
//! **A terminal session's row ids are its transcript lines' `uuid`s**, so its
//! cut is the row id. A hosted session's rows are Fleet's own and carry none,
//! so the cut is found by laying its messages over the transcript's. Where they
//! do not lie over each other the fork is refused and nothing is guessed.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{SessionRow, SessionVoice};

use super::serving::NO_SUCH_SESSION;
use crate::daemon::Fleet;

/// No message of the thread has the id, or it has no line in the transcript. A 422.
pub(super) const FORK_POINT_UNWRITTEN: &str = "fleet.fork_point_unwritten";

/// A message, as far as the two threads can be told apart: whether the agent
/// said it, and the words.
struct Said<'a> {
    id: &'a str,
    agent: bool,
    text: &'a str,
}

fn messages(rows: &[SessionRow]) -> Vec<Said<'_>> {
    rows.iter()
        .filter_map(|row| match row {
            SessionRow::Message { id, from, text, .. } => Some(Said {
                id,
                agent: *from == SessionVoice::Agent,
                text,
            }),
            _ => None,
        })
        .collect()
}

/// The transcript line a terminal session's row `at` was drawn from.
fn line_of_row(transcript: &[SessionRow], at: &str) -> Option<String> {
    messages(transcript)
        .iter()
        .any(|one| one.id == at)
        .then(|| at.split('.').next().unwrap_or(at).to_string())
}

/// Whether the transcript's message is the hosted thread's. The agent's words
/// are the same words; a person's are there inside what Fleet added around
/// them (a mention line, a path to an attachment).
fn same(hosted: &Said<'_>, written: &Said<'_>) -> bool {
    let words = hosted.text.trim();
    hosted.agent == written.agent
        && !words.is_empty()
        && if hosted.agent {
            words == written.text.trim()
        } else {
            written.text.contains(words)
        }
}

/// The transcript line of hosted row `at`.
///
/// **Laid over from the end**, because a fork's transcript opens with the
/// conversation it copied and its thread does not, and the transcript has the
/// odd message the thread never showed (an agent standing down for a move). A
/// message after `at` that finds no partner is passed over; `at` itself must
/// have one.
fn line_of_hosted_row(thread: &[SessionRow], transcript: &[SessionRow], at: &str) -> Option<String> {
    let thread = messages(thread);
    let target = thread.iter().position(|one| one.id == at)?;
    let written = messages(transcript);
    let mut before = written.len();
    for (index, one) in thread.iter().enumerate().skip(target).rev() {
        let found = written[..before].iter().rposition(|line| same(one, line));
        match (found, index == target) {
            (Some(found), true) => return Some(written[found].id.split('.').next()?.to_string()),
            (Some(found), false) => before = found,
            (None, true) => return None,
            (None, false) => {}
        }
    }
    None
}

/// Which line a row stands for, given the thread and the transcript as they are.
pub(super) fn cut_in(hosted: bool, thread: &[SessionRow], transcript: &[SessionRow], at: &str) -> Option<String> {
    if hosted {
        line_of_hosted_row(thread, transcript, at)
    } else {
        line_of_row(transcript, at)
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
    /// Session `old` as a fork reads it: whether Fleet hosts it, its thread, and the transcript.
    async fn fork_reads(&self, old: &str) -> Result<(bool, Vec<SessionRow>, Vec<SessionRow>), Refusal> {
        let (known, hosted) = {
            let store = self.store().lock().await;
            let known = store.session(old).map_err(|why| self.ledger_fault(why))?;
            let hosted = store.hosting(old).map_err(|why| self.ledger_fault(why))?;
            (known, hosted.is_some())
        };
        if known.is_none() {
            return Err(self.hosted_refusal(NO_SUCH_SESSION, &format!("no job is named {old}")));
        }
        let thread = if hosted { self.rows_of(old).await? } else { Vec::new() };
        let threads = self.hosts().terminals().threads();
        let (home, id) = (self.host().home.clone(), old.to_string());
        let (_, transcript, _) = tokio::task::spawn_blocking(move || threads.thread(&home, &id))
            .await
            .map_err(|why| self.hosted_fault(super::rows::SESSION_THREAD_UNREADABLE, &why.to_string()))?;
        Ok((hosted, thread, transcript))
    }

    /// The transcript line session `old`'s row `at` was written as. **Read before anything is
    /// written**, so a refusal leaves no session behind.
    pub(super) async fn fork_cut(&self, old: &str, at: &str) -> Result<String, Refusal> {
        let (hosted, thread, transcript) = self.fork_reads(old).await?;
        cut_in(hosted, &thread, &transcript, at).ok_or_else(|| {
            self.hosted_refusal(
                FORK_POINT_UNWRITTEN,
                &format!("no message {at} is written in job {old}'s conversation"),
            )
        })
    }

    /// The row of session `old`'s thread a fork with no `at` ends at: its latest message of its own
    /// that the transcript holds. **The agent's, not the person's**, so the fork's first message
    /// follows an agent turn as it would in the old conversation.
    pub(crate) async fn latest_fork_point(&self, old: &str) -> Result<String, Refusal> {
        let (hosted, thread, transcript) = self.fork_reads(old).await?;
        thread
            .iter()
            .rev()
            .find_map(|row| match row {
                SessionRow::Message { id, from: SessionVoice::Agent, .. }
                    if cut_in(hosted, &thread, &transcript, id).is_some() =>
                {
                    Some(id.clone())
                }
                _ => None,
            })
            .ok_or_else(|| {
                self.hosted_refusal(
                    FORK_POINT_UNWRITTEN,
                    &format!("none of job {old}'s own messages is written in its conversation yet"),
                )
            })
    }
}
