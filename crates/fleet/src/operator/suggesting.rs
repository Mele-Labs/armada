//! What a role at Suggest would do, waiting on one press.
//!
//! **Kept with what Decide would have acted on**, so a restart keeps it and `Do it` goes through
//! [`Operator::acted`] as On does. An answer's suggestion shows on the question it would settle, where
//! his press on the option is the one press; any other is a row of its own on the Job's Needs you
//! list. A suggestion with no Job, on a report nothing reported from a Job, on a note he left or on an error Bridge showed him, is
//! kept for the panel. **A note's is kept whatever its role stands at**, since a note is only ever proposed.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use core_model::Actor;
use ipc::operator_board::{AnswerOperatorSuggestion, OperatorBoard};
use ipc::{Instant, OperatorRead, SessionRecord, WaitingAct, WaitingActKind, WaitingItem, WaitingOption, WaitingSource, WireError};
use store::KeptSuggestion;

use super::questions::asked_item;
use super::{Arrival, Arrived, Logged, Outcome, Ruling, Source, Way};
use crate::daemon::Fleet;
use crate::session_host::waiting::SESSION_WAITING_EMPTY;

/// The code of a 404 on a press no suggestion answers to.
const SUGGESTION_UNKNOWN: &str = "fleet.operator_suggestion_unknown";

/// The id of the item for one suggestion: `suggestion:<n>`.
pub(super) const SUGGESTION: &str = "suggestion:";

/// Choice 0 does what Decide would have.
pub(crate) const DO_IT: u32 = 0;
/// Choice 1 leaves it.
pub(crate) const SKIP: u32 = 1;

/// The rows of a Job's Needs you list for what it would do. An answer's shows on its question.
pub(crate) fn suggestion_items(kept: &[KeptSuggestion]) -> impl Iterator<Item = WaitingItem> + '_ {
    kept.iter().filter(|one| one.item.is_none()).map(|one| {
        let option = |label: &str| WaitingOption { label: label.to_string(), description: None };
        let id = format!("{SUGGESTION}{}", one.n);
        WaitingItem {
            act: Some(WaitingAct { kind: WaitingActKind::Answer, target: id.clone() }),
            id,
            text: one.text.clone(),
            since: Instant::carried(&one.raised_at),
            source: WaitingSource::OperatorSuggestion,
            options: vec![option("Do it"), option("Skip")],
            steps: Vec::new(),
            place: None,
            confirmed: false,
            holders: Vec::new(),
            creating_since: None,
            create_refused: None,
            operator_read: None,
        }
    })
}

/// Each question the Operator would answer, with its read and its pick.
pub(crate) fn reads_shown(items: &mut [WaitingItem], kept: &[KeptSuggestion]) {
    for one in kept {
        let (Some(item), Some(pick)) = (&one.item, &one.pick) else { continue };
        if let Some(asked) = items.iter_mut().find(|asked| &asked.id == item) {
            asked.operator_read = Some(OperatorRead { read: one.because.clone(), pick: pick.clone() });
        }
    }
}

