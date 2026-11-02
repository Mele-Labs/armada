//! The decision pass: what a running Autopilot answers for the person, by the boxes they checked.
//! `docs/concepts/autopilot.md`.
//!
//! **A kind that is checked is answered, a kind that is not waits, and destructive always waits.**
//! Each answer is one fresh call (`super::deciding`) that names a way and says it is sure; anything
//! else leaves the item with the person, and an item already left is not asked about again until it
//! changes. Only while the Autopilot is running: paused or stopped answers nothing.
//!
//! **Its own loop, beside the pick's**, so a landing never waits behind a call.

use std::collections::BTreeSet;
use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::HostedSessions;
use ipc::{
    AnswerWaiting, ApprovePlan, AutopilotDecision, HelmCallAnswer, HelmCallInFlight, SessionRow, WaitingItem,
    WaitingSource,
};
use store::{KeptAutopilot, KeptDecision, SessionState};
use tokio::task::JoinHandle;

use super::bounds::State;
use super::deciding::{decided, Deciding, NotDecided, Reading, EARLIER};
use super::destructive::{destructive_text, GRACE};
use super::sorting::sorted;
use super::wire::{decision_from_word, decision_word};
use super::Asking;
use crate::adrift::Adrift;
use crate::daemon::Fleet;

/// The most calls one repository makes in one pass. **A bound on money**: what is left is asked
/// about at the next.
const CALLS_PER_PASS: usize = 4;

/// What one repository's pass carries.
pub(super) struct Helping {
    pub kept: KeptAutopilot,
    pub kinds: BTreeSet<AutopilotDecision>,
    asking: Asking,
    earlier: Vec<KeptDecision>,
    calls: usize,
}

/// One Job's waiting items, with its name.
pub(super) struct Job {
    pub id: String,
    pub who: String,
    pub items: Vec<WaitingItem>,
}

