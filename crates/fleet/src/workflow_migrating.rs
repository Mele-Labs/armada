//! Moving running Jobs to a workflow's new version. Protocol
//! `docs/practices/protocol/protocol-23-97-moving-running-jobs-to-a-new-workflow-version.md`; `docs/concepts/workflow.md`,
//! *Drafts and Publish*.
//!
//! **A publish moves no Job.** A live Job that follows the workflow and holds no copy of its own reads what its repository
//! serves, so a publish would change it under its feet. Before the save, each such Job is pinned: it is kept a copy of the
//! text its repository runs now, marked `pinned`. After the save and the re-read, a Job whose repository runs that same text
//! is unpinned, since nothing moved for it (a project copy can still win there when Kit is published). The person then picks
//! which pinned Jobs move (`migrate_jobs`), and a move drops the pin so the Job reads the published definition.
//!
//! **The rail keeps its place by step id.** Where the step a Job stood on is not in the new steps, the rail goes to the first
//! step after it, in the old order, that the new steps still have, else the new last step.
//!
//! **`save_workflow` and a file edited by hand pin nothing.** Only Publish asks which Jobs move.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use core_model::ScopeId;
use ipc::{
    AffectedJob, AffectedJobs, ChangedBy, ManifestId, MigrateJobs, MigrateResult, MigratedJobs, SessionId, TimelineKind, WireError, WireValue,
    WorkflowId,
};
use store::{DraftKey, KeptHosting, KeptSession, KeptWorkflowCopy, WorkflowPlace};

use crate::authoring::WORKFLOW_PROJECT_UNKNOWN;
use crate::daemon::Fleet;
use crate::rail;
use crate::repositories::Served;
use crate::workflow_drafts::WORKFLOW_KIND;
use crate::workflow_page::PROJECT_UNNAMED;
use crate::workflow_versions::text_hash;

/// A Job a publish pinned: what it was running.
pub(crate) struct Pin {
    pub session: String,
    pub manifest: String,
    pub text: String,
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
    /// The repositories a place reaches: the repository's own for `repository` (`named`), every served one for `kit`, and
    /// those the project holds for `project`.
    pub(crate) fn place_reach(&self, workflow: &str, source: &str, project: Option<&str>, named: &Served) -> Result<Vec<Served>, Refusal> {
        match source {
            "repository" => Ok(vec![named.clone()]),
            "kit" => Ok(self.repositories().served()),
            "project" => {
                let Some(id) = project else {
                    return Err(Refusal::Unacceptable(
                        WireError::raised(
                            PROJECT_UNNAMED,
                            format!("a copy kept for a project is named by the project too, and `{workflow}` names none"),
                            self.run_id(),
                        )
                        .with_field("workflow_id", WireValue::Str(workflow.to_string())),
                    ));
                };
                let held = self.projects_held();
                let Some(found) = ScopeId::new(id).ok().and_then(|scope| held.get(&scope).cloned()) else {
                    return Err(Refusal::NotFound(
                        WireError::raised(WORKFLOW_PROJECT_UNKNOWN, format!("Fleet holds no project {id}, so no copy is kept for it"), self.run_id())
                            .with_field("project", WireValue::Str(id.to_string())),
                    ));
                };
                Ok(self.repositories().served().into_iter().filter(|one| found.holds(one.manifest().id())).collect())
            }
            other => Err(self.no_such_definition(workflow, Some(other))),
        }
    }

    /// The live hosted Jobs that follow `workflow` in one of `reach`'s repositories.
    async fn jobs_following(&self, workflow: &str, reach: &[Served]) -> Result<Vec<(KeptSession, KeptHosting)>, Refusal> {
        let store = self.store().lock().await;
        let hostings = store.hostings().map_err(|why| self.ledger_fault(why))?;
        let mut found = Vec::new();
        for hosting in hostings {
            let in_reach = reach.iter().any(|one| one.manifest().id().as_str() == hosting.manifest_id);
            if hosting.workflow.as_deref() != Some(workflow) || !in_reach {
                continue;
            }
            let session = store.session(&hosting.session_id).map_err(|why| self.ledger_fault(why))?;
            if let Some(session) = session.filter(|session| session.state == store::SessionState::Live) {
                found.push((session, hosting));
            }
        }
        Ok(found)
    }

    /// Keep a pinned copy for each live Job that follows `workflow` in `reach` and holds none, of the text its repository
    /// runs now. **Call before the save**, which changes what the repository runs.
    pub(crate) async fn pins_kept(&self, workflow: &str, reach: &[Served]) -> Result<Vec<Pin>, Refusal> {
        let mut pins = Vec::new();
        for (session, hosting) in self.jobs_following(workflow, reach).await? {
            let Some(served) = reach.iter().find(|one| one.manifest().id().as_str() == hosting.manifest_id) else { continue };
            let Ok((text, path)) = self.workflow_file(served, workflow) else { continue };
            let mut store = self.store().lock().await;
            if store.job_workflow_copy(&session.id).map_err(|why| self.ledger_fault(why))?.is_some() {
                continue;
            }
            let copy = KeptWorkflowCopy { workflow_id: workflow.to_string(), path, text: text.clone(), pinned: true };
            store.keep_job_workflow_copy(&session.id, &copy).map_err(|why| self.ledger_fault(why))?;
            pins.push(Pin { session: session.id, manifest: hosting.manifest_id, text });
        }
        Ok(pins)
    }

