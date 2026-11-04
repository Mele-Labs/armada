//! Fleet handing the Operator what arrives: a dispatch and a Job an agent started, at their start,
//! and each group of a plan recorded that is new or names other paths than before. Seated once,
//! at start, on the Operator model with one call kept started.
//!
//! **A start never waits for its ruling.** Its first write does, at most [`FIRST_WRITE_WAIT`] and
//! only while Dispatch is on, so a ruling that hands the Job's work away stops it having only read.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Model, Vcs, WorkProduct};
use config::settings as keys;
use ipc::Notice;
use store::KeptPlan;

use super::candidates::shared_paths;
use super::desk::held_of;
use super::{Arrival, Arrived, Calling, FleetActing, FleetDesk, KeptStarted, Moved, Operator, Ruled, Source, Way, FIRST_WRITE_WAIT, NEEDS_YOU_WAIT};
use crate::daemon::Fleet;

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
    /// Seat the Operator, its call kept started on the Operator model as settings.json names it
    /// at each call. A Fleet whose call cannot be assembled seats none, and every arrival goes on
    /// as it did before the Operator.
    pub fn operator_seated(self: &Arc<Self>) {
        let proposing = match self.proposing() {
            Ok(proposing) => proposing,
            Err(why) => {
                eprintln!("the Operator was not seated: {why:?}");
                return;
            }
        };
        let fleet = Arc::downgrade(self);
        let model = move || Model::named(&fleet.upgrade()?.machine_settings().now().get(keys::MODELS_OPERATOR)).ok();
        let call = Arc::new(KeptStarted::new(proposing.client, model, proposing.environment, proposing.budget));
        if self.operator_seated_with(call.clone()) {
            tokio::spawn(async move {
                if let Err(why) = call.ready().await {
                    eprintln!("the Operator's call did not start ahead of its first arrival: {why}");
                }
            });
        }
    }

    /// Seat an Operator making its calls through `call`. `false` where one is already seated.
    pub(crate) fn operator_seated_with(self: &Arc<Self>, call: Arc<dyn Calling>) -> bool {
        let desk = FleetDesk::on(self, |why| eprintln!("the Operator's Log: {why}"));
        self.operator_seat().seat(Operator::new(call, Arc::new(desk), Arc::new(FleetActing::on(self))))
    }

    /// A Job's start, handed to the Operator with `words`, what it was asked.
    pub(crate) async fn start_handed(&self, id: &str, source: Source, words: &str) {
        let arrival = {
            let store = self.store().lock().await;
            let (Ok(Some(session)), Ok(Some(hosting))) = (store.session(id), store.hosting(id)) else { return };
            let (paths, issues, _) = held_of(&store, id);
            let subject = session.title.unwrap_or_else(|| words.lines().next().unwrap_or_default().trim().to_string());
            Arrival {
                id: format!("start:{id}"),
                arrived: Arrived::Start,
                source,
                session_id: id.to_string(),
                manifest_id: hosting.manifest_id,
                subject,
                words: words.to_string(),
                paths,
                issues,
                group: None,
                ways: Way::STARTING.to_vec(),
            }
        };
        self.operator_seat().hand_start(arrival, FIRST_WRITE_WAIT);
    }

    /// Each group of `kept`, just recorded, that `before` did not hold with the same paths. A group
    /// that goes to a Job of its own is that Job's start to rule, so it is not handed here.
    pub(crate) async fn plan_handed(&self, id: &str, before: Option<&KeptPlan>, kept: &KeptPlan) {
        self.operator_seat().moved(id, Moved::Plan);
        let Ok(Some(hosting)) = self.store().lock().await.hosting(id) else { return };
        for group in kept.groups.iter().filter(|group| !group.own_job && !group.paths.is_empty()) {
            let held = before.and_then(|before| before.groups.iter().find(|one| one.id == group.id));
            if held.is_some_and(|one| one.paths == group.paths) {
                continue;
            }
            let mut words: Vec<String> = group.gives.iter().cloned().collect();
            words.extend(group.tasks.iter().map(|task| task.text.clone()));
            self.operator_seat().hand(
                Arrival {
                    id: format!("plan:{id}:{}:{}", kept.version, group.id),
                    arrived: Arrived::Plan,
                    source: Source::Agent,
                    session_id: id.to_string(),
                    manifest_id: hosting.manifest_id.clone(),
                    subject: group.title.clone(),
                    words: words.join(". "),
                    paths: group.paths.clone(),
                    issues: group.issue.iter().cloned().collect(),
                    group: Some(group.id.clone()),
                    ways: Way::STARTING.to_vec(),
                },
                NEEDS_YOU_WAIT,
            );
        }
    }

    /// What a Job's first write is told where its start's ruling sends its work to another Job,
    /// having waited for that ruling at most [`FIRST_WRITE_WAIT`]. `None` lets the write go on.
    pub(crate) async fn first_write_ruled(&self, id: &str) -> Option<String> {
        if !self.operator_dispatches() {
            return None;
        }
        let mut ruling = self.operator_seat().start_ruling(id)?;
        let waited = tokio::time::timeout(FIRST_WRITE_WAIT, ruling.wait_for(Option::is_some)).await;
        let ruled = waited.ok()?.ok()?.clone()?;
        match ruled {
            Ruled::Ruled(ruling) if matches!(ruling.way, Way::ExistingJob | Way::Merge) => {
                let other = ruling.other.unwrap_or_default();
                Some(format!(
                    "The Operator found Job {other} already doing this work, so it goes to that Job and this Job ends. \
                     Make no changes: stop here."
                ))
            }
            _ => None,
        }
    }

    /// Tell each live Job kept in touch with one that moved, where the move was on the paths they
    /// share. Once a tick, and nothing while Dispatch is off.
    pub async fn in_touch_told(self: &Arc<Self>) {
        let moved = self.operator_seat().take_moved();
        if moved.is_empty() || !self.operator_dispatches() {
            return;
        }
        for (id, how) in moved {
            let (pairs, plan, touched) = {
                let store = self.store().lock().await;
                let pairs = store.operator_in_touch(&id).unwrap_or_default();
                let plan: Vec<String> = store.session_plan(&id).ok().flatten().into_iter().flat_map(|plan| plan.groups).flat_map(|group| group.paths).collect();
                let touched: Vec<String> = store.session_touches(&id).unwrap_or_default().into_iter().map(|touch| touch.path).collect();
                (pairs, plan, touched)
            };
            for (other, shared) in pairs {
                let live = self.store().lock().await.session(&other).ok().flatten().is_some_and(|one| one.state != store::SessionState::Ended);
                let (paths, did) = match how {
                    Moved::Plan => (shared_paths(&plan, &shared), "changed its plan on ground you share"),
                    Moved::Pushed => (shared_paths(&touched, &shared), "pushed on ground you share"),
                };
                if !live || paths.is_empty() {
                    continue;
                }
                let text = self.in_touch_said(&id, &paths, did).await;
                let _ = self.told_by_fleet(&other, &text, Notice::Operator).await;
            }
        }
    }
}
