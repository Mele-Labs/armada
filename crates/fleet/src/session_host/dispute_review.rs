//! The job that settles a disputed Fleet notice: started in Armada's own repository on the `bug`
//! workflow, or joined where one is already open for the same notice.
//!
//! **The brief carries the thread.** No agent can read another job's thread, so the rows around
//! the notice go in it as text. **Who sent the notice is not recorded**: `told_by_fleet` has some
//! fifty callers and none names itself, so the review job is told to find the notice's fixed words
//! in the code instead.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::HostedSessions;
use config::settings as keys;
use ipc::{ManifestId, SessionId, SessionRow, SessionVoice, SetSessionWorkflow, StartSession};
use store::KeptDispute;

use super::disputing::{is_fleets, Settled};
use crate::daemon::Fleet;
use crate::retro::ARMADAS_OWN;

/// What the review job's title is asked for in; a test's model double answers on it.
pub(crate) const NAMING: &str = "Name what this notice is about";
/// A review job's title where the model named nothing usable.
const UNNAMED: &str = "Disputed notice";
/// The most words a title the model gave may have.
const TITLE_WORDS: usize = 6;
const REVIEW_WORKFLOW: &str = "bug";
/// Rows of the thread on each side of the notice.
const AROUND: usize = 10;
/// The most of one row's words the brief carries.
const ROW_CHARS: usize = 1_500;

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
    /// Join the open review of the same notice, or start one. **Never an error**: the dispute is
    /// kept whatever happens here, and what stopped a review is said on Fleet's stderr.
    pub(super) async fn review_settled(self: &Arc<Self>, dispute: &KeptDispute, rows: &[SessionRow]) -> Settled {
        let job = dispute.session_id.as_str();
        if !self.dispute_reviews_on() {
            eprintln!(
                "dispute {}: job {job} disputed a Fleet notice, and dispute reviews are off \
                 (`features.disputeReviews`), so no review job was started or joined. The dispute is kept",
                dispute.id
            );
            return Settled { review: None, joined: false };
        }
        let (title, voice) = self.disputing_voice(job).await;
        let thread = thread_around(rows, &dispute.notice_row);
        if let Some(open) = self.open_review(&dispute.key).await {
            let words = joined_words(dispute, &title, &thread);
            match self.first_message_from(&open, voice.clone(), &words).await {
                Ok(()) => return Settled { review: Some(open), joined: true },
                Err(why) => eprintln!("dispute {}: review job {open} was not sent it: {why:?}", dispute.id),
            }
        }
        let own = ManifestId::carried(ARMADAS_OWN);
        if self.served_named(Some(&own)).is_err() {
            eprintln!(
                "dispute {}: job {job} disputed a Fleet notice, and this Fleet does not serve Armada's own \
                 repository (`{ARMADAS_OWN}`), so no review job was started. The dispute is kept",
                dispute.id
            );
            return Settled { review: None, joined: false };
        }
        let named = self.review_named(dispute).await;
        let started = self
            .start_session(StartSession {
                manifest_id: own,
                title: Some(named),
                model: None,
                effort: None,
                mode: None,
                root: None,
                fork: None,
                request: None,
                after: None,
            })
            .await;
        let review = match started {
            Ok(record) => record.id.as_str().to_string(),
            Err(why) => {
                eprintln!("dispute {}: no review job was started: {why:?}", dispute.id);
                return Settled { review: None, joined: false };
            }
        };
        let set = SetSessionWorkflow { session_id: SessionId::carried(&review), workflow: Some(REVIEW_WORKFLOW.to_string()) };
        if let Err(why) = Arc::clone(self).set_session_workflow(set).await {
            eprintln!("dispute {}: review job {review} starts without workflow `{REVIEW_WORKFLOW}`: {why:?}", dispute.id);
        }
        // Kept once started, unsent or not: the job exists and a person can open it.
        if let Err(why) = self.first_message_from(&review, voice, &brief(dispute, &title, &thread)).await {
            eprintln!("dispute {}: review job {review} was not sent its brief: {why:?}", dispute.id);
        }
        Settled { review: Some(review), joined: false }
    }

    /// The review job's title: `Disputed:` and a few words the model names the notice by.
    async fn review_named(&self, dispute: &KeptDispute) -> String {
        let model = self.machine_settings().now().get(keys::MODELS_DISPUTE);
        match self.dispute_asked(&naming(&dispute.notice), &model).await {
            Ok(answer) => titled(&answer),
            Err(why) => {
                eprintln!("dispute {}: the review job was not named, so it is `{UNNAMED}`: {why}", dispute.id);
                UNNAMED.to_string()
            }
        }
    }

    /// The disputing job's title, and the voice its dispute is written to a review job in.
    async fn disputing_voice(&self, job: &str) -> (String, SessionVoice) {
        let kept = self.store().lock().await.session(job).ok().flatten();
        let title = kept.and_then(|one| one.title).unwrap_or_else(|| String::from("a job"));
        (title.clone(), SessionVoice::Session { id: job.to_string(), title })
    }

    /// The newest review of a notice matched by `key` that has not ended.
    async fn open_review(&self, key: &str) -> Option<String> {
        let store = self.store().lock().await;
        let reviews = store.dispute_reviews(key).ok()?;
        reviews.into_iter().find(|review| {
            store.session(review).ok().flatten().is_some_and(|one| one.state != store::SessionState::Ended)
        })
    }
}