    /// Give back the pins a publish made, where the publish did not write.
    pub(crate) async fn pins_dropped(&self, pins: &[Pin]) {
        let mut store = self.store().lock().await;
        for pin in pins {
            if let Err(why) = store.drop_job_workflow_copy(&pin.session) {
                self.session_warned(&pin.session, "the pin of a publish that wrote nothing was not dropped", format!("{why:?}"));
            }
        }
    }

    /// After the save: unpin each Job whose repository runs the text it was pinned to, and answer the Jobs left pinned.
    /// Repositories other than `served` are read again first, since a save re-reads only its own.
    pub(crate) async fn pins_settled(&self, workflow: &str, pins: Vec<Pin>, served: &Served) -> Result<Vec<SessionId>, Refusal> {
        let mut read: Vec<&str> = vec![served.manifest().id().as_str()];
        for pin in &pins {
            if !read.contains(&pin.manifest.as_str()) {
                read.push(&pin.manifest);
                if let Some(other) = self.repositories().serving(&pin.manifest) {
                    self.read_workflows_again(&other);
                }
            }
        }
        let mut left = Vec::new();
        for pin in &pins {
            let running = self.repositories().serving(&pin.manifest).and_then(|one| self.workflow_file(&one, workflow).ok()).map(|(text, _)| text);
            if running.as_deref() == Some(pin.text.as_str()) {
                self.store().lock().await.drop_job_workflow_copy(&pin.session).map_err(|why| self.ledger_fault(why))?;
            } else {
                left.push(SessionId::carried(&pin.session));
            }
        }
        Ok(left)
    }

    /// `get_affected_jobs`: every live Job that follows `workflow` in a repository the place reaches, against the text a
    /// publish there takes: the draft's where one is kept, else what is published.
    pub(crate) async fn affected_read(&self, workflow: &WorkflowId, source: &str, project: Option<&str>, named: &Served) -> Result<AffectedJobs, Refusal> {
        let id = workflow.as_str();
        let reach = self.place_reach(id, source, project, named)?;
        let key = DraftKey {
            kind: WORKFLOW_KIND.into(),
            source: source.into(),
            item_id: id.into(),
            project: project.unwrap_or_default().into(),
            manifest_id: if source == "repository" { named.manifest().id().as_str().to_string() } else { String::new() },
            ..DraftKey::default()
        };
        let (draft, versions) = {
            let store = self.store().lock().await;
            (
                store.workflow_draft(&key).map_err(|why| self.draft_not_kept(why))?,
                store.workflow_versions(&WorkflowPlace::from(&key)).map_err(|why| self.draft_not_kept(why))?,
            )
        };
        let drafted = draft.map(|draft| draft.text().to_string());
        let mut jobs = Vec::new();
        for (session, hosting) in self.jobs_following(id, &reach).await? {
            let Some(served) = reach.iter().find(|one| one.manifest().id().as_str() == hosting.manifest_id) else { continue };
            let (own, stands) = {
                let store = self.store().lock().await;
                let own = store.job_workflow_copy(&session.id).map_err(|why| self.ledger_fault(why))?;
                (own, self.ledger_row(&store, &session)?.workflow_step)
            };
            let target = drafted.clone().or_else(|| self.workflow_published_text(served, id, source, project));
            let target_ids = target
                .and_then(|text| self.text_resolved(served, &text).ok())
                .map(|found| rail::step_ids(&rail::steps_of(&found)));
            let steps = self.steps_with(&hosting.manifest_id, id, own.as_ref());
            let step = stands.zip(steps.as_ref()).and_then(|(at, steps)| steps.get(at as usize));
            // Where there is no step to lose, or no target that reads to lose it from, nothing is lost.
            let survives = match (step, &target_ids) {
                (Some(step), Some(ids)) => ids.contains(&step.id),
                _ => true,
            };
            let on_version = own.as_ref().and_then(|own| {
                let hash = text_hash(&own.text);
                versions.iter().find(|one| one.hash == hash).map(|one| one.version)
            });
            jobs.push(AffectedJob {
                session_id: SessionId::carried(&session.id),
                title: session.title.clone(),
                manifest_id: ManifestId::carried(&hosting.manifest_id),
                step_id: step.map(|step| step.id.clone()),
                step_label: step.map(|step| step.label.clone()),
                survives,
                pinned: own.as_ref().is_some_and(|own| own.pinned),
                edited: own.as_ref().is_some_and(|own| !own.pinned),
                on_version,
            });
        }
        Ok(AffectedJobs { jobs })
    }

