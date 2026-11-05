//! The Operator's [`Desk`] on a running Fleet: its clock, its settings and its store.

use std::collections::BTreeSet;
use std::sync::{Arc, Weak};

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use config::settings as keys;
use core_model::Timestamp;
use ipc::{SessionRow, SessionVoice};
use store::{AttachmentState, Holder};

use super::{Desk, Done, InLine, LiveJob, Logged};
use crate::session_issues::ISSUE;
use crate::adrift::Adrift;
use crate::daemon::Fleet;

/// The attachment on a started Job that names the Job that started it.
const STARTED_BY: &str = "started_by";

/// Fleet as the Operator reaches it. A store failure is said through `adrift`, and a count of
/// calls that cannot be read stands for the ceiling reached. **Held weakly**, since Fleet holds the
/// Operator that holds this: a Fleet that is gone has no Jobs and makes no calls.
pub struct FleetDesk<H, V, W> {
    fleet: Weak<Fleet<H, V, W>>,
    adrift: Box<dyn Fn(Adrift) + Send + Sync>,
}

impl<H, V, W> FleetDesk<H, V, W> {
    pub fn on(fleet: &Arc<Fleet<H, V, W>>, adrift: impl Fn(Adrift) + Send + Sync + 'static) -> FleetDesk<H, V, W> {
        FleetDesk { fleet: Arc::downgrade(fleet), adrift: Box::new(adrift) }
    }
}

impl<H, V, W> Desk for FleetDesk<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    fn now(&self) -> Timestamp {
        match self.fleet.upgrade() {
            Some(fleet) => fleet.now(),
            None => Timestamp::from_rfc3339(crate::clock::rfc3339_utc(0)),
        }
    }

    fn day_began(&self) -> Done<'_, Timestamp> {
        Box::pin(async move {
            let now = self.now().epoch_millis().unwrap_or(0);
            Timestamp::from_rfc3339(crate::clock::rfc3339_utc(crate::clock::local_day_began(now).await))
        })
    }

    fn daily_calls(&self) -> u32 {
        let Some(fleet) = self.fleet.upgrade() else { return 0 };
        let calls = fleet.machine_settings().now().get(keys::OPERATOR_DAILY_CALLS);
        u32::try_from(calls.max(0)).unwrap_or(u32::MAX)
    }

    /// Every hosted Job not ended, with what Fleet holds of its work: its plan's
    /// paths and the files it has written, the issues it holds or its plan works, and its words,
    /// which are its plan where it has one and its first message where it has not.
    fn live(&self) -> Done<'_, Vec<LiveJob>> {
        Box::pin(async move {
            let Some(fleet) = self.fleet.upgrade() else { return Vec::new() };
            let store = fleet.store().lock().await;
            let hostings = match store.hostings() {
                Ok(hostings) => hostings,
                Err(why) => {
                    (self.adrift)(Adrift::Writing(why));
                    return Vec::new();
                }
            };
            let mut live = Vec::new();
            for hosting in hostings.into_iter() {
                let Ok(Some(session)) = store.session(&hosting.session_id) else { continue };
                if session.state == store::SessionState::Ended {
                    continue;
                }
                let (paths, issues, words) = held_of(&store, &session.id);
                let started_by = started_by_chain(&store, &session.id);
                live.push(LiveJob {
                    title: session.title.unwrap_or_else(|| session.id.clone()),
                    session_id: session.id,
                    manifest_id: hosting.manifest_id,
                    words,
                    paths,
                    issues,
                    started_by,
                });
            }
            live
        })
    }

    fn calls_since(&self, since: Timestamp) -> Done<'_, u32> {
        Box::pin(async move {
            let Some(fleet) = self.fleet.upgrade() else { return u32::MAX };
            let counted = fleet.store().lock().await.operator_calls_since(since.as_str());
            match counted {
                Ok(calls) => calls,
                Err(why) => {
                    (self.adrift)(Adrift::Writing(why));
                    u32::MAX
                }
            }
        })
    }

    fn waiting(&self) -> Done<'_, Vec<InLine>> {
        Box::pin(async move {
            match self.fleet.upgrade() {
                Some(fleet) => fleet.waiting_to_start().await,
                None => Vec::new(),
            }
        })
    }

    fn settled(&self, arrival: &super::Arrival) -> Done<'_, Option<String>> {
        let arrival = arrival.clone();
        Box::pin(async move {
            match self.fleet.upgrade() {
                Some(fleet) => fleet.answer_settled(&arrival).await,
                None => Some("Fleet stopped before the ruling came".into()),
            }
        })
    }

    fn orders(&self) -> Done<'_, super::Orders> {
        Box::pin(async move { self.fleet.upgrade().map(|fleet| fleet.operator_seat().orders()).unwrap_or_default() })
    }

    fn board(&self) -> Done<'_, super::Board> {
        Box::pin(async move {
            match self.fleet.upgrade() {
                Some(fleet) => fleet.operator_board().await,
                None => super::Board::default(),
            }
        })
    }

    fn board_changed(&self) {
        if let Some(fleet) = self.fleet.upgrade() {
            fleet.events().publish(ipc::Event::OperatorBoardChanged(ipc::operator_board::OperatorBoardChanged {}));
        }
    }

    fn keep(&self, row: Logged) -> Done<'_, ()> {
        Box::pin(async move {
            // A call out when the Operator was turned off is recorded nowhere, as if it had not been made.
            let Some(fleet) = self.fleet.upgrade().filter(|fleet| fleet.operator_on()) else { return };
            let kept = fleet.store().lock().await.add_operator_ruling(&row.kept());
            if let Err(why) = kept {
                (self.adrift)(Adrift::Writing(why));
            }
        })
    }
}