/// What the row says the Operator would do, naming the other Job by `other`, its title.
fn said(arrival: &Arrival, ruling: &Ruling, other: &str) -> String {
    let because = &ruling.because;
    let subject = &arrival.subject;
    match (arrival.arrived, ruling.way) {
        (Arrived::Report, Way::NewJob) => format!("The Operator would start a Job on the report \"{subject}\": {because}"),
        (Arrived::Report, _) => format!("The Operator would hand the report \"{subject}\" to \"{other}\": {because}"),
        (Arrived::Note, Way::File) => format!("The Operator would file your note \"{subject}\" as a new issue: {because}"),
        (Arrived::Note, Way::NewJob) => format!("The Operator would start a Job on your note \"{subject}\": {because}"),
        (Arrived::Note, Way::Link) => format!("The Operator would link your note \"{subject}\" to {}: {because}", ruling.issue.as_deref().unwrap_or("an open issue")),
        (Arrived::Note, _) => format!("The Operator would send your note \"{subject}\" to \"{other}\": {because}"),
        (Arrived::Ended | Arrived::Merged, Way::File) => format!("The Operator would file \"{}\" as a new issue now that {}: {because}", what_of(ruling), happened(arrival)),
        (Arrived::Ended | Arrived::Merged, _) => format!("The Operator would tell \"{other}\" that {}: {because}", happened(arrival)),
        (Arrived::Error, Way::File) => format!("The Operator would file the error \"{subject}\" as a new issue: {because}"),
        (Arrived::Error, Way::NewJob) => format!("The Operator would start a Job on the error \"{subject}\": {because}"),
        (Arrived::Error, Way::Link) => format!("The Operator would link the error \"{subject}\" to {}: {because}", ruling.issue.as_deref().unwrap_or("an open issue")),
        (Arrived::Error, _) => format!("The Operator would hand the error \"{subject}\" to \"{other}\": {because}"),
        (Arrived::Later, _) => format!("The Operator would file \"{subject}\" as a new issue: {because}"),
        (_, Way::Merge) => format!("The Operator would end this Job into \"{other}\": {because}"),
        (_, Way::KeepInTouch) => format!("The Operator would tell this Job and \"{other}\" of each other: {because}"),
        _ => format!("The Operator would hand this work to \"{other}\": {because}"),
    }
}

/// What a suggestion is about, so one raised again for it is a repeat: its repository, kind of arrival
/// and way, and its subject, trimmed, single-spaced and lower-cased. **A file on an end or a merge is
/// about its `what`**, the next piece of work, since its subject there is the pull request and each
/// merge is a new one naming the same work.
fn same_thing(one: &KeptSuggestion) -> [String; 4] {
    let ending_file = one.way == Way::File.word() && Arrived::from_word(&one.arrived).is_some_and(Arrived::is_ending);
    let about = match &one.what {
        Some(what) if ending_file && !what.trim().is_empty() => what,
        _ => &one.subject,
    };
    let said = about.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase();
    [one.manifest_id.clone(), one.arrived.clone(), one.way.clone(), said]
}

/// What a ruling on an end or a merge would start or file.
fn what_of(ruling: &Ruling) -> &str {
    ruling.what.as_deref().unwrap_or_default()
}

/// What an end or a merge is, as a row says it: `"<Job>" ended`, or `#<n> merged`.
fn happened(arrival: &Arrival) -> String {
    match arrival.arrived {
        Arrived::Merged => format!("#{} merged", arrival.id.rsplit(':').next().unwrap_or_default()),
        _ => format!("\"{}\" ended", arrival.subject),
    }
}

/// The arrival a suggestion was kept from, as far as doing it reads one.
fn arrival_of(one: &KeptSuggestion) -> Option<Arrival> {
    Some(Arrival {
        id: one.arrival.clone(),
        arrived: Arrived::from_word(&one.arrived)?,
        source: Source::Person,
        session_id: one.session_id.clone(),
        manifest_id: one.manifest_id.clone(),
        subject: one.subject.clone(),
        words: one.words.clone(),
        paths: Vec::new(),
        issues: Vec::new(),
        group: one.group.clone(),
        ways: Vec::new(),
        answers: Vec::new(),
        rule: None,
        open: Vec::new(),
        ties: Vec::new(),
    })
}