/// Decide for every running Autopilot, every `tick`.
pub fn keep_deciding<H, V, W>(
    fleet: Arc<Fleet<H, V, W>>,
    tick: Duration,
    adrift: Arc<impl Fn(Adrift) + Send + Sync + 'static>,
) -> JoinHandle<()>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(tick);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            ticker.tick().await;
            if let Err(why) = fleet.decision_pass().await {
                adrift(why);
            }
        }
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
    /// One pass over every running Autopilot. Answers how many decisions it answered.
    pub(crate) async fn decision_pass(self: &Arc<Self>) -> Result<usize, Adrift> {
        let every = self.store().lock().await.autopilots().map_err(Adrift::Writing)?;
        let mut answered = 0;
        for kept in every {
            if State::from_stored(&kept.state) != Some(State::Running) {
                continue;
            }
            let kinds: BTreeSet<_> = kept.decisions.iter().filter_map(|word| decision_from_word(word)).collect();
            if kinds.is_empty() {
                continue;
            }
            let Ok(asking) = self.asking() else { continue };
            let earlier = self
                .store()
                .lock()
                .await
                .autopilot_decisions_of(&kept.manifest_id, EARLIER)
                .map_err(Adrift::Writing)?;
            let mut helping = Helping { kept, kinds, asking, earlier, calls: CALLS_PER_PASS };
            let jobs = self.jobs_in_scope(&helping.kept).await;
            answered += self.items_answered(&mut helping, &jobs).await;
            answered += self.pulls_landed(&mut helping, &jobs).await;
            answered += self.set_asides_started(&mut helping).await;
            answered += self.red_fixed(&helping).await;
        }
        Ok(answered)
    }

    /// The Jobs whose items this Autopilot answers: every live Job of the repository where it works
    /// what is running, and otherwise only the ones it started.
    pub(super) async fn jobs_in_scope(&self, kept: &KeptAutopilot) -> Vec<Job> {
        let store = self.store().lock().await;
        let Ok(found) = store.find_sessions(&store::SessionSearch {
            manifest_id: Some(&kept.manifest_id),
            state: Some(SessionState::Live),
            text: None,
        }) else {
            return Vec::new();
        };
        let started: BTreeSet<String> = if kept.on_running {
            BTreeSet::new()
        } else {
            store
                .autopilot_picks(&kept.run_id)
                .unwrap_or_default()
                .into_iter()
                .filter_map(|pick| pick.started_session)
                .collect()
        };
        found
            .iter()
            .filter(|session| kept.on_running || started.contains(&session.id))
            .filter_map(|session| {
                let items = self.ledger_row(&store, session).ok()?.waiting_for;
                let who = session.title.clone().unwrap_or_else(|| session.id.clone());
                Some(Job { id: session.id.clone(), who, items })
            })
            .collect()
    }

    async fn items_answered(self: &Arc<Self>, helping: &mut Helping, jobs: &[Job]) -> usize {
        let mut answered = 0;
        for job in jobs {
            // Only a Job Fleet hosts can be answered: a terminal's own prompt is answered in the terminal.
            if !matches!(self.terminal_session(&job.id).await, Ok(None)) {
                continue;
            }
            for item in &job.items {
                let id = format!("{}:{}", job.id, item.id);
                if self.decision_kept(&id).await {
                    continue;
                }
                let card = self.card_of(&job.id, item);
                let Some(kind) = sorted(item, card.as_ref()).filter(|kind| helping.kinds.contains(kind)) else {
                    continue;
                };
                if !waited_out(item, self.now().epoch_millis()) {
                    continue;
                }
                let Some(deciding) = self.deciding_of(&job.id, item, kind, card.as_ref()).await else {
                    continue;
                };
                let Some((way, because)) = self.helper_says(helping, &id, &deciding).await else {
                    continue;
                };
                if self.answer_for_them(&job.id, item, way, card.as_ref()).await.is_err() {
                    continue;
                }
                let asked = item.text.clone();
                let chose = &deciding.ways[way];
                self.decision_kept_as(helping, &id, Some(&job.id), &job.who, kind, asked, chose, because).await;
                answered += 1;
            }
        }
        answered
    }

    fn card_of(&self, session: &str, item: &WaitingItem) -> Option<HelmCallInFlight> {
        if item.source != WaitingSource::Permission {
            return None;
        }
        let asked = self.hosts().of(session).state().asked.clone();
        asked.filter(|card| format!("perm:{}", card.call) == item.id)
    }

    /// What the helper is shown of one item: the ways to answer it, and what it is about.
    async fn deciding_of(
        self: &Arc<Self>,
        session: &str,
        item: &WaitingItem,
        kind: AutopilotDecision,
        card: Option<&HelmCallInFlight>,
    ) -> Option<Deciding> {
        let (ways, detail): (Vec<String>, Option<String>) = match item.source {
            WaitingSource::AskCard | WaitingSource::Agent => {
                (item.options.iter().map(|one| one.label.clone()).collect(), None)
            }
            WaitingSource::Permission => {
                let card = card?;
                card.offers.contains(&HelmCallAnswer::AllowOnce).then_some(())?;
                (vec!["Allow once".to_string()], card.whole.clone())
            }
            WaitingSource::Plan => {
                let plan = self.store().lock().await.session_plan(session).ok().flatten()?;
                let titles: Vec<&str> = plan.groups.iter().map(|group| group.title.as_str()).collect();
                (vec!["Approve the plan".to_string()], Some(format!("{}\n{}", plan.summary, titles.join("\n"))))
            }
            WaitingSource::Draft => {
                let read = Arc::clone(self).get_session(ipc::SessionId::carried(session)).await.ok()?;
                let text = read.rows.into_iter().find_map(|row| match row {
                    SessionRow::Draft { id, text, .. } if format!("draft:{id}") == item.id => Some(text),
                    _ => None,
                });
                (vec!["Approve as written".to_string()], text)
            }
            WaitingSource::Walk => {
                (vec!["Approve the walk".to_string()], item.act.as_ref().map(|act| act.target.clone()))
            }
            _ => return None,
        };
        Some(Deciding { kind, subject: item.text.clone(), detail, ways })
    }

    /// Ask the helper, unless this decision was left as it stands or it is resting after a failure.
    /// **A way is returned only where it was sure**, and it is read for destructive words again.
    pub(super) async fn helper_says(
        &self,
        helping: &mut Helping,
        id: &str,
        deciding: &Deciding,
    ) -> Option<(usize, String)> {
        let memory = &self.tracking().autopilot;
        let what = format!("{}\n{:?}\n{:?}", deciding.subject, deciding.detail, deciding.ways);
        let key = format!("decisions:{}", helping.kept.manifest_id);
        let now_ms = self.now().epoch_millis().unwrap_or_default();
        if helping.calls == 0 || memory.left_as_it_is(id, &what) || memory.picker_resting(&key, now_ms) {
            return None;
        }
        helping.calls -= 1;
        let read = decided(&helping.kept.instruction, deciding, &helping.earlier, &helping.asking).await;
        // A Pause or a Stop pressed while the call was out wins over its answer.
        if !self.still_deciding(&helping.kept).await {
            return None;
        }
        match read {
            Ok(Reading::Answer { way, because }) if !destructive_text(&deciding.ways[way]) => {
                memory.picker_answered(&key);
                Some((way, because))
            }
            Ok(_) | Err(NotDecided::Unreadable { .. }) => {
                memory.picker_answered(&key);
                memory.leave(id, what);
                None
            }
            Err(NotDecided::Call(_)) => {
                memory.picker_failed(&key, now_ms);
                None
            }
        }
    }

    pub(super) async fn still_deciding(&self, kept: &KeptAutopilot) -> bool {
        let now = self.store().lock().await.autopilot(&kept.manifest_id);
        now.ok().flatten().is_some_and(|now| {
            now.run_id == kept.run_id && State::from_stored(&now.state) == Some(State::Running)
        })
    }

    pub(super) async fn decision_kept(&self, id: &str) -> bool {
        self.store().lock().await.autopilot_decision(id).ok().flatten().is_some()
    }

    /// Put an answer on record. **Left once**: an id already there changes nothing.
    pub(super) async fn decision_kept_as(
        &self,
        helping: &Helping,
        id: &str,
        session: Option<&str>,
        who: &str,
        kind: AutopilotDecision,
        asked: String,
        chose: &str,
        because: String,
    ) {
        let one = KeptDecision {
            id: id.to_string(),
            manifest_id: helping.kept.manifest_id.clone(),
            session_id: session.map(str::to_string),
            who: who.to_string(),
            kind: decision_word(kind).to_string(),
            asked,
            chose: chose.to_string(),
            because,
            at: self.now().as_str().to_string(),
            reviewed: false,
            corrected: None,
        };
        let new = self.store().lock().await.add_autopilot_decision(&one).unwrap_or(false);
        if new {
            let _ = self.autopilot_decisions_published().await;
        }
    }

    /// The answer, as Bridge's press makes it: a choice on a question, a card or a draft, nothing
    /// but the approval on a walk, and the plan's own route.
    async fn answer_for_them(
        self: &Arc<Self>,
        session: &str,
        item: &WaitingItem,
        way: usize,
        card: Option<&HelmCallInFlight>,
    ) -> Result<(), api::Refusal> {
        let said = |choice: Option<u32>| AnswerWaiting {
            session_id: ipc::SessionId::carried(session),
            item_id: item.id.clone(),
            choice,
            text: None,
            mode: None,
            fixed: None,
        };
        match item.source {
            WaitingSource::Plan => {
                let version = item.id.strip_prefix("plan:").and_then(|version| version.parse().ok());
                let version = version.ok_or_else(|| self.settled_already())?;
                Arc::clone(self)
                    .plan_approved(ApprovePlan { session_id: ipc::SessionId::carried(session), version })
                    .await
                    .map(|_| ())
            }
            WaitingSource::Walk => Arc::clone(self).settle_waiting(said(None)).await.map(|_| ()),
            WaitingSource::Draft => Arc::clone(self).settle_waiting(said(Some(0))).await.map(|_| ()),
            WaitingSource::Permission => {
                let at = card.and_then(|card| card.offers.iter().position(|one| *one == HelmCallAnswer::AllowOnce));
                let at = at.ok_or_else(|| self.settled_already())?;
                Arc::clone(self).settle_waiting(said(Some(at as u32))).await.map(|_| ())
            }
            _ => Arc::clone(self).settle_waiting(said(Some(way as u32))).await.map(|_| ()),
        }
    }
}

/// Whether the item has waited out [`GRACE`], so an agent could carry on by itself first.
fn waited_out(item: &WaitingItem, now: Option<i64>) -> bool {
    let since = core_model::Timestamp::from_rfc3339(item.since.as_str()).epoch_millis();
    now.zip(since).is_some_and(|(now, since)| now.saturating_sub(since) >= GRACE.as_millis() as i64)
}
