//! Fleet handing the Operator each question and permission card a Job waits on, and settling one
//! it answers as the person's press would (`settle_waiting`). **Read from state**: the Job's Needs
//! you items, its plan's seen marks, the person's own earlier answers in the Log, and whether
//! Autopilot runs for it.
//!
//! **Once an item.** A read of a row notes what it waits on; the tick hands each on unless the Log
//! already holds it or a call on it is out, so a row read many times arrives once. An item with no
//! Answer path is left after its first read, and pulses as it does today.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use core_model::Actor;
use ipc::{AnswerWaiting, AutopilotDecision, HelmCallAnswer, SessionId, WaitingItem, WaitingSource};

use super::questions::{asked_id, asked_item, Choice, Earlier, Group, Kind, Present, Question, ALLOW_ONCE};
use super::seat::Seat;
use super::{held, Arrival, Arrived, Logged, Outcome, Source, NEEDS_YOU_WAIT};
use crate::autopilot::bounds::State;
use crate::daemon::Fleet;

/// The run command's tool, whose card is the person's while he is here.
const RUN: &str = "Bash";

/// Whether `item` is of a kind with an Answer path: a question or a permission card.
pub(crate) fn answers(item: &WaitingItem) -> bool {
    matches!(item.source, WaitingSource::AskCard | WaitingSource::Agent | WaitingSource::Permission)
}

/// The items each row's last read found waiting with an Answer path, those a call is out on, and
/// those found with none.
#[derive(Default)]
pub(crate) struct Questions {
    seen: Mutex<BTreeMap<String, Vec<WaitingItem>>>,
    out: Arc<Mutex<BTreeSet<String>>>,
    left: Mutex<BTreeSet<String>>,
}

impl Seat {
    /// What a read of `session`'s row found it waiting on.
    pub(crate) fn asked_noted(&self, session: &str, waiting: &[WaitingItem]) {
        let asked: Vec<WaitingItem> = waiting.iter().filter(|item| answers(item)).cloned().collect();
        let mut held_now = held(&self.questions.seen);
        match asked.is_empty() {
            true => held_now.remove(session),
            false => held_now.insert(session.to_string(), asked),
        };
    }

    fn asked_seen(&self) -> Vec<(String, WaitingItem)> {
        let seen = held(&self.questions.seen);
        let now: Vec<(String, WaitingItem)> =
            seen.iter().flat_map(|(id, items)| items.iter().map(move |item| (id.clone(), item.clone()))).collect();
        let still: BTreeSet<String> = now.iter().map(|(id, item)| asked_id(id, &item.id)).collect();
        held(&self.questions.left).retain(|id| still.contains(id));
        now
    }

