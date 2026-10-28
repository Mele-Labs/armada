//! What Fleet does when a Session's step moves or shows a step skipped. `docs/concepts/session.md`,
//! *The rail*. The rules are `rail`'s; this is the part that acts on them.
//!
//! **Three things, none of which blocks.** The move is entered on the timeline as `session.step`; a
//! step skipped draws one short message to the agent, once for each step, and so does a step the Job
//! stopped on that only its own act ends; and a step left behind that
//! asks the Judge something is read in the background. The step itself is derived while the row is
//! drawn (`Fleet::rail_drawn`), so this loop only reacts to a published row and never decides one.

use std::collections::HashMap;
use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use ipc::{Event, SessionRecord, TimelineKind};
use store::{KeptHosting, KeptRail, KeptSession};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::daemon::Fleet;
use crate::rail::{self, Extra, Facts, RailStep, Skip};
use crate::rail_keeping::rail_read;
use crate::session_host::delivering::{card_choice, chosen_words, delivery_asked, ASKED};
use crate::session_host::session_judging::{Question, Reading};

/// Whether a step left behind is read by a judge. **Off since 16 Oct 2026**, the owner's decision:
/// most concerns were useless or too technical, and how a judge reads the work is to be reworked first.
/// The Judge tile still reads on his press.
const BOUNDARY_JUDGES: bool = false;

