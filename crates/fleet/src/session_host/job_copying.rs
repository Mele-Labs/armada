//! A Job's own copy of its workflow, lifecycles and rules, and the changes made to it. Protocol
//! `docs/practices/protocol.md`, *A Job's own copy*; `docs/concepts/workflow.md`, *Changing one Job*.
//!
//! **A change applies to the one Job and to nothing else.** The files in the repository and in Kit, and every
//! other Job's copy, stay as they are. The Job's agent makes a change with a tool, and the owner makes the same
//! one from the Job's lanes; both come here, and the timeline and the lanes say who.
//!
//! **A workflow is copied at the first change to its steps**, as the file the Job followed read then, so the
//! Job keeps following that text while the repository's changes. Lifecycles and rules are the copy a Job froze
//! when it started (`lifecycles::copying`). Proposing a change for every Job is `keep_job_workflow`, and it
//! waits on the owner's Approve like any other drafted change.

use std::path::Path;
use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use config::{EditFault, NewStep, ResolvedWorkflow, StepEdit};
use core_model::{Ulid, WorkflowId};
use ipc::{
    ChangeJobCopy, ChangeSessionSteps, ChangedBy, CopyPieceKind, JobCopy, JobCopyChange, JobCopyPiece, JobCopyStep,
    KeepJobWorkflow, ManifestId, SaveWorkflow, SessionId, StepChange, TimelineKind, WorkflowChange, WorkflowScope,
};
use store::{KeptCopyMark, KeptHosting, KeptPiece, KeptRail, KeptSession, KeptWorkflowCopy, PieceKind};

use super::proposing::{WORKFLOWS_OFF, WORKFLOW_NOT_HELD};
use super::serving::SESSION_COMMAND_REFUSED;
use crate::daemon::Fleet;
use crate::rail::{self, RailStep};
use crate::repositories::Served;

/// A call through the door that no Job holds the connection of, or a request that names none. A 422.
pub(crate) const JOB_COPY_UNPLACED: &str = "fleet.job_copy_unplaced";
/// An edit the loader refuses, or a check the repository would refuse the result for. A 422.
pub(crate) const JOB_WORKFLOW_UNFIT: &str = "fleet.job_workflow_unfit";
/// A step that is not there, one already there, a blank label, or the step that delivers. A 422.
pub(crate) const JOB_COPY_STEP_REFUSED: &str = "fleet.job_copy_step_refused";
/// A workflow Fleet holds no file for, so there is no text to copy. A 422.
pub(crate) const WORKFLOW_NOT_COPYABLE: &str = "fleet.workflow_not_copyable";
/// A lifecycle or a rule the copy cannot take: it does not read, it is not the id named, or it is not there to
/// drop. A 422.
pub(crate) const JOB_COPY_PIECE_REFUSED: &str = "fleet.job_copy_piece_refused";
/// A Job that froze no copy of its lifecycles and rules, which is a Job that began before copies existed and a
/// Job of no repository. A 422.
pub(crate) const JOB_HOLDS_NO_COPY: &str = "fleet.job_holds_no_copy";

/// The lane a step's mark is on.
pub(crate) const WORKFLOW_LANE: &str = "workflow";
/// The lane a rule's mark is on.
pub(crate) const RULE_LANE: &str = "rule";

/// How a change is kept in a mark, and read back.
pub(crate) fn who_kept(by: ChangedBy) -> &'static str {
    match by {
        ChangedBy::Job => "job",
        ChangedBy::You => "you",
    }
}

pub(crate) fn who_read(kept: &str) -> ChangedBy {
    if kept == "job" {
        ChangedBy::Job
    } else {
        ChangedBy::You
    }
}

/// Who made a change, as a person reads it.
fn who_said(by: ChangedBy) -> &'static str {
    match by {
        ChangedBy::Job => "the Job",
        ChangedBy::You => "you",
    }
}

/// A mark as the wire carries it.
pub(crate) fn mark_wire(kept: &KeptCopyMark) -> JobCopyChange {
    JobCopyChange {
        lane: kept.lane.clone(),
        item: kept.item.clone(),
        text: kept.text.clone(),
        by: who_read(&kept.by),
        at: ipc::Instant::carried(&kept.at),
    }
}

