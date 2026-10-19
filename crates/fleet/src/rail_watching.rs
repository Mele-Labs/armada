//! What Fleet does when a Session's step moves or shows a step skipped. `docs/concepts/session.md`,
//! *The rail*. The rules are `rail`'s; this is the part that acts on them.
//!
//! **Three things, none of which blocks.** The move is entered on the timeline as `session.step`; a
//! step skipped draws one short message to the agent, once for each step; and a step left behind that
//! asks the Judge something is read in the background. The step itself is derived while the row is
//! drawn (`Fleet::rail_drawn`), so this loop only reacts to a published row and never decides one.

use std::collections::HashMap;
use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use ipc::{Event, SessionRecord, TimelineKind};
use store::KeptRail;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::daemon::Fleet;
use crate::rail::{self, Facts, RailStep, Skip};
use crate::rail_keeping::rail_read;
use crate::session_host::session_judging::{Question, Reading};

/// Whether a step left behind is read by a judge. **Off since 16 Oct 2026**, the owner's decision:
/// most concerns were useless or too technical, and how a judge reads the work is to be reworked first.
/// The Judge tile still reads on his press.
const BOUNDARY_JUDGES: bool = false;

/// What the loop last acted on for a Session, so a row that moved nothing costs no read.
#[derive(Clone, PartialEq, Eq)]
struct Seen {
    step: Option<u32>,
    skipped: Vec<(usize, bool)>,
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
        if !self.workflows_on() {
            return;
        }
        let (Some(manifest), Some(workflow)) = (record.manifest_id.as_ref(), record.workflow.as_deref()) else {
            return;
        };
        let Some(steps) = self.rail_steps(manifest.as_str(), workflow) else {
            return;
        };
        let facts = Facts::of(&record, Default::default());
        let skips = rail::skipped(&steps, &facts);
        let now = Seen {
            step: record.workflow_step,
            skipped: skips.iter().map(|(at, skip)| (*at, *skip == Skip::Plan)).collect(),
        };
        if seen.get(&id) == Some(&now) {
            return;
        }
        let Some(kept) = self.rail_acted_on(&id, &steps, &record).await else {
            return;
        };
        seen.insert(id.clone(), now);
        // The nudges go after the rail is kept, one message for each step skipped and never twice.
        for (at, skip) in skips {
            if kept.nudged.contains(&(at as u32)) {
                continue;
            }
            let Some(step) = steps.get(at) else { continue };
            let name = self.workflow_name(manifest.as_str(), workflow);
            let sent = self.told_by_fleet(&id, &rail::nudge(&name, step, skip)).await;
            if matches!(sent, Ok(true)) {
                self.rail_nudged(&id, at as u32).await;
            }
        }
    }

    /// Enter the move on the timeline, keep where the rail stands, and start the boundary judge for a
    /// step left behind. Answers the rail as kept.
    async fn rail_acted_on(
        self: &Arc<Self>,
        id: &str,
        steps: &[RailStep],
        record: &SessionRecord,
    ) -> Option<KeptRail> {
        let (session, mut kept, extra) = {
            let store = self.store().lock().await;
            let session = store.session(id).ok().flatten()?;
            let kept = store.session_rail(id).ok()?;
            let extra = rail_read(&store, &session, true).ok()?.extra;
            (session, kept, extra)
        };
        if record.workflow_step == kept.step {
            return Some(kept);
        }
        let facts = Facts::of(record, extra);
        let from = kept.step.map(|at| at as usize);
        let to = record.workflow_step.map(|at| at as usize);
        self.session_noted(
            &session,
            TimelineKind::SessionStep,
            format!("step: {} → {}", rail::named(steps, from, "start"), rail::named(steps, to, "done")),
        );
        let left = if BOUNDARY_JUDGES { rail::left_behind(steps, from, to, facts.landed, &kept.judged) } else { None };
        kept.step = record.workflow_step;
        if let Some(left) = left {
            kept.judged.push(left as u32);
        }
        if let Err(why) = self.store().lock().await.keep_session_rail(id, &kept) {
            self.session_warned(id, "the rail was not kept", why);
            return None;
        }
        if let Some(left) = left {
            let step = &steps[left];
            let question = Question {
                from: Some((left as u32, step.label.clone())),
                label: step.label.clone(),
                criteria: step.criteria.clone(),
            };
            let runtime = self.hosts().of(id);
            let reading = Reading::begin(runtime);
            let _ = self.published_hosted(id).await;
            let fleet = Arc::clone(self);
            let id = id.to_string();
            tokio::spawn(async move {
                fleet.judged(&id, question).await;
                drop(reading);
                let _ = fleet.published_hosted(&id).await;
            });
        }
        // The step's own instructions go once on each entry, so a move back is told again.
        if let Some(step) = to.and_then(|at| steps.get(at)) {
            let manifest = record.manifest_id.as_ref().map_or("", |manifest| manifest.as_str());
            let workflow = record.workflow.as_deref().unwrap_or("");
            if let Some(words) = rail::entered(&self.workflow_name(manifest, workflow), step) {
                let _ = self.told_by_fleet(id, &words).await;
            }
        }
        Some(kept)
    }

    /// Remember that the agent was nudged about step `at`.
    async fn rail_nudged(&self, id: &str, at: u32) {
        let mut store = self.store().lock().await;
        if let Ok(mut kept) = store.session_rail(id) {
            if !kept.nudged.contains(&at) {
                kept.nudged.push(at);
                let _ = store.keep_session_rail(id, &kept);
            }
        }
    }

    fn workflow_name(&self, manifest: &str, workflow: &str) -> String {
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