    /// Rule on a question in the background, unless a call on it is already out.
    fn hand_question(&self, arrival: Arrival, within: Duration) {
        let Some(operator) = self.operator() else { return };
        let out = Arc::clone(&self.questions.out);
        if !held(&out).insert(arrival.id.clone()) {
            return;
        }
        tokio::spawn(async move {
            let id = arrival.id.clone();
            operator.ruled(arrival, within).await;
            held(&out).remove(&id);
        });
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
    /// Each question and card a read found waiting, handed to the Operator once. Once a tick, and
    /// nothing while the Operator is off.
    pub async fn questions_handed(&self) {
        if !self.operator_on() {
            return;
        }
        let seat = self.operator_seat();
        for (session, item) in seat.asked_seen() {
            let id = asked_id(&session, &item.id);
            if held(&seat.questions.left).contains(&id) || held(&seat.questions.out).contains(&id) {
                continue;
            }
            if !self.unruled(&session, &id).await {
                continue;
            }
            match self.question_of(&session, &item).await.and_then(|question| question.arrival()) {
                Some(arrival) => seat.hand_question(arrival, NEEDS_YOU_WAIT),
                None => {
                    held(&seat.questions.left).insert(id);
                }
            }
        }
    }

    /// Whether arrival `id` of `session` is to be handed: the Log holds no row for it, or **an
    /// Autopilot run that covers the Job started after its last**, so what was waiting on him when
    /// it turned on is offered once more under its setup.
    async fn unruled(&self, session: &str, id: &str) -> bool {
        let store = self.store().lock().await;
        if !store.operator_ruled(id).unwrap_or(true) {
            return true;
        }
        let Some(manifest) = store.hosting(session).ok().flatten().map(|hosting| hosting.manifest_id) else { return false };
        away(&store, &manifest, session).is_some_and(|run| !store.operator_ruled_since(id, &run.started_at).unwrap_or(true))
    }

    /// `item` as the Operator reads it: what Fleet's rules make of it, the plan group its Job works
    /// on, the person's earlier answer to the same, and whether he is away. `None` for an item with
    /// no Answer path, or a Job in a terminal, whose prompt is answered there.
    pub(crate) async fn question_of(&self, session: &str, item: &WaitingItem) -> Option<Question> {
        if !matches!(self.terminal_session(session).await, Ok(None)) {
            return None;
        }
        let card = self.card_of(session, item);
        let kind = match crate::autopilot::sorting::sorted(item, card.as_ref())? {
            AutopilotDecision::Recommended => Kind::Recommended(crate::autopilot::destructive::recommended(item)?),
            AutopilotDecision::Open => Kind::Open,
            AutopilotDecision::Reads => Kind::Reads,
            AutopilotDecision::Writes if card.as_ref().is_some_and(|card| card.tool == RUN) => Kind::Runs,
            AutopilotDecision::Writes => Kind::Edits,
            _ => return None,
        };
        let (text, options) = match &card {
            Some(card) => (
                format!("{} {}", card.tool, card.whole.as_deref().unwrap_or(&card.detail)).trim().to_string(),
                card.offers.iter().map(|offer| offered(*offer)).collect(),
            ),
            None => (item.text.clone(), item.options.iter().map(|one| one.label.clone()).collect()),
        };
        let store = self.store().lock().await;
        let job = store.session(session).ok().flatten()?;
        let manifest = store.hosting(session).ok().flatten()?.manifest_id;
        let plan = store.session_plan(session).ok().flatten();
        let earlier = match card {
            Some(_) => None,
            None => store.operator_answers_to(&manifest, &item.text).unwrap_or_default().into_iter().find(|row| row.session_id != session).and_then(
                |row| {
                    let title = store.session(&row.session_id).ok().flatten()?.title.unwrap_or_else(|| row.session_id.clone());
                    Some(Earlier { session_id: row.session_id, title, answer: row.answer? })
                },
            ),
        };
        let present = match away(&store, &manifest, session) {
            Some(kept) => {
                let lets = kept.decisions.iter().filter_map(|word| crate::autopilot::wire::decision_from_word(word)).collect();
                Present::Away { lets, instruction: kept.instruction }
            }
            None => Present::Here,
        };
        Some(Question {
            session_id: session.to_string(),
            manifest_id: manifest,
            title: job.title.unwrap_or_else(|| session.to_string()),
            item_id: item.id.clone(),
            text,
            options,
            kind,
            group: plan.as_ref().map_or(Group::NoPlan, working_on),
            earlier,
            present,
        })
    }

    /// Item `item` of `id` is settled as his press on `choice` would, and the Job's thread says so.
    pub(super) async fn operator_answered(self: &Arc<Self>, id: &str, item: &str, choice: &Choice, because: &str) {
        let said = AnswerWaiting {
            session_id: SessionId::carried(id),
            item_id: item.to_string(),
            choice: Some(choice.at),
            text: None,
            mode: None,
            fixed: None,
        };
        if let Err(why) = Arc::clone(self).settle_waiting(said).await {
            return self.session_warned(id, "the Operator's answer did not settle", format!("{why:?}"));
        }
        let text = match &choice.earlier {
            Some(earlier) => format!("The Operator answered \"{}\", your answer when \"{}\" asked the same.", choice.label, earlier.title),
            None => format!("The Operator answered \"{}\": {because}", choice.label),
        };
        self.fleet_note(id, text).await;
    }

    /// Why the Operator's answer to `arrival` can no longer be given, as its call comes back.
    pub(super) async fn answer_settled(&self, arrival: &Arrival) -> Option<String> {
        if !self.operator_on() {
            return Some("the Operator was turned off before the ruling came".into());
        }
        let item = asked_item(arrival)?;
        let store = self.store().lock().await;
        let session = store.session(&arrival.session_id).ok().flatten()?;
        let waiting = self.ledger_row(&store, &session).map(|row| row.waiting_for).unwrap_or_default();
        (!waiting.iter().any(|one| one.id == item)).then(|| "the person answered it before the ruling came".into())
    }

    /// The person's own press on a question, kept in the Log while the Operator is on, so a second
    /// Job asking the same is offered his answer. Read before the press settles it.
    pub(crate) async fn person_answer_of(&self, said: &AnswerWaiting) -> Option<Logged> {
        if !self.operator_on() {
            return None;
        }
        let id = said.session_id.as_str();
        let store = self.store().lock().await;
        let session = store.session(id).ok().flatten()?;
        let manifest = store.hosting(id).ok().flatten()?.manifest_id;
        let item = self.ledger_row(&store, &session).ok()?.waiting_for.into_iter().find(|one| one.id == said.item_id)?;
        if !matches!(item.source, WaitingSource::AskCard | WaitingSource::Agent) {
            return None;
        }
        let answer = item.options.get(said.choice? as usize)?.label.clone();
        Some(Logged {
            arrival: asked_id(id, &item.id),
            arrived: Arrived::Question,
            source: Source::Person,
            session_id: id.to_string(),
            manifest_id: manifest,
            subject: item.text,
            outcome: Outcome::Answered,
            other: None,
            shared: Vec::new(),
            because: String::new(),
            call: None,
            by: Actor::Human,
            at: self.now(),
            place: None,
            first: None,
            refused: Vec::new(),
            answer: Some(answer),
            issue: None,
        })
    }

    /// Keep `row`, the person's own answer, once his press has settled it.
    pub(crate) async fn person_answer_kept(&self, row: Logged) {
        let _ = self.store().lock().await.add_operator_ruling(&row.kept());
    }
}

/// The Autopilot run that has the person away from `session`: running in its repository, and
/// over what is running or a Job it started.
fn away(store: &store::Store, manifest: &str, session: &str) -> Option<store::KeptAutopilot> {
    let kept = store.autopilot(manifest).ok().flatten().filter(|kept| State::from_stored(&kept.state) == Some(State::Running))?;
    let started_it = || store.autopilot_picks(&kept.run_id).unwrap_or_default().iter().any(|pick| pick.started_session.as_deref() == Some(session));
    (kept.on_running || started_it()).then_some(kept)
}

/// The plan group the Job works on: the one with a task in progress, else the first with a task
/// left, else the last. **A plan marked seen with no group marked reads as every group seen**, as
/// a plan kept before the marks moved to its groups.
fn working_on(plan: &store::KeptPlan) -> Group {
    let status = |task: &store::KeptPlanTask| crate::plan_record::status(task).unwrap_or("pending").to_string();
    let group = plan
        .groups
        .iter()
        .find(|group| group.tasks.iter().any(|task| status(task) == "in_progress"))
        .or_else(|| plan.groups.iter().find(|group| group.tasks.iter().any(|task| status(task) != "completed")))
        .or(plan.groups.last());
    let Some(group) = group else { return Group::NoPlan };
    let whole = plan.seen && !plan.groups.iter().any(|one| one.seen);
    match group.seen || whole {
        true => Group::Seen(group.title.clone()),
        false => Group::Unseen(group.title.clone()),
    }
}

/// A card's offer as its option reads. Only [`ALLOW_ONCE`] is ever an answer the Operator gives.
fn offered(offer: HelmCallAnswer) -> String {
    match offer {
        HelmCallAnswer::AllowOnce => ALLOW_ONCE.to_string(),
        other => format!("{other:?}"),
    }
}