    /// `migrate_jobs`: move each Job asked for, one result for each in order. **A refusal is a Job's own**: the others move.
    pub(crate) async fn jobs_migrated(self: &Arc<Self>, migrate: MigrateJobs, named: &Served) -> Result<MigratedJobs, Refusal> {
        let id = migrate.workflow_id.as_str();
        let reach = self.place_reach(id, &migrate.source, migrate.project.as_deref(), named)?;
        let mut results = Vec::new();
        for asked in &migrate.session_ids {
            results.push(self.job_migrated(id, asked, &reach).await?);
        }
        Ok(MigratedJobs { results })
    }

    async fn job_migrated(self: &Arc<Self>, workflow: &str, asked: &SessionId, reach: &[Served]) -> Result<MigrateResult, Refusal> {
        let sid = asked.as_str().to_string();
        let stayed = |code: Option<&str>, message: String| MigrateResult {
            session_id: asked.clone(),
            moved: false,
            step_id: None,
            step_label: None,
            code: code.map(str::to_string),
            message: Some(message),
        };
        let (session, hosting, own) = {
            let store = self.store().lock().await;
            let session = store.session(&sid).map_err(|why| self.ledger_fault(why))?;
            let hosting = store.hosting(&sid).map_err(|why| self.ledger_fault(why))?;
            let own = store.job_workflow_copy(&sid).map_err(|why| self.ledger_fault(why))?;
            (session, hosting, own)
        };
        let (Some(session), Some(hosting)) = (session.filter(|session| session.state == store::SessionState::Live), hosting) else {
            return Ok(stayed(Some("fleet.workflow_migrate_not_live"), format!("job {sid} is not a live job Fleet hosts")));
        };
        if hosting.workflow.as_deref() != Some(workflow) {
            return Ok(stayed(Some("fleet.workflow_migrate_not_following"), format!("job {sid} follows a different workflow than `{workflow}`")));
        }
        if !reach.iter().any(|one| one.manifest().id().as_str() == hosting.manifest_id) {
            return Ok(stayed(Some("fleet.workflow_migrate_not_following"), format!("job {sid} is in a repository this place does not reach")));
        }
        let Some(own) = own.filter(|own| own.workflow_id == workflow) else {
            // Not an error: the Job reads what is published already, which is where a move would have put it.
            return Ok(stayed(None, format!("job {sid} holds no copy, so it already reads the published `{workflow}`")));
        };
        if !own.pinned {
            return Ok(stayed(
                Some("fleet.workflow_migrate_edited"),
                format!("job {sid} edited the steps of its own copy, and a move would lose them"),
            ));
        }
        let Some(new) = self.rail_steps(&hosting.manifest_id, workflow) else {
            return Ok(stayed(Some("fleet.workflow_migrate_not_following"), format!("job {sid}'s repository no longer holds `{workflow}`")));
        };
        let old_ids = self.steps_with(&hosting.manifest_id, workflow, Some(&own)).map(|steps| rail::step_ids(&steps)).unwrap_or_default();
        let new_ids = rail::step_ids(&new);
        {
            let mut store = self.store().lock().await;
            let fault = |why| self.ledger_fault(why);
            let stood = store.session_rail(&sid, &old_ids).map_err(fault)?.step.and_then(|at| old_ids.get(at as usize).cloned());
            store.drop_job_workflow_copy(&sid).map_err(fault)?;
            // Its step is gone: the first step after it that survives, else the last.
            if let Some(gone) = stood.filter(|stood| !new_ids.contains(stood)).and_then(|stood| old_ids.iter().position(|one| *one == stood)) {
                let to = old_ids[gone + 1..].iter().find_map(|one| new_ids.iter().position(|new| new == one)).unwrap_or(new_ids.len().saturating_sub(1));
                let mut kept = store.session_rail(&sid, &new_ids).map_err(fault)?;
                kept.step = Some(to as u32);
                kept.visits.push((to as u32, self.now().as_str().to_string()));
                store.keep_session_rail(&sid, &kept, &new_ids).map_err(fault)?;
            }
        }
        let record = self.published_hosted(&sid).await?;
        let step = record.workflow_step.and_then(|at| new.get(at as usize));
        let name = self.workflow_name(&hosting.manifest_id, workflow);
        self.copy_noted(&session, TimelineKind::SessionWorkflow, format!("workflow: moved to the newest {name}"), ChangedBy::You);
        if hosting.ran {
            let line = match step {
                Some(step) => format!("Fleet: the person moved this Job to the newest {name} workflow; it is on the \"{}\" step.", step.label),
                None => format!("Fleet: the person moved this Job to the newest {name} workflow."),
            };
            // The move stands whether or not the agent was told, as the rail's own step line does.
            let _ = self.told_by_fleet(&sid, &line, ipc::Notice::RailStep).await;
        }
        self.job_lifecycles_published(&sid).await;
        Ok(MigrateResult {
            session_id: asked.clone(),
            moved: true,
            step_id: step.map(|step| step.id.clone()),
            step_label: step.map(|step| step.label.clone()),
            code: None,
            message: None,
        })
    }
}