/// What the loop last acted on for a Session, so a row that moved nothing costs no read.
#[derive(Clone, PartialEq, Eq)]
struct Seen {
    step: Option<u32>,
    skipped: Vec<(usize, Skip)>,
    stalled: Option<usize>,
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
        // The walk the nudge reads is in the thread, so it is read only for a Job that wrote a test.
        let mut facts = Facts::of(&record, Default::default());
        if facts.test_edited && steps.iter().any(|step| step.walks) {
            facts = Facts::of(&record, self.rail_extra(&id).await);
        }
        let skips = rail::skipped(&steps, &facts);
        let stalled = rail::stalled(&steps, &record);
        let now = Seen { step: record.workflow_step, skipped: skips.clone(), stalled: stalled.map(|(at, _)| at) };
        if seen.get(&id) == Some(&now) {
            return;
        }
        let Some((kept, ran)) = self.rail_acted_on(&id, &steps, &record).await else {
            return;
        };
        // A Job that has not run is told nothing, and is looked at again once it has.
        if !ran {
            return;
        }
        seen.insert(id.clone(), now);
        // The nudges go after the rail is kept, one message for each step skipped and never twice.
        for (at, skip) in skips {
            if kept.nudged.contains(&(at as u32)) {
                continue;
            }
            let Some(step) = steps.get(at) else { continue };
            let name = self.workflow_name(manifest.as_str(), workflow);
            let sent = self.told_by_fleet(&id, &rail::nudge(&name, step, skip), ipc::Notice::RailNudge).await;
            if matches!(sent, Ok(true)) {
                self.rail_nudged(&id, at as u32).await;
                if let Some(source) = self.nudge_kept(manifest.as_str(), workflow, step, &id, skip).await {
                    self.drift_suggested(manifest.as_str(), workflow, source, &step.id).await;
                }
            }
        }
        // A Job stopped on a step only its own act ends is told so once for each step.
        if let Some((at, end)) = stalled.filter(|(at, _)| !kept.stalled.contains(&(*at as u32))) {
            let Some(step) = steps.get(at) else { return };
            let name = self.workflow_name(manifest.as_str(), workflow);
            let sent = self.told_by_fleet(&id, &rail::stall_nudge(&name, step, end), ipc::Notice::RailNudge).await;
            if matches!(sent, Ok(true)) {
                self.rail_stalled(&id, at as u32).await;
            }
        }
    }

    /// Enter the move on the timeline, keep where the rail stands, and start the boundary judge for a
    /// step left behind. Answers the rail as kept, and whether the Job has run: **one that has not is
    /// told nothing**, since its first turn carries the step (`first_step_kept`) and a word sent now
    /// would start it ahead of the message it waits for.
    async fn rail_acted_on(
        self: &Arc<Self>,
        id: &str,
        steps: &[RailStep],
        record: &SessionRecord,
    ) -> Option<(KeptRail, bool)> {
        let (session, mut kept, extra, ran) = {
            let store = self.store().lock().await;
            let session = store.session(id).ok().flatten()?;
            let kept = store.session_rail(id).ok()?;
            let extra = rail_read(&store, &session, true).ok()?.extra;
            let ran = store.hosting(id).ok().flatten().is_some_and(|hosting| hosting.ran);
            (session, kept, extra, ran)
        };
        if record.workflow_step == kept.step {
            return Some((kept, ran));
        }
        let facts = Facts::of(record, extra);
        let from = kept.step.map(|at| at as usize);
        let to = record.workflow_step.map(|at| at as usize);
        self.step_noted(&session, steps, from, to);
        let left = if BOUNDARY_JUDGES { rail::left_behind(steps, from, to, facts.landed, &kept.judged) } else { None };
        kept.step = record.workflow_step;
        if let Some(to) = record.workflow_step {
            kept.visits.push((to, self.now().as_str().to_string()));
        }
        if let Some(left) = left {
            kept.judged.push(left as u32);
        }
        let keeping = {
            let mut store = self.store().lock().await;
            // A skip kept since the read above is the person's, and is not written away.
            if let Ok(now) = store.session_rail(id) {
                kept.skipped = now.skipped;
            }
            store.keep_session_rail(id, &kept)
        };
        if let Err(why) = keeping {
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
        if !ran {
            return Some((kept, ran));
        }
        // The step's own instructions go once on each entry, so a move back is told again.
        if let Some((at, step)) = to.and_then(|at| steps.get(at).map(|step| (at, step))) {
            let manifest = record.manifest_id.as_ref().map_or("", |manifest| manifest.as_str());
            let workflow = record.workflow.as_deref().unwrap_or("");
            // A step that asks how the work is delivered holds its instructions until the answer,
            // which sends them with it.
            let held = step.asks_delivery && self.delivery_entered(id).await;
            if let Some(words) = rail::instructed(&self.workflow_name(manifest, workflow), steps, at).filter(|_| !held) {
                let _ = self.told_by_fleet(id, &words, ipc::Notice::RailStep).await;
            }
        }
        Some((kept, ran))
    }

    fn step_noted(&self, session: &KeptSession, steps: &[RailStep], from: Option<usize>, to: Option<usize>) {
        self.session_noted(
            session,
            TimelineKind::SessionStep,
            format!("step: {} → {}", rail::named(steps, from, "start"), rail::named(steps, to, "done")),
        );
    }

    /// The step a Job that has not run begins on, as the words its first turn carries, with the step
    /// **kept before the process starts** so the rail, seeing the Job run, finds it entered and sends
    /// nothing. The step's instructions; or, for a step that asks how the work is delivered, the question
    /// where `text` names no delivery, and the instructions go with the answer. The steps the person
    /// skipped are said after it. `None` where there is nothing to carry.
    pub(crate) async fn first_step_kept(&self, id: &str, hosting: &KeptHosting, text: &str) -> Option<String> {
        let said = self.first_step_said(id, hosting, text).await;
        if hosting.ran || !self.workflows_on() {
            return said;
        }
        let workflow = hosting.workflow.as_deref()?;
        let steps = self.rail_steps(&hosting.manifest_id, workflow)?;
        let skipped = self.store().lock().await.session_rail(id).map(|kept| kept.skipped).unwrap_or_default();
        let skips = rail::skips_said(&self.workflow_name(&hosting.manifest_id, workflow), &steps, &skipped);
        match (said, skips) {
            (Some(said), Some(skips)) => Some(format!("{said}\n\n{skips}")),
            (said, skips) => said.or(skips),
        }
    }

    async fn first_step_said(&self, id: &str, hosting: &KeptHosting, text: &str) -> Option<String> {
        if hosting.ran || !self.workflows_on() {
            return None;
        }
        let workflow = hosting.workflow.as_deref()?;
        let steps = self.rail_steps(&hosting.manifest_id, workflow)?;
        let (session, from, to) = {
            let mut store = self.store().lock().await;
            let session = store.session(id).ok().flatten()?;
            let to = self.ledger_row(&store, &session).ok()?.workflow_step?;
            steps.get(to as usize)?;
            let mut kept = store.session_rail(id).ok()?;
            let from = kept.step;
            if from != Some(to) {
                kept.step = Some(to);
                kept.visits.push((to, self.now().as_str().to_string()));
                if let Err(why) = store.keep_session_rail(id, &kept) {
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
        let words = rail::instructed(&self.workflow_name(&hosting.manifest_id, workflow), &steps, to as usize);
        let rows = self.rows_of(id).await.unwrap_or_default();
        let asked = delivery_asked(&rows);
        if step.asks_delivery && !asked && self.delivery_row_kept(id, text).await {
            return Some(ASKED.to_string());
        }
        // A landing the start card answered goes with the first turn, as an answer given later goes with its own.
        match card_choice(&rows).filter(|_| step.asks_delivery) {
            Some(choice) => Some(match words {
                Some(words) => format!("{}\n\n{words}", chosen_words(choice)),
                None => chosen_words(choice),
            }),
            None => words,
        }
    }

    /// What the rail reads beyond the record, for a Session that follows a workflow.
    async fn rail_extra(&self, id: &str) -> Extra {
        let store = self.store().lock().await;
        let Some(session) = store.session(id).ok().flatten() else { return Extra::default() };
        rail_read(&store, &session, true).map(|read| read.extra).unwrap_or_default()
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

    /// Remember that the agent was told it had stopped on step `at`.
    async fn rail_stalled(&self, id: &str, at: u32) {
        let mut store = self.store().lock().await;
        if let Ok(mut kept) = store.session_rail(id) {
            if !kept.stalled.contains(&at) {
                kept.stalled.push(at);
                let _ = store.keep_session_rail(id, &kept);
            }
        }
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