/// How a dispute was spotted, in words.
fn spotted_words(spotted: &str) -> &'static str {
    match spotted {
        "read" => "Fleet read it off the job's replies; the job did not call `dispute_fleet`",
        _ => "the job called `dispute_fleet`",
    }
}

/// The longest stretch of the notice with no number in it, cut to a few words: what to grep for.
fn fixed_words(notice: &str) -> String {
    let longest = notice
        .split(|c: char| c.is_ascii_digit())
        .map(|part| part.trim_matches(|c: char| !c.is_alphanumeric()))
        .max_by_key(|part| part.len())
        .unwrap_or_default();
    longest.split_whitespace().take(8).collect::<Vec<_>>().join(" ")
}

fn quoted(text: &str) -> String {
    text.trim().lines().map(|line| format!("> {line}")).collect::<Vec<_>>().join("\n")
}

/// What the model is asked to name a review job by.
fn naming(notice: &str) -> String {
    format!(
        "A tool that supervises a coding agent sent it this notice:\n\n{notice}\n\n\
         {NAMING} in two to four plain words, as a title would say it: for example `conflict warnings`, \
         `behind main nudge` or `opus spend notice`. Answer with the words and nothing else.",
        notice = notice.trim(),
    )
}

/// The model's answer as a review job's title, or [`UNNAMED`] where it is not a few words on one line.
pub(crate) fn titled(answer: &str) -> String {
    let words = answer.trim().trim_matches(|c: char| matches!(c, '"' | '\'' | '`' | '.')).trim();
    let count = words.split_whitespace().count();
    if count == 0 || count > TITLE_WORDS || words.contains('\n') {
        return UNNAMED.to_string();
    }
    format!("Disputed: {words}")
}

/// The dispute as the review job is first told it.
fn brief(dispute: &KeptDispute, title: &str, thread: &str) -> String {
    format!(
        "A job disputed a notice Fleet sent it. Decide who was right, with evidence from Armada's code.\n\n\
         The notice, as Fleet sent it:\n{notice}\n\n\
         Where it came from: Fleet does not record which code sent a notice. Grep `crates/` for its fixed words, \
         for example `{fixed}`, and read the rule that wrote it and what it reads.\n\n\
         The job's reason ({spotted}):\n{why}\n\n\
         The job, \"{title}\" ({job}). Its thread around the notice, oldest first:\n\n{thread}\n\n\
         What to do:\n\
         - Fleet was wrong: fix it in Armada and land it on the bug workflow.\n\
         - The job was wrong: say why in one reply and end, changing nothing.",
        notice = quoted(&dispute.notice),
        fixed = fixed_words(&dispute.notice),
        spotted = spotted_words(&dispute.spotted),
        why = quoted(&dispute.why),
        job = dispute.session_id,
    )
}

/// A second dispute of the notice this review job settles, as it is sent to it.
fn joined_words(dispute: &KeptDispute, title: &str, thread: &str) -> String {
    format!(
        "Another dispute of the notice you are reviewing. Weigh it with the first.\n\n\
         The notice, as Fleet sent it this time:\n{notice}\n\n\
         The job's reason ({spotted}):\n{why}\n\n\
         The job, \"{title}\" ({job}). Its thread around the notice, oldest first:\n\n{thread}",
        notice = quoted(&dispute.notice),
        spotted = spotted_words(&dispute.spotted),
        why = quoted(&dispute.why),
        job = dispute.session_id,
    )
}

/// About twenty rows around the notice, one paragraph each, the notice marked.
pub(crate) fn thread_around(rows: &[SessionRow], notice_row: &str) -> String {
    let at = rows.iter().position(|row| row.id() == notice_row).unwrap_or(rows.len().saturating_sub(1));
    let from = at.saturating_sub(AROUND);
    let to = (at + AROUND).min(rows.len());
    rows[from..to]
        .iter()
        .map(|row| {
            let line = row_text(row);
            if row.id() == notice_row {
                format!("[the disputed notice] {line}")
            } else {
                line
            }
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn cut(text: &str) -> String {
    let text = text.trim();
    match text.char_indices().nth(ROW_CHARS) {
        Some((end, _)) => format!("{} [cut]", &text[..end]),
        None => text.to_string(),
    }
}

/// One row as words.
fn row_text(row: &SessionRow) -> String {
    match row {
        SessionRow::Message { from, text, .. } => {
            let who = match from {
                SessionVoice::You => String::from("Person"),
                SessionVoice::Agent => String::from("Agent"),
                voice if is_fleets(voice) => String::from("Fleet"),
                SessionVoice::Session { title, .. } => format!("Job \"{title}\""),
            };
            format!("{who}: {}", cut(text))
        }
        SessionRow::Tool { text, .. } => format!("Tool: {}", cut(text)),
        SessionRow::Command { text, .. } => format!("Command: {}", cut(text)),
        SessionRow::Ran { command, code, tail, .. } => {
            let ended = code.map_or_else(|| String::from("stopped"), |code| format!("exit {code}"));
            format!("Ran `{command}`, {ended}:\n{}", cut(tail))
        }
        SessionRow::Disputed { why, .. } => format!("Disputed a Fleet notice: {}", cut(why)),
        other => format!("({})", kind_of(other)),
    }
}

/// A row's kind, as the wire spells it.
fn kind_of(row: &SessionRow) -> String {
    ipc::encode(row)
        .ok()
        .and_then(|body| {
            let start = body.find("\"kind\":\"")? + 8;
            let end = body[start..].find('"')? + start;
            Some(body[start..end].to_string())
        })
        .unwrap_or_else(|| String::from("row"))
}
