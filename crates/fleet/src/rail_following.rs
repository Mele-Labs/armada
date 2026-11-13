//! What Fleet does when a Session's step moves. `docs/concepts/session.md`, *The rail*. The rules are
//! `rail`'s; this is the part that acts on them.
//!
//! **The move is entered on the timeline as `session.step`**, and the agent is told nothing of it. The
//! step itself is derived while the row is drawn (`Fleet::rail_drawn`), so this loop only reacts to a
//! published row and never decides one.

use std::collections::HashMap;
use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use ipc::{Event, SessionRecord, TimelineKind};
use store::{KeptHosting, KeptSession};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::daemon::Fleet;
use crate::rail::{self, RailStep};
use crate::session_host::delivering::{card_choice, chosen_words, delivery_asked, ASKED};
use crate::session_host::delivery_setting::DeliveryKept;

/// What the loop last acted on for a Session, so a row that moved nothing costs no read.
#[derive(Clone, PartialEq, Eq)]
struct Seen {
    step: Option<u32>,
    /// A delivering step's lifecycle end has come and the agent has not been told.
    lifted: bool,
}

/// Start reacting to a hosted Session's published row. Production starts it beside the timeline's, and
/// a test starts it itself.
pub fn keep_the_rail<H, V, W>(fleet: Arc<Fleet<H, V, W>>) -> JoinHandle<()>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    let (tell, mut heard) = mpsc::unbounded_channel::<SessionRecord>();
    fleet.events().tap(move |event| {
        if let Event::SessionChanged(record) = event {
            if record.hosted.is_some() && record.workflow.is_some() && record.state == ipc::SessionState::Live {
                let _ = tell.send(record.clone());
            }
        }
    });
    tokio::spawn(async move {
        let mut seen: HashMap<String, Seen> = HashMap::new();
        while let Some(record) = heard.recv().await {
            fleet.rail_moved(record, &mut seen).await;
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
    async fn rail_moved(self: &Arc<Self>, record: SessionRecord, seen: &mut HashMap<String, Seen>) {
        let id = record.id.as_str().to_string();
        let (Some(manifest), Some(workflow)) = (record.manifest_id.as_ref(), record.workflow.as_deref()) else {
            return;
        };
        let project = record.hosted.as_ref().and_then(|hosted| hosted.project.as_deref());
        if !self.workflow_on(manifest.as_str(), project, workflow) {
            return;
        }
        let Some(steps) = self.job_rail_steps(&id, manifest.as_str(), workflow).await else {
            return;
        };
        // A delivering step gated by a lifecycle is not left when its end comes, so the agent is woken here.
        let lifted = match record.workflow_step.map(|at| at as usize).and_then(|at| steps.get(at).map(|step| (at, step))) {
            Some((at, step)) if step.delivers && step.has_lifecycle_end() => {
                let name = self.workflow_name(manifest.as_str(), workflow);
                self.lifecycle_lifted(&id, at, step, &steps, &name, manifest.as_str()).await
            }
            _ => None,
        };
        let now = Seen { step: record.workflow_step, lifted: lifted.is_some() };
        if seen.get(&id) == Some(&now) {
            return;
        }
        let Some(ran) = self.rail_acted_on(&id, &steps, &record).await else {
            return;
        };
        // A Job that has not run is told nothing, and is looked at again once it has.
        if !ran {
            return;
        }
        seen.insert(id.clone(), now);
        if let Some(line) = lifted {
            let _ = self.told_by_fleet(&id, &line, ipc::Notice::RailStep).await;
        }
    }

    /// Enter the move on the timeline and keep where the rail stands. Answers whether the Job has run:
    /// **one that has not is told nothing**, since a word sent now would start it ahead of the message
    /// it waits for.
    async fn rail_acted_on(
        self: &Arc<Self>,
        id: &str,
        steps: &[RailStep],
        record: &SessionRecord,
    ) -> Option<bool> {
        let (session, mut kept, ran) = {
            let store = self.store().lock().await;
            let session = store.session(id).ok().flatten()?;
            let kept = store.session_rail(id, &rail::step_ids(steps)).ok()?;
            let ran = store.hosting(id).ok().flatten().is_some_and(|hosting| hosting.ran);
            (session, kept, ran)
        };
        if record.workflow_step == kept.step {
            return Some(ran);
        }
        let from = kept.step.map(|at| at as usize);
        let to = record.workflow_step.map(|at| at as usize);
        self.step_noted(&session, steps, from, to);
        kept.step = record.workflow_step;
        if let Some(to) = record.workflow_step {
            kept.visits.push((to, self.now().as_str().to_string()));
        }
        let keeping = {
            let mut store = self.store().lock().await;
            // A skip kept since the read above is the person's, and is not written away.
            if let Ok(now) = store.session_rail(id, &rail::step_ids(steps)) {
                kept.skipped = now.skipped;
            }
            store.keep_session_rail(id, &kept, &rail::step_ids(steps))
        };
        if let Err(why) = keeping {
            self.session_warned(id, "the rail was not kept", why);
            return None;
        }
        if !ran {
            return Some(ran);
        }
        let manifest = record.manifest_id.as_ref().map_or("", |manifest| manifest.as_str());
        let workflow = record.workflow.as_deref().unwrap_or("");
        let name = self.workflow_name(manifest, workflow);
        // A step that waited on a lifecycle state and got it wakes the agent with what happened.
        if let Some(left) = from.and_then(|from| steps.get(from)) {
            if let Some(woken) = self.lifecycle_woken(id, left, &name, manifest).await {
                let _ = self.told_by_fleet(id, &woken, ipc::Notice::RailStep).await;
            }
        }
        // A step that asks how the work is delivered puts the question on entry.
        if to.and_then(|at| steps.get(at)).is_some_and(|step| step.asks_delivery) {
            self.delivery_entered(id).await;
        }
        Some(ran)
    }

    fn step_noted(&self, session: &KeptSession, steps: &[RailStep], from: Option<usize>, to: Option<usize>) {
        self.session_noted(
            session,
            TimelineKind::SessionStep,
            format!("step: {} → {}", rail::named(steps, from, "start"), rail::named(steps, to, "done")),
        );
    }

    /// The step a Job that has not run begins on, with the step **kept before the process starts** so
    /// the rail, seeing the Job run, finds it entered and sends nothing. What its first turn carries: for
    /// a step that asks how the work is delivered, the question where `text` names no delivery, or the
    /// choice the start card made. The steps the person skipped are said after it. `None` where there
    /// is nothing to carry.
    pub(crate) async fn first_step_kept(&self, id: &str, hosting: &KeptHosting, text: &str) -> Option<String> {
        let said = self.first_step_said(id, hosting, text).await;
        if hosting.ran {
            return said;
        }
        let workflow = hosting.workflow.as_deref().filter(|workflow| self.workflow_on(&hosting.manifest_id, hosting.project.as_deref(), workflow))?;
        let steps = self.job_rail_steps(id, &hosting.manifest_id, workflow).await?;
        let skipped = self.store().lock().await.session_rail(id, &rail::step_ids(&steps)).map(|kept| kept.skipped).unwrap_or_default();
        let skips = rail::skips_said(&self.workflow_name(&hosting.manifest_id, workflow), &steps, &skipped);
        match (said, skips) {
            (Some(said), Some(skips)) => Some(format!("{said}\n\n{skips}")),
            (said, skips) => said.or(skips),
        }
    }

    async fn first_step_said(&self, id: &str, hosting: &KeptHosting, text: &str) -> Option<String> {
        if hosting.ran {
            return None;
        }
        let workflow = hosting.workflow.as_deref().filter(|workflow| self.workflow_on(&hosting.manifest_id, hosting.project.as_deref(), workflow))?;
        let steps = self.job_rail_steps(id, &hosting.manifest_id, workflow).await?;
        let (session, from, to) = {
            let mut store = self.store().lock().await;
            let session = store.session(id).ok().flatten()?;
            let to = self.ledger_row(&store, &session).ok()?.workflow_step?;
            steps.get(to as usize)?;
            let mut kept = store.session_rail(id, &rail::step_ids(&steps)).ok()?;
            let from = kept.step;
            if from != Some(to) {
                kept.step = Some(to);
                kept.visits.push((to, self.now().as_str().to_string()));
                if let Err(why) = store.keep_session_rail(id, &kept, &rail::step_ids(&steps)) {
                    self.session_warned(id, "the rail was not kept", why);
                    return None;
                }
            }
            (session, from, to)
        };
        if from != Some(to) {
            self.step_noted(&session, &steps, from.map(|at| at as usize), Some(to as usize));
        }
        let step = &steps[to as usize];
        let rows = self.rows_of(id).await.unwrap_or_default();
        let asked = delivery_asked(&rows);
        if step.asks_delivery && !asked {
            match self.delivery_row_kept(id, text).await {
                DeliveryKept::Asked => return Some(ASKED.to_string()),
                DeliveryKept::BySetting(words) => return Some(words),
                DeliveryKept::Named => {}
            }
        }
        // A landing the start card answered goes with the first turn, as an answer given later goes with its own.
        card_choice(&rows).filter(|_| step.asks_delivery).map(chosen_words)
    }

    pub(crate) fn workflow_name(&self, manifest: &str, workflow: &str) -> String {
        self.repositories()
            .serving(manifest)
            .and_then(|served| {
                served
                    .workflows()
                    .get(&core_model::WorkflowId::carried(core_model::Ulid::carried(workflow)))
                    .map(|found| found.name().to_string())
            })
            .unwrap_or_else(|| workflow.to_string())
    }
}
