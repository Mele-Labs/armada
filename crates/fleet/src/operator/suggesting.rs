//! What a role at Suggest would do, waiting on one press.
//!
//! **Kept with what On would have acted on**, so a restart keeps it and `Do it` goes through
//! [`Operator::acted`] as On does. An answer's suggestion shows on the question it would settle, where
//! his press on the option is the one press; any other is a row of its own on the Job's Needs you
//! list. A suggestion with no Job, on a report nothing reported from a Job or on a note he left, is
//! kept for the panel. **A note's is kept whatever its role stands at**, since a note is only ever proposed.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use core_model::Actor;
use ipc::{Instant, OperatorRead, SessionRecord, WaitingAct, WaitingActKind, WaitingItem, WaitingOption, WaitingSource};
use store::KeptSuggestion;

use super::questions::asked_item;
use super::{Arrival, Arrived, Logged, Outcome, Ruling, Source, Way};
use crate::daemon::Fleet;
use crate::session_host::waiting::SESSION_WAITING_EMPTY;

/// The id of the item for one suggestion: `suggestion:<n>`.
const SUGGESTION: &str = "suggestion:";

/// Choice 0 does what On would have.
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
        (Arrived::Later, _) => format!("The Operator would file \"{subject}\" as a new issue: {because}"),
        (_, Way::NewJob) => format!("The Operator would start what is left of this Job as a Job of its own: {because}"),
        (_, Way::Nudge) => format!("The Operator would tell this Job: {}", ruling.step.as_deref().unwrap_or(because)),
        (_, Way::Merge) => format!("The Operator would end this Job into \"{other}\": {because}"),
        (_, Way::KeepInTouch) => format!("The Operator would tell this Job and \"{other}\" of each other: {because}"),
        _ => format!("The Operator would hand this work to \"{other}\": {because}"),
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
    })
}

/// The ruling a suggestion was kept from, as far as doing it reads one.
fn ruling_of(one: &KeptSuggestion) -> Option<Ruling> {
    Some(Ruling {
        other: one.other_session.clone(),
        shared: one.shared.clone(),
        step: one.step.clone(),
        first: one.first_session.clone(),
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
    /// `ruling` kept for one press, and the Job's list drawn again.
    pub(super) async fn operator_suggested(&self, arrival: &Arrival, ruling: &Ruling) {
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
                session_id: arrival.session_id.clone(),
                manifest_id: arrival.manifest_id.clone(),
                subject: arrival.subject.clone(),
                words: arrival.words.clone(),
                group: arrival.group.clone(),
                other_session: ruling.other.clone(),
                shared: ruling.shared.clone(),
                first_session: ruling.first.clone(),
                step: ruling.step.clone(),
                item: asked_item(arrival).map(str::to_string),
                pick: ruling.answer.as_ref().map(|choice| choice.label.clone()),
                because: ruling.because.clone(),
                text: said(arrival, ruling, &title),
                raised_at,
            };
            if store.keep_operator_suggestion(&kept).is_err() {
                return;
            }
        }
        let _ = self.published_hosted(&arrival.session_id).await;
    }

    /// `answer_waiting` on a suggestion's row. **Settled in the store first**, so a second press finds
    /// it settled: the first one wins and the other is a 409. Do it is the person's, so it goes
    /// whatever the role stands at by now, and the Log keeps it signed by him.
    pub(crate) async fn suggestion_chosen(self: &Arc<Self>, session: &str, item: &str, choice: Option<u32>) -> Result<SessionRecord, Refusal> {
        let go = match choice {
            Some(DO_IT) => true,
            Some(SKIP) => false,
            _ => return Err(self.hosted_refusal(SESSION_WAITING_EMPTY, "what the Operator would do is answered by choosing Do it or Skip")),
        };
        let n: i64 = item.strip_prefix(SUGGESTION).and_then(|n| n.parse().ok()).ok_or_else(|| self.settled_already())?;
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
        }
        self.published_hosted(session).await
    }

    /// What `kept` would have done, done, with a row of the Log saying who pressed.
    async fn suggestion_done(self: &Arc<Self>, kept: &KeptSuggestion, at: core_model::Timestamp) {
        let (Some(arrival), Some(ruling), Some(operator)) = (arrival_of(kept), ruling_of(kept), self.operator_seat().operator()) else { return };
        let row = Logged {
            arrival: arrival.id.clone(),
            arrived: arrival.arrived,
            source: Source::Person,
            session_id: arrival.session_id.clone(),
            manifest_id: arrival.manifest_id.clone(),
            subject: arrival.subject.clone(),
            outcome: Outcome::Ruled(ruling.way),
            other: ruling.other.clone(),
            shared: ruling.shared.clone(),
            because: ruling.because.clone(),
            call: None,
            by: Actor::Human,
            handed: at.clone(),
            at,
            place: None,
            first: ruling.first.clone(),
            refused: Vec::new(),
            answer: None,
            issue: None,
        };
        let _ = self.store().lock().await.add_operator_ruling(&row.kept());
        operator.acted(&arrival, &ruling);
    }
}
