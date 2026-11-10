//! The Operator's [`Desk`] on a running Fleet: its clock, its settings and its store.

use std::collections::BTreeSet;
use std::sync::{Arc, Weak};

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use core_model::Timestamp;
use ipc::{SessionRow, SessionVoice};
use store::{AttachmentState, Holder};

use super::{Desk, Done, InLine, LiveJob, Logged, Paused};
use crate::session_issues::ISSUE;
use crate::adrift::Adrift;
use crate::daemon::Fleet;

/// The attachment on a started Job that names the Job that started it.
const STARTED_BY: &str = "started_by";

/// Fleet as the Operator reaches it. A store failure is said through `adrift`. **Held weakly**, since Fleet holds the
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

    fn plan_pause(&self) -> Done<'_, Option<Paused>> {
        Box::pin(async move {
            let fleet = self.fleet.upgrade()?;
            fleet.operator_paused().await
        })
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
                let held = store.attachments_of(&Holder::session(&session.id)).unwrap_or_default();
                let running = fleet.running_of(&session.id, &held);
                let started_by = started_by_chain(&store, &session.id);
                live.push(LiveJob {
                    title: session.title.unwrap_or_else(|| session.id.clone()),
                    session_id: session.id,
                    manifest_id: hosting.manifest_id,
                    words,
                    paths,
                    issues,
                    running,
                    started_by,
                });
            }
            live
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

    fn jobs_read(&self, live: &[LiveJob]) -> Done<'_, Vec<super::talking::JobRead>> {
        let ids: Vec<String> = live.iter().map(|one| one.session_id.clone()).collect();
        Box::pin(async move {
            match self.fleet.upgrade() {
                Some(fleet) => fleet.operator_jobs_read(&ids).await,
                None => Vec::new(),
            }
        })
    }

    fn to_read(&self, live: &[LiveJob], needs: &[super::NeedsYou]) -> Done<'_, Option<super::ToRead>> {
        let (live, needs) = (live.to_vec(), needs.to_vec());
        Box::pin(async move {
            let fleet = self.fleet.upgrade()?;
            Some(fleet.operator_to_read(&live, &needs).await)
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
                return;
            }
            let subject = row.subject.lines().map(str::trim).find(|line| !line.is_empty()).unwrap_or_default();
            fleet.operator_journaled(&format!("Ruled: \"{subject}\" went {}. {}", row.outcome.word(), row.because)).await;
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
