//! The one question a bug or feature Job is asked at its start: how the work leaves the branch.
//! `docs/concepts/session/the-delivery-question.md`.
//!
//! **Kept as a `delivery` row in the Job's thread**, so the question survives a restart and the
//! answer is read from the same place. The item is derived from the row on every read while it has
//! no choice. Auto land is the one answer Fleet acts on, by turning the Session's flag on; the rest
//! is the agent's, which reads it with [`delivery_of`] at a later step, after any walk.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{
    DeliveryChoice, Instant, SessionId, SessionRecord, SessionRow, SessionVoice, SetSessionAutoLand, WaitingAct, WaitingActKind, WaitingItem,
    WaitingOption, WaitingSource,
};

use crate::daemon::Fleet;
use crate::rail::{self, RailStep};

/// The item's id, and the target of its `answer` act.
pub(crate) const DELIVERY_ITEM: &str = "delivery";

/// What the owner is asked. Choices 0, 1 and 2 are [`DeliveryChoice::OFFERED`] in order.
pub(crate) fn delivery_item(since: Instant) -> WaitingItem {
    WaitingItem {
        id: DELIVERY_ITEM.to_string(),
        text: String::from("How should this work be delivered?"),
        since,
        source: WaitingSource::Delivery,
        act: Some(WaitingAct { kind: WaitingActKind::Answer, target: DELIVERY_ITEM.to_string() }),
        options: DeliveryChoice::OFFERED
            .iter()
            .map(|one| WaitingOption { label: one.label().to_string(), description: Some(one.detail().to_string()) })
            .collect(),
        steps: Vec::new(),
        place: None,
        concern: None,
        cites: Vec::new(),
        confirmed: false,
    }
}

/// The answer kept on the Job: the choice of its `delivery` row, if it has one and it is answered.
// The agent is told the answer in words, so only the tests read it so far.
#[allow(dead_code)]
pub(crate) fn delivery_of(rows: &[SessionRow]) -> Option<DeliveryChoice> {
    rows.iter().find_map(|row| match row {
        SessionRow::Delivery { choice, .. } => *choice,
        _ => None,
    })
}

/// Whether the thread already has a `delivery` row, asked or answered.
pub(crate) fn delivery_asked(rows: &[SessionRow]) -> bool {
    rows.iter().any(|row| matches!(row, SessionRow::Delivery { .. }))
}

/// The Job's first message from the person.
pub(crate) fn first_message(rows: &[SessionRow]) -> Option<&str> {
    rows.iter().find_map(|row| match row {
        SessionRow::Message { from: SessionVoice::You, text, .. } => Some(text.as_str()),
        _ => None,
    })
}

const LOCAL: &[&str] =
    &["keep it local", "keep this local", "keep local", "local only", "do not push", "don't push", "dont push", "without pushing"];
const OPEN_PR: &[&str] = &[
    "open a pr",
    "open a pull request",
    "open the pr",
    "open pr only",
    "pr only",
    "pull request only",
    "stop at the pr",
    "stop at the pull request",
];
const AUTO: &[&str] = &["auto land", "auto-land", "auto merge", "auto-merge", "land it", "merge it", "merge when green", "and merge", "then merge"];
/// A negation near a delivery word flips it, and Fleet does not read which way: it asks.
const NEGATED: &[&str] = &["not merge", "n't merge", "not land", "n't land", "no merge", "not open a pr", "n't open a pr", "not open a pull request"];

/// The delivery a message names, if it names exactly one. **Conservative**: no phrase, two different
/// deliveries, or a negated one is `None`, and the owner is asked.
pub(crate) fn named_delivery(text: &str) -> Option<DeliveryChoice> {
    let said = text.to_lowercase().replace('\u{2019}', "'");
    if NEGATED.iter().any(|phrase| said.contains(phrase)) {
        return None;
    }
    let hits: Vec<DeliveryChoice> = [
        (LOCAL, DeliveryChoice::KeepLocal),
        (OPEN_PR, DeliveryChoice::OpenPr),
        (AUTO, DeliveryChoice::AutoLand),
    ]
    .into_iter()
    .filter(|(phrases, _)| phrases.iter().any(|phrase| said.contains(phrase)))
    .map(|(_, choice)| choice)
    .collect();
    match hits.as_slice() {
        [one] => Some(*one),
        _ => None,
    }
}