/// Where each step of `old` stands in `new`, by the id it declares. Absent for a step the edit took out.
fn moved_to(old: &[RailStep], new: &[RailStep]) -> impl Fn(u32) -> Option<u32> {
    let places: Vec<Option<u32>> =
        old.iter().map(|step| new.iter().position(|one| one.id == step.id).map(|at| at as u32)).collect();
    move |at| places.get(at as usize).copied().flatten()
}

/// What the rail kept by step index, kept against the same steps after an edit moved them. **A step is
/// followed by the id it declares**, so a skip stays on the step it was for when another is added before it.
pub(crate) fn rail_remapped(rail: &mut KeptRail, old: &[RailStep], new: &[RailStep]) {
    let to = moved_to(old, new);
    rail.step = rail.step.and_then(&to);
    let mapped = |list: &mut Vec<u32>| {
        *list = list.iter().filter_map(|at| to(*at)).collect();
        list.sort_unstable();
        list.dedup();
    };
    mapped(&mut rail.nudged);
    mapped(&mut rail.judged);
    mapped(&mut rail.stalled);
    mapped(&mut rail.skipped);
    rail.visits = rail.visits.iter().filter_map(|(at, since)| Some((to(*at)?, since.clone()))).collect();
}

fn edit_of(change: &StepChange) -> StepEdit {
    match change {
        StepChange::Add { after, id, label, about, instructions, ends_on } => StepEdit::Add {
            after: after.clone(),
            step: NewStep {
                id: id.clone(),
                label: label.clone(),
                about: about.clone(),
                instructions: instructions.clone(),
                ends_on: ends_on.clone(),
            },
        },
        StepChange::Move { step, after } => StepEdit::Move { step: step.clone(), after: after.clone() },
        StepChange::Reword { step, label, about, instructions } => StepEdit::Reword {
            step: step.clone(),
            label: label.clone(),
            about: about.clone(),
            instructions: instructions.clone(),
        },
        StepChange::Drop { step } => StepEdit::Drop { step: step.clone() },
    }
}

/// The id a piece file declares, or the loader's sentence for one that does not read.
fn declared_id(kind: PieceKind, path: &str, text: &str) -> Result<String, String> {
    match kind {
        PieceKind::Lifecycle => config::parse_lifecycle(Path::new(path), text).map(|one| one.id().as_str().to_string()).map_err(|why| why.to_string()),
        PieceKind::Rule => config::parse_rule(Path::new(path), text).map(|one| one.id().as_str().to_string()).map_err(|why| why.to_string()),
    }
}

