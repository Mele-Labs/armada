//! A Fleet notice the job pushed back on in its replies and never called `dispute_fleet` for,
//! read once by a cheap model call and kept as a dispute spotted `read`.
//!
//! **Only a notice that asks something is read**, which `fleet_word` keeps in `fleet_notices` as
//! it is sent; news asks nothing. A notice is read once its `limits.disputeReadMinutes` have passed
//! and the job has replied, and **marked read before the call**, so neither a second pass nor a
//! restart reads it again. One the job disputed through the call is marked read there.

use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{AgentHarness, Ask, Delivery, Model, Vcs, WorkProduct};
use config::settings as keys;
use core_model::Timestamp;
use ipc::{DisputeSpotted, SessionRow, SessionVoice};
use store::KeptNotice;

use super::disputing::Disputing;
use crate::daemon::Fleet;
use crate::judging::watched;

/// How often the notices are looked over. The wait itself is the setting's.
pub(crate) const EVERY: Duration = Duration::from_secs(60);
/// The most of the job's replies after the notice that are read.
const REPLIES: usize = 5;

/// What the model said of the replies.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Read {
    PushedBack { why: String },
    Not,
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
    /// Read the replies to every notice whose wait is over, and keep a dispute where they pushed back.
    pub(crate) async fn notices_read(self: &Arc<Self>) {
        // Off, a kept notice stays unread, so switching back on reads it then.
        if !self.dispute_reviews_on() {
            return;
        }
        let settings = self.machine_settings().now();
        let minutes = u64::try_from(settings.get(keys::DISPUTE_READ_MINUTES)).unwrap_or(0);
        let model = settings.get(keys::MODELS_DISPUTE);
        let Ok(unread) = self.store().lock().await.unread_fleet_notices() else { return };
        let now = self.now();
        for notice in unread {
            if crate::clock::elapsed(&Timestamp::from_rfc3339(notice.at.clone()), &now) < Duration::from_secs(minutes * 60) {
                continue;
            }
            self.notice_read(&notice, &model).await;
        }
    }

    async fn notice_read(self: &Arc<Self>, notice: &KeptNotice, model: &str) {
        let session = notice.session_id.as_str();
        let disputed = self.store().lock().await.notice_disputed(session, &notice.row_id).unwrap_or(false);
        let ended = self
            .store()
            .lock()
            .await
            .session(session)
            .ok()
            .flatten()
            .is_none_or(|one| one.state == store::SessionState::Ended);
        let Ok(rows) = self.rows_of(session).await else { return };
        let replies = replies_after(&rows, &notice.row_id);
        // Silence is read again next pass, until the job replies or ends.
        if !disputed && replies.is_empty() && !ended {
            return;
        }
        let at = self.now().as_str().to_string();
        if !matches!(self.store().lock().await.fleet_notice_read(session, &notice.row_id, &at), Ok(true)) {
            return;
        }
        if disputed || replies.is_empty() {
            return;
        }
        match self.pushback_read(&notice.text, &replies, model).await {
            Ok(Read::PushedBack { why }) => {
                let disputing = Disputing {
                    session,
                    notice_row: &notice.row_id,
                    notice: &notice.text,
                    why: &why,
                    spotted: DisputeSpotted::Read,
                };
                if let Err(refused) = self.dispute_kept(&disputing, &rows).await {
                    eprintln!("job {session}: a dispute read off its replies was not kept: {refused:?}");
                }
            }
            Ok(Read::Not) => {}
            Err(why) => eprintln!("job {session}: its replies to a Fleet notice were not read: {why}"),
        }
    }

    /// One call, one answer. Whatever goes wrong is words for the log.
    async fn pushback_read(&self, notice: &str, replies: &[&str], model: &str) -> Result<Read, String> {
        let reply = self.dispute_asked(&question(notice, replies), model).await?;
        Ok(read_of(&reply, replies))
    }

    /// `question` put once to `models.dispute` through the Judge's client and the proposer's budget.
    pub(super) async fn dispute_asked(&self, question: &str, model: &str) -> Result<String, String> {
        let proposing = self.proposing().map_err(|why| format!("{why:?}"))?;
        let model = Model::named_at(model, None).unwrap_or_else(|_| proposing.model.clone());
        let ask = Ask::put(model, question, proposing.environment.clone())
            .map_err(|_| String::from("there was nothing to ask"))?;
        watched(proposing.client.as_ref(), &ask, proposing.budget, &|_| {}, std::future::pending::<()>())
            .await
            .map_err(|why| why.to_string())
    }
}

/// The agent's words in the first [`REPLIES`] of its messages after the notice.
fn replies_after<'a>(rows: &'a [SessionRow], notice_row: &str) -> Vec<&'a str> {
    let Some(at) = rows.iter().position(|row| row.id() == notice_row) else { return Vec::new() };
    rows[at + 1..]
        .iter()
        .filter_map(|row| match row {
            SessionRow::Message { from: SessionVoice::Agent, text, .. } => Some(text.as_str()),
            _ => None,
        })
        .take(REPLIES)
        .collect()
}

pub(crate) const PUSHBACK: &str = "pushback:";
const WHY: &str = "why:";

/// What the model is asked.
pub(crate) fn question(notice: &str, replies: &[&str]) -> String {
    let said: Vec<String> = replies.iter().enumerate().map(|(n, text)| format!("Reply {}:\n{}", n + 1, text.trim())).collect();
    format!(
        "A tool that supervises a coding agent sent it this notice:\n\n{notice}\n\n\
         These are the agent's next replies, in order:\n\n{replies}\n\n\
         Did the agent push back on the notice: say it was wrong, mistaken, did not apply to what it \
         was doing, or that it would not follow it because it was based on a wrong reading? Agreeing, \
         following it, or not mentioning it is not pushing back.\n\n\
         Answer in two lines and nothing else:\n\
         {PUSHBACK} yes or no\n\
         {WHY} the agent's reason in one sentence, in its own words, or nothing where the answer is no",
        notice = notice.trim(),
        replies = said.join("\n\n"),
    )
}

/// The model's answer, read. Anything but a `yes` is no pushback.
pub(crate) fn read_of(reply: &str, replies: &[&str]) -> Read {
    let field = |name: &str| {
        reply.lines().find_map(|line| {
            let line = line.trim();
            line.to_lowercase().starts_with(name).then(|| line[name.len()..].trim().to_string())
        })
    };
    if !field(PUSHBACK).is_some_and(|said| said.to_lowercase().starts_with("yes")) {
        return Read::Not;
    }
    let why = field(WHY).filter(|why| !why.is_empty()).unwrap_or_else(|| {
        let first = replies.first().map(|text| text.trim()).unwrap_or_default();
        format!("Read off the job's reply: {}", first.chars().take(300).collect::<String>())
    });
    Read::PushedBack { why }
}