/// What the agent is told when the question is put: stop until it is answered. **It says Fleet asked**,
/// or the agent, told a decision of the owner's goes on `waiting_for`, asks it a second time.
pub(crate) const ASKED: &str = "Fleet: the owner is choosing how this work is delivered: auto land, open a pull request only, or keep it local. \
Fleet has asked him already, so do not ask it again, in waiting_for or with AskUserQuestion. \
Stop now and end your turn. Do not edit, commit or push. Fleet sends the choice when it is made.";

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
    /// The Job has entered a step that asks. Keep a `delivery` row, answered where the first message
    /// named one and asking where it did not, and tell the agent to stop in the second case.
    /// **Answers whether the step's own instructions were held back**: they go with the choice. A
    /// Job whose thread has no message from the person yet is left alone, and a row already kept
    /// is never written again.
    pub(crate) async fn delivery_entered(self: &Arc<Self>, id: &str) -> bool {
        let Ok(rows) = self.rows_of(id).await else { return false };
        if delivery_asked(&rows) {
            return false;
        }
        let Some(first) = first_message(&rows) else { return false };
        if !self.delivery_row_kept(id, first).await {
            return false;
        }
        let _ = self.told_by_fleet(id, ASKED).await;
        true
    }

    /// Auto land answered turns the Session's flag on, by the path `set_session_auto_land` takes.
    /// The other two answers leave the flag as it is. **A refusal never undoes the answer**: the
    /// forge's words go to the agent, as a pull request attaching under Auto land does.
    async fn auto_land_follows(self: &Arc<Self>, id: &str, choice: Option<DeliveryChoice>) {
        if choice != Some(DeliveryChoice::AutoLand) {
            return;
        }
        let asked = SetSessionAutoLand { id: SessionId::carried(id), on: true };
        if let Err(refusal) = self.session_auto_land_set(asked).await {
            let why = &refusal.error().message;
            let _ = self.told_by_fleet(id, &format!("Auto land was chosen, and it could not be turned on in full: {why}")).await;
        }
    }

    /// Keep the `delivery` row `message` gives: answered where it names a delivery, open where it does
    /// not. `true` where the question is open.
    pub(crate) async fn delivery_row_kept(&self, id: &str, message: &str) -> bool {
        let named = named_delivery(message);
        let row = SessionRow::Delivery { id: self.row_id(id), at: self.instant(), choice: named, named: named.is_some() };
        self.row_put(id, row).await;
        if named == Some(DeliveryChoice::AutoLand) {
            // Named before any pull request exists, so only a terminal or ended Job refuses, and it is said in the log.
            if let Err(refusal) = self.session_auto_land_set(SetSessionAutoLand { id: SessionId::carried(id), on: true }).await {
                self.session_warned(id, "auto land was named and not turned on", &refusal.error().message);
            }
        }
        named.is_none()
    }

    /// `answer_waiting` on the `delivery` item: keep the choice on its row and tell the agent, with the
    /// instructions of the step that asked.
    pub(crate) async fn delivery_chosen(
        self: &Arc<Self>,
        id: &str,
        choice: Option<u32>,
    ) -> Result<SessionRecord, Refusal> {
        let chosen = choice.and_then(|at| DeliveryChoice::OFFERED.get(at as usize).copied()).ok_or_else(|| {
            self.hosted_refusal("fleet.session_waiting_empty", "how the work is delivered is answered by choosing one of the three")
        })?;
        let rows = self.rows_of(id).await?;
        let Some(SessionRow::Delivery { id: row_id, at, .. }) =
            rows.into_iter().find(|row| matches!(row, SessionRow::Delivery { choice: None, .. }))
        else {
            return Err(self.settled_already());
        };
        self.row_put(id, SessionRow::Delivery { id: row_id, at, choice: Some(chosen), named: false }).await;
        self.auto_land_follows(id, Some(chosen)).await;
        let record = self.published_hosted(id).await?;
        let mut words = format!("Fleet: the owner chose {}. {}", chosen.label(), chosen.detail());
        let (manifest, workflow) = (record.manifest_id.as_ref().map_or("", |one| one.as_str()), record.workflow.as_deref().unwrap_or(""));
        let held = self
            .rail_steps(manifest, workflow)
            .and_then(|steps: Vec<RailStep>| steps.iter().find(|step| step.asks_delivery).and_then(|step| rail::instructed(&self.workflow_name(manifest, workflow), step)));
        if let Some(held) = held {
            words.push_str("\n\n");
            words.push_str(&held);
        }
        let _ = self.told_by_fleet(id, &words).await;
        Ok(record)
    }
}