/// Who started `id`, nearest first and up to the root: `(id, title)` of its starter, then of the
/// starter's. Read from each Job's own `started_by` attachment, which is the row already read for
/// its issues, so no new read. An ended starter still counts; a repeat ends the walk.
pub(super) fn started_by_chain(store: &store::Store, id: &str) -> Vec<(String, String)> {
    let mut chain: Vec<(String, String)> = Vec::new();
    let mut at = id.to_string();
    loop {
        let held = store.attachments_of(&Holder::session(&at)).unwrap_or_default();
        let Some(starter) = held.into_iter().find(|one| one.kind == STARTED_BY).map(|one| one.target) else { break };
        if starter == id || chain.iter().any(|(seen, _)| *seen == starter) {
            break;
        }
        let title = store.session(&starter).ok().flatten().and_then(|one| one.title).unwrap_or_else(|| starter.clone());
        chain.push((starter.clone(), title));
        at = starter;
    }
    chain
}

/// One Job's paths, issues and words, as [`FleetDesk::live`] reads them. What will not read is left out.
pub(super) fn held_of(store: &store::Store, id: &str) -> (Vec<String>, Vec<String>, String) {
    let mut paths = BTreeSet::new();
    let mut issues = BTreeSet::new();
    let plan = store.session_plan(id).ok().flatten();
    let words = match &plan {
        Some(plan) => {
            let mut words = vec![plan.summary.clone()];
            for group in &plan.groups {
                paths.extend(group.paths.iter().cloned());
                issues.extend(group.issue.iter().cloned());
                words.push(group.title.clone());
            }
            words.join(". ")
        }
        None => first_words(store, id),
    };
    paths.extend(store.session_touches(id).unwrap_or_default().into_iter().map(|touch| touch.path));
    let held = store.attachments_of(&Holder::session(id)).unwrap_or_default();
    issues.extend(held.into_iter().filter(|one| one.kind == ISSUE && one.state != AttachmentState::GivenBack).map(|one| one.target));
    (paths.into_iter().collect(), issues.into_iter().collect(), words)
}

/// The first message in a Job's thread that is not its agent's: the dispatch, or the brief the
/// Job that started it wrote.
pub(super) fn first_words(store: &store::Store, id: &str) -> String {
    store
        .session_rows(id)
        .unwrap_or_default()
        .iter()
        .filter(|body| body.contains("\"message\""))
        .filter_map(|body| ipc::decode::<SessionRow>("a job row", body.as_bytes()).ok())
        .find_map(|row| match row {
            SessionRow::Message { from, text, notice: None, .. } if from != SessionVoice::Agent => Some(text),
            _ => None,
        })
        .unwrap_or_default()
}