/// The steps of a copy as `get_job_copy` answers them.
fn steps_wire(steps: &[RailStep], skipped: &[u32]) -> Vec<JobCopyStep> {
    steps
        .iter()
        .enumerate()
        .map(|(at, step)| JobCopyStep {
            id: step.id.clone(),
            label: step.label.clone(),
            about: step.about.clone(),
            instructions: step.instructions.clone(),
            ends_on: step.ends_on.iter().map(|end| end.as_wire().into_owned()).collect(),
            skipped: skipped.contains(&(at as u32)),
        })
        .collect()
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
    /// The Job a call is about and who is making the change. **Through the door the Job is the one whose
    /// process holds the connection**, whatever the body names; every other door is the person's, and names
    /// the Job.
    pub(crate) fn copy_placed(&self, named: Option<&SessionId>) -> Result<(String, ChangedBy), Refusal> {
        if api::via().is_some_and(|via| via.as_wire() == "door") {
            return api::asking()
                .and_then(|caller| self.session_holding(&caller))
                .map(|job| (job, ChangedBy::Job))
                .ok_or_else(|| {
                    self.hosted_refusal(JOB_COPY_UNPLACED, "a Job changes its own copy, and nothing places one on this connection")
                });
        }
        named
            .map(|id| id.as_str().trim().to_string())
            .filter(|id| !id.is_empty())
            .map(|id| (id, ChangedBy::You))
            .ok_or_else(|| self.hosted_refusal(JOB_COPY_UNPLACED, "the request names no Job"))
    }

    /// The hosted Job a copy is changed on. A terminal Job has no workflow, and one that ended is closed.
    async fn copy_hosted(&self, id: &str) -> Result<(KeptSession, KeptHosting), Refusal> {
        if self.terminal_session(id).await?.is_some() {
            return Err(self.hosted_refusal(
                SESSION_COMMAND_REFUSED,
                &format!("job {id} is a terminal job, and only a hosted one has a copy of its workflow"),
            ));
        }
        let (session, hosting) = self.session_and_hosting(id).await?;
        if session.state == store::SessionState::Ended {
            return Err(self.closed(id));
        }
        Ok((session, hosting))
    }

    /// A workflow's text read as a repository would load it. The sentence is the loader's own.
    fn text_resolved(&self, served: &Served, text: &str) -> Result<ResolvedWorkflow, String> {
        let roster = config::Roster::of(self.models_on());
        let def = config::WorkflowDef::parse(Path::new("workflow.json"), text, &roster).map_err(|why| why.to_string())?;
        ResolvedWorkflow::resolve(&def, served.manifest()).map_err(|why| why.to_string())
    }

    /// The workflow the Job's copy holds, read against its repository. Absent where the repository is not
    /// served or the copy no longer reads.
    pub(crate) fn own_resolved(&self, manifest: &str, own: &KeptWorkflowCopy) -> Option<ResolvedWorkflow> {
        let served = self.repositories().serving(manifest)?;
        self.text_resolved(&served, &own.text).ok()
    }

    /// The steps `workflow` has for a Job: those of its own copy where it holds one of this workflow, else the
    /// repository's. **Every reader of a Job's steps asks here or at [`Self::job_rail_steps`].**
    pub(crate) fn steps_with(&self, manifest: &str, workflow: &str, own: Option<&KeptWorkflowCopy>) -> Option<Vec<RailStep>> {
        own.filter(|own| own.workflow_id == workflow)
            .and_then(|own| self.own_resolved(manifest, own))
            .map(|found| rail::steps_of(&found))
            .or_else(|| self.rail_steps(manifest, workflow))
    }

    /// [`Self::steps_with`] for `session`, reading its copy. **Never call it with the store held.**
    pub(crate) async fn job_rail_steps(&self, session: &str, manifest: &str, workflow: &str) -> Option<Vec<RailStep>> {
        let own = self.store().lock().await.job_workflow_copy(session).ok().flatten();
        self.steps_with(manifest, workflow, own.as_ref())
    }

    /// The file `workflow` is read from for a repository: the winning one, which a Job's copy starts as.
    fn workflow_file(&self, served: &Served, workflow: &str) -> Result<(String, String), Refusal> {
        let held = served.workflows();
        let source = held
            .get(&WorkflowId::carried(Ulid::carried(workflow)))
            .map(|found| found.source().as_wire().to_string())
            .ok_or_else(|| self.hosted_refusal(WORKFLOW_NOT_HELD, &format!("this repository holds no workflow named `{workflow}`")))?;
        served
            .files()
            .iter()
            .find(|file| file.id().as_str() == workflow && file.source().as_wire() == source)
            .map(|file| (file.text().to_string(), file.path().to_string_lossy().to_string()))
            .ok_or_else(|| {
                self.hosted_refusal(WORKFLOW_NOT_COPYABLE, &format!("Fleet holds no file for `{workflow}` to copy for this Job"))
            })
    }

    /// The text a step edit is made to: the Job's copy, else the file it follows now.
    async fn copy_base(&self, id: &str, served: &Served, workflow: &str) -> Result<KeptWorkflowCopy, Refusal> {
        let own = self.store().lock().await.job_workflow_copy(id).map_err(|why| self.ledger_fault(why))?;
        if let Some(own) = own.filter(|own| own.workflow_id == workflow) {
            return Ok(own);
        }
        let (text, path) = self.workflow_file(served, workflow)?;
        Ok(KeptWorkflowCopy { workflow_id: workflow.to_string(), path, text })
    }

    fn step_refused(&self, fault: EditFault) -> Refusal {
        match fault {
            EditFault::NotAWorkflow(_) => self.hosted_refusal(JOB_WORKFLOW_UNFIT, &fault.to_string()),
            _ => self.hosted_refusal(JOB_COPY_STEP_REFUSED, &fault.to_string()),
        }
    }

    /// An entry on the Job's timeline that says who made it.
    pub(crate) fn copy_noted(&self, session: &KeptSession, kind: TimelineKind, says: String, by: ChangedBy) {
        let detail = match by {
            ChangedBy::Job => "by the Job",
            ChangedBy::You => "by you",
        };
        self.session_noted_with(session, kind, says, Some(detail.to_string()));
    }

    /// `change_session_steps`: a step added, moved, reworded or dropped in the Job's copy of its workflow.
    pub(crate) async fn steps_changed(self: &Arc<Self>, change: ChangeSessionSteps) -> Result<JobCopy, Refusal> {
        let (id, by) = self.copy_placed(Some(&change.session_id))?;
        let (session, hosting) = self.copy_hosted(&id).await?;
        let Some(workflow) = hosting.workflow.clone() else {
            return Err(self.hosted_refusal(
                SESSION_COMMAND_REFUSED,
                &format!("job {id} follows no workflow, so it has no steps to change"),
            ));
        };
        if !self.workflow_on(&hosting.manifest_id, &workflow) {
            return Err(self.hosted_refusal(
                WORKFLOWS_OFF,
                "this Job's workflow is switched off in Settings, so its steps are not changed",
            ));
        }
        let served = self.served_named(Some(&ManifestId::carried(&hosting.manifest_id)))?;
        let base = self.copy_base(&id, &served, &workflow).await?;
        let old = self.text_resolved(&served, &base.text).map(|found| rail::steps_of(&found)).map_err(|why| self.hosted_refusal(JOB_WORKFLOW_UNFIT, &why))?;
        let edited = config::edit_steps(&base.text, &edit_of(&change.change)).map_err(|fault| self.step_refused(fault))?;
        let new = self.text_resolved(&served, &edited.text).map(|found| rail::steps_of(&found)).map_err(|why| self.hosted_refusal(JOB_WORKFLOW_UNFIT, &why))?;
        self.check_workflow_file(&SaveWorkflow { scope: WorkflowScope::Repository, definition: edited.text.clone(), overwrite: true }, &served)
            .map_err(|refusal| self.hosted_refusal(JOB_WORKFLOW_UNFIT, &refusal.error().message.clone()))?;
        let label_of = |steps: &[RailStep], step: &str| steps.iter().find(|one| one.id == step).map(|one| one.label.clone()).unwrap_or_else(|| step.to_string());
        let after_said = |after: &Option<String>| after.as_deref().map_or_else(|| String::from("first"), |after| format!("after {}", label_of(&new, after)));
        let who = who_said(by);
        let (noted, mark) = match &change.change {
            StepChange::Add { after, .. } => (
                format!("step added: {}", label_of(&new, &edited.step)),
                format!("Added {} by {who}", after_said(after)),
            ),
            StepChange::Move { after, .. } => (
                format!("step moved: {}", label_of(&new, &edited.step)),
                format!("Moved {} by {who}", after_said(after)),
            ),
            StepChange::Reword { .. } => (format!("step reworded: {}", label_of(&new, &edited.step)), format!("Reworded by {who}")),
            StepChange::Drop { .. } => (format!("step dropped: {}", label_of(&old, &edited.step)), format!("Dropped by {who}")),
        };
        let at = self.now().as_str().to_string();
        {
            let mut store = self.store().lock().await;
            store
                .keep_job_workflow_copy(&id, &KeptWorkflowCopy { workflow_id: workflow.clone(), path: base.path, text: edited.text })
                .map_err(|why| self.ledger_fault(why))?;
            let mut kept = store.session_rail(&id).map_err(|why| self.ledger_fault(why))?;
            rail_remapped(&mut kept, &old, &new);
            store.keep_session_rail(&id, &kept).map_err(|why| self.ledger_fault(why))?;
            store
                .keep_job_copy_mark(&id, &KeptCopyMark { lane: WORKFLOW_LANE.into(), item: edited.step.clone(), text: mark, by: who_kept(by).into(), at })
                .map_err(|why| self.ledger_fault(why))?;
        }
        self.copy_noted(&session, TimelineKind::SessionCopy, noted.clone(), by);
        // The person's change reaches a Job that has run; the Job's own does not need telling.
        if by == ChangedBy::You && hosting.ran {
            let labels: Vec<&str> = new.iter().map(|step| step.label.as_str()).collect();
            let words = format!(
                "Fleet: the person changed the steps of this Job's own copy of its {} workflow ({noted}). Its steps, in order: {}.",
                self.workflow_name(&hosting.manifest_id, &workflow),
                labels.join(" -> ")
            );
            if let Err(why) = self.told_by_fleet(&id, &words, ipc::Notice::WorkflowChanged).await {
                eprintln!("job {id}: the change to its steps was not sent to the agent: {why:?}");
            }
        }
        self.copy_published(&id).await;
        self.job_copy_read(&id).await
    }

    /// Tell Bridge what changed: the row, which carries the rail and the copy's workflow, and the lanes.
    pub(crate) async fn copy_published(&self, id: &str) {
        let _ = self.published_hosted(id).await;
        self.job_lifecycles_published(id).await;
    }

    /// `change_job_copy`: a lifecycle or a rule put in the Job's copy, or taken out of it.
    pub(crate) async fn piece_changed(self: &Arc<Self>, change: ChangeJobCopy) -> Result<JobCopy, Refusal> {
        let (id, by) = self.copy_placed(Some(&change.session_id))?;
        let (session, _) = self.copy_hosted(&id).await?;
        let (kind, said) = match change.kind {
            CopyPieceKind::Lifecycle => (PieceKind::Lifecycle, "lifecycle"),
            CopyPieceKind::Rule => (PieceKind::Rule, "rule"),
        };
        let mut piece_id = change.id.trim().to_string();
        if piece_id.is_empty() {
            // A file given whole names its own id, so a blank `id` takes it; a drop has no file to ask.
            if let Some(text) = change.text.as_deref().map(str::trim).filter(|text| !text.is_empty()) {
                piece_id = declared_id(kind, &format!("(job copy)/{said}s/new.yml"), text).map_err(|why| self.hosted_refusal(JOB_COPY_PIECE_REFUSED, &why))?;
            }
        }
        if piece_id.is_empty() {
            return Err(self.hosted_refusal(JOB_COPY_PIECE_REFUSED, "the `id` is blank; name the lifecycle or the rule"));
        }
        let held = self
            .store()
            .lock()
            .await
            .job_copy_of(&id)
            .map_err(|why| self.ledger_fault(why))?
            .ok_or_else(|| self.hosted_refusal(JOB_HOLDS_NO_COPY, &format!("job {id} froze no lifecycles or rules, so it has no copy to change")))?;
        let existing = held.pieces.iter().find(|piece| piece.kind == kind && piece.piece_id == piece_id);
        let at = self.now().as_str().to_string();
        let (lane, item) = match kind {
            PieceKind::Lifecycle => (piece_id.clone(), String::new()),
            PieceKind::Rule => (RULE_LANE.to_string(), piece_id.clone()),
        };
        let (noted, mark) = match change.text.as_deref() {
            Some(text) => {
                let text = text.trim();
                if text.is_empty() {
                    return Err(self.hosted_refusal(JOB_COPY_PIECE_REFUSED, &format!("the {said} text is blank; leave `text` out to drop it")));
                }
                let path = existing.map_or_else(|| format!("(job copy)/{said}s/{piece_id}.yml"), |piece| piece.path.clone());
                match declared_id(kind, &path, text) {
                    Ok(declared) if declared == piece_id => {}
                    Ok(declared) => {
                        return Err(self.hosted_refusal(
                            JOB_COPY_PIECE_REFUSED,
                            &format!("the {said} declares the id `{declared}`, and the change names `{piece_id}`"),
                        ))
                    }
                    Err(why) => return Err(self.hosted_refusal(JOB_COPY_PIECE_REFUSED, &why)),
                }
                let kept = KeptPiece { kind, piece_id: piece_id.clone(), path, text: text.to_string() };
                self.store().lock().await.keep_job_copy_piece(&id, &kept).map_err(|why| self.ledger_fault(why))?;
                (
                    format!("{said} {}: {piece_id}", if existing.is_some() { "changed" } else { "added" }),
                    format!("{} by {}", if existing.is_some() { "Changed" } else { "Added" }, who_said(by)),
                )
            }
            None => {
                if existing.is_none() {
                    return Err(self.hosted_refusal(JOB_COPY_PIECE_REFUSED, &format!("the copy holds no {said} named `{piece_id}` to drop")));
                }
                self.store().lock().await.drop_job_copy_piece(&id, kind, &piece_id).map_err(|why| self.ledger_fault(why))?;
                (format!("{said} dropped: {piece_id}"), format!("Dropped by {}", who_said(by)))
            }
        };
        self.store()
            .lock()
            .await
            .keep_job_copy_mark(&id, &KeptCopyMark { lane, item, text: mark, by: who_kept(by).into(), at })
            .map_err(|why| self.ledger_fault(why))?;
        self.copy_noted(&session, TimelineKind::SessionCopy, noted, by);
        self.copy_published(&id).await;
        self.job_copy_read(&id).await
    }

    /// What the Job holds of its workflow, lifecycles and rules, and what was changed in it.
    pub(crate) async fn job_copy_read(&self, id: &str) -> Result<JobCopy, Refusal> {
        let (session, hosting) = self.session_and_hosting(id).await?;
        let (own, pieces, marks, skipped) = {
            let store = self.store().lock().await;
            let fault = |why| self.ledger_fault(why);
            (
                store.job_workflow_copy(id).map_err(fault)?,
                store.job_copy_of(id).map_err(fault)?.map(|kept| kept.pieces).unwrap_or_default(),
                store.job_copy_marks(id).map_err(fault)?,
                store.session_rail(id).map_err(fault)?.skipped,
            )
        };
        let workflow = hosting.workflow.clone();
        let own = own.filter(|own| workflow.as_deref() == Some(own.workflow_id.as_str()));
        let steps = workflow
            .as_deref()
            .and_then(|workflow| self.steps_with(&hosting.manifest_id, workflow, own.as_ref()))
            .map(|steps| steps_wire(&steps, &skipped))
            .unwrap_or_default();
        let piece = |kind| -> Vec<JobCopyPiece> {
            pieces.iter().filter(|one| one.kind == kind).map(|one| JobCopyPiece { id: one.piece_id.clone(), text: one.text.clone() }).collect()
        };
        Ok(JobCopy {
            session_id: SessionId::carried(&session.id),
            workflow: workflow.map(ipc::WorkflowId::carried),
            own: own.is_some(),
            steps,
            lifecycles: piece(PieceKind::Lifecycle),
            rules: piece(PieceKind::Rule),
            changes: marks.iter().map(mark_wire).collect(),
        })
    }

    /// `get_job_copy`: the Job the connection places, else the one named.
    pub(crate) async fn job_copy_asked(&self, named: Option<SessionId>) -> Result<JobCopy, Refusal> {
        let placed = api::asking().and_then(|caller| self.session_holding(&caller));
        let id = placed
            .or_else(|| named.map(|id| id.as_str().trim().to_string()).filter(|id| !id.is_empty()))
            .ok_or_else(|| self.hosted_refusal(JOB_COPY_UNPLACED, "what a Job holds is its own, and nothing places one"))?;
        self.job_copy_read(&id).await
    }

    /// `keep_job_workflow`: the Job's copy of its workflow proposed for every Job, as a change that waits on the
    /// owner's Approve. **Nothing is written until he approves it.**
    pub(crate) async fn job_workflow_kept(self: &Arc<Self>, keep: KeepJobWorkflow) -> Result<WorkflowChange, Refusal> {
        let id = keep.session_id.as_str().trim().to_string();
        let (_, hosting) = self.copy_hosted(&id).await?;
        let own = self.store().lock().await.job_workflow_copy(&id).map_err(|why| self.ledger_fault(why))?;
        let Some(own) = own.filter(|own| hosting.workflow.as_deref() == Some(own.workflow_id.as_str())) else {
            return Err(self.hosted_refusal(
                JOB_HOLDS_NO_COPY,
                &format!("job {id} has not changed its workflow, so there is nothing of its own to keep"),
            ));
        };
        let served = self.served_named(Some(&ManifestId::carried(&hosting.manifest_id)))?;
        let winning = served
            .workflows()
            .get(&WorkflowId::carried(Ulid::carried(own.workflow_id.as_str())))
            .map(|found| found.source().as_wire().to_string())
            .unwrap_or_default();
        // Armada's is read-only, so a change to it is kept as a workflow of the repository's own.
        let source = keep.source.clone().filter(|source| !source.trim().is_empty()).unwrap_or_else(|| match winning.as_str() {
            "armada" | "" => String::from("repository"),
            other => other.to_string(),
        });
        let new_allowed = winning == "armada";
        self.workflow_proposed(&served, Some(&id), &own.workflow_id, &source, &own.text, new_allowed).await
    }
}