/// The ruling a suggestion was kept from, as far as doing it reads one.
fn ruling_of(one: &KeptSuggestion) -> Option<Ruling> {
    Some(Ruling {
        other: one.other_session.clone(),
        shared: one.shared.clone(),
        first: one.first_session.clone(),
        what: one.what.clone(),
        ..Ruling::alone(Way::from_word(&one.way)?, one.because.clone())
    })
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
    /// `ruling` kept for one press, and the Job's list drawn again. The board is told too, which lists
    /// the ones no Job holds.
    /// **Nothing is kept for a repeat** of one standing, skipped or done: `Some` says which, for its Log row.
    pub(super) async fn operator_suggested(&self, arrival: &Arrival, ruling: &Ruling) -> Result<Option<String>, String> {
        let raised_at = self.now().as_str().to_string();
        {
            let mut store = self.store().lock().await;
            let other = ruling.other.as_deref().unwrap_or_default();
            let title = store.session(other).ok().flatten().and_then(|one| one.title).unwrap_or_else(|| other.to_string());
            let kept = KeptSuggestion {
                n: 0,
                arrival: arrival.id.clone(),
                arrived: arrival.arrived.word().to_string(),
                way: ruling.way.word().to_string(),
                // An end's suggestion is kept for the panel: the Job it is about has ended. So is a
                // note's he sent to a Job, which is the note's and not that Job's to settle.
                session_id: if arrival.arrived.is_ending() || arrival.is_sent_note() { String::new() } else { arrival.session_id.clone() },
                manifest_id: arrival.manifest_id.clone(),
                subject: arrival.subject.clone(),
                words: arrival.words.clone(),
                group: arrival.group.clone(),
                other_session: ruling.other.clone(),
                shared: ruling.shared.clone(),
                first_session: ruling.first.clone(),
                step: None,
                item: asked_item(arrival).map(str::to_string),
                pick: ruling.answer.as_ref().map(|choice| choice.label.clone()),
                because: ruling.because.clone(),
                what: ruling.what.clone(),
                text: said(arrival, ruling, &title),
                raised_at,
            };
            let earlier = store.operator_suggestions_like(&kept).map_err(|_| "the suggestion was not kept".to_string())?;
            if let Some(first) = earlier.iter().find(|first| same_thing(first) == same_thing(&kept)) {
                return Ok(Some(format!("a repeat of the suggestion kept for {}", first.arrival)));
            }
            if store.keep_operator_suggestion(&kept).is_err() {
                return Err("the suggestion was not kept".to_string());
            }
        }
        self.operator_board_changed();
        let _ = self.published_hosted(&arrival.session_id).await;
        Ok(None)
    }

    /// Whether choice `choice` is Do it, or Skip. Anything else is refused.
    fn suggestion_go(&self, choice: Option<u32>) -> Result<bool, Refusal> {
        match choice {
            Some(DO_IT) => Ok(true),
            Some(SKIP) => Ok(false),
            _ => Err(self.hosted_refusal(SESSION_WAITING_EMPTY, "what the Operator would do is answered by choosing Do it or Skip")),
        }
    }

    /// `answer_waiting` on a suggestion's row. **Settled in the store first**, so a second press finds
    /// it settled: the first one wins and the other is a 409. Do it is the person's, so it goes
    /// whatever the role stands at by now, and the Log keeps it signed by him.
    pub(crate) async fn suggestion_chosen(self: &Arc<Self>, session: &str, item: &str, choice: Option<u32>) -> Result<SessionRecord, Refusal> {
        let go = self.suggestion_go(choice)?;
        let n = suggestion_number(item).ok_or_else(|| self.settled_already())?;
        self.suggestion_settled(session, n, go).await?;
        self.published_hosted(session).await
    }

    /// `answer_waiting` with words and no choice on a suggestion's row. **The words are the person's to
    /// the Operator**, talk about this suggestion's arrival, as `talk_to_operator` takes it; the row
    /// stays standing and the Operator's reply lands in the conversation.
    pub(crate) async fn suggestion_talked(self: &Arc<Self>, session: &str, item: &str, words: &str) -> Result<SessionRecord, Refusal> {
        let n = suggestion_number(item).ok_or_else(|| self.settled_already())?;
        let kept = self.store().lock().await.operator_suggestion(n).map_err(|why| self.ledger_fault(why))?.filter(|one| one.session_id == session);
        let kept = kept.ok_or_else(|| self.settled_already())?;
        self.operator_talked(words, Some(&kept.arrival)).await?;
        self.published_hosted(session).await
    }

    /// `answer_operator_suggestion`: one press on a suggestion no Job holds, which is the only kind it
    /// takes. **404** where `id` names none, a Job's own suggestion included; 409 where it is settled.
    pub(crate) async fn suggestion_answered(self: &Arc<Self>, press: AnswerOperatorSuggestion) -> Result<OperatorBoard, Refusal> {
        let go = self.suggestion_go(Some(press.choice))?;
        let unknown = || {
            Refusal::NotFound(WireError::raised(SUGGESTION_UNKNOWN, format!("{} is not a suggestion waiting on a press", press.id), self.run_id()))
        };
        let n = suggestion_number(&press.id).ok_or_else(unknown)?;
        let seen = self.store().lock().await.operator_suggestion_seen(n).map_err(|why| self.ledger_fault(why))?;
        if !seen.is_some_and(|(session, _)| session.is_empty()) {
            return Err(unknown());
        }
        self.suggestion_settled("", n, go).await?;
        self.operator_board_wire().await
    }

    /// Suggestion `n` of `session` (empty where it has no Job), settled first and then done or skipped.
    /// A suggestion that is not standing on that Job is settled already, as far as this press knows.
    async fn suggestion_settled(self: &Arc<Self>, session: &str, n: i64, go: bool) -> Result<(), Refusal> {
        let at = self.now();
        let kept = {
            let mut store = self.store().lock().await;
            // The row must be this Job's: an id from another Job's list settles nothing here.
            let kept = store.operator_suggestion(n).map_err(|why| self.ledger_fault(why))?.filter(|one| one.session_id == session).ok_or_else(|| self.settled_already())?;
            if !store.settle_operator_suggestion(n, go, at.as_str()).map_err(|why| self.ledger_fault(why))? {
                return Err(self.settled_already());
            }
            kept
        };
        if go {
            self.suggestion_done(&kept, at).await;
        } else {
            self.suggestion_skipped(&kept, at).await;
        }
        self.operator_board_changed();
        Ok(())
    }

    /// What `kept` would have done, done, with a row of the Log saying who pressed. **A note's is done
    /// here and waited for**, so the row can name the issue it was filed as or the Job it started. The
    /// row is kept once it is done, and where it could not be it says so.
    async fn suggestion_done(self: &Arc<Self>, kept: &KeptSuggestion, at: core_model::Timestamp) {
        let (Some(arrival), Some(ruling), Some(operator)) = (arrival_of(kept), ruling_of(kept), self.operator_seat().operator()) else { return };
        let mut row = pressed(&arrival, Outcome::Ruled(ruling.way), &ruling.because, at);
        row.other = ruling.other.clone();
        row.shared = ruling.shared.clone();
        row.first = ruling.first.clone();
        let done = if arrival.arrived.is_sourced() {
            self.operator_sourced(&arrival, &ruling).await.map(|made| {
                row.issue = made.issue;
                row.other = made.job.or(row.other.take());
            })
        } else {
            operator.acted(&arrival, &ruling).await
        };
        if let Err(why) = done {
            row.outcome = Outcome::PassedOn;
            row.because = format!("it could not be carried out: {why}");
        }
        let _ = self.store().lock().await.add_operator_ruling(&row.kept());
    }

    /// A Skip, in the Log as the person's.
    async fn suggestion_skipped(&self, kept: &KeptSuggestion, at: core_model::Timestamp) {
        let Some(arrival) = arrival_of(kept) else { return };
        let row = pressed(&arrival, Outcome::Skipped, &kept.because, at);
        let _ = self.store().lock().await.add_operator_ruling(&row.kept());
    }
}

/// The number in a suggestion's id, `suggestion:<n>`.
fn suggestion_number(id: &str) -> Option<i64> {
    id.strip_prefix(SUGGESTION).and_then(|n| n.parse().ok())
}

/// A row of the Log for the person's press on `arrival`'s suggestion.
pub(super) fn pressed(arrival: &Arrival, outcome: Outcome, because: &str, at: core_model::Timestamp) -> Logged {
    Logged {
        arrival: arrival.id.clone(),
        arrived: arrival.arrived,
        source: Source::Person,
        session_id: arrival.session_id.clone(),
        manifest_id: arrival.manifest_id.clone(),
        subject: arrival.subject.clone(),
        outcome,
        other: None,
        shared: Vec::new(),
        because: because.to_string(),
        call: None,
        by: Actor::Human,
        handed: at.clone(),
        at,
        place: None,
        first: None,
        refused: Vec::new(),
        answer: None,
        issue: None,
    }
}
