//! What the Workflows page asks of Fleet beside a definition's text: each step read out, a copy
//! deleted or reset to Armada's, and a nudge marked not a drift. `docs/concepts/workflow.md`.
//!
//! **A step is read from the parsed file**, resolved the way a Job would freeze it, so the words and
//! what ends the step come from the rules the rail places a Job by (`rail`, `rail_ends`) and are not
//! restated. **A copy is a file in a place Fleet can write**: repository or Kit. Armada's own is carried
//! in the binary and is neither deleted nor reset.

use std::path::Path;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use config::{ResolvedWorkflow, WorkflowFile};
use ipc::{
    Instant, MarkNotADrift, NotADriftMarked, SessionId, StepId, StepNudge, StepReading, WireError, WireValue,
    WorkflowCopy, WorkflowCopyDeleted, WorkflowSaved, WorkflowScope, WorkflowSteps,
};

use crate::authoring::{WORKFLOW_UNFIT as UNFIT, WORKFLOW_UNWRITABLE as UNWRITABLE};
use crate::daemon::Fleet;
use crate::rail::{self, RailStep};
use crate::rail_ends::{ends_when, passed_over_when};
use crate::repositories::{Served, WorkflowNotRemoved};

/// No definition by that id in that place. A 422.
const NO_SUCH_DEFINITION: &str = "fleet.no_such_workflow_definition";
/// The definition is Armada's own, which is carried in the binary. A 422.
const IS_ARMADAS: &str = "fleet.workflow_is_armadas";
/// Armada carries no definition of the id to reset a copy to. A 422.
const NOT_CARRIED: &str = "fleet.workflow_not_carried";
/// Fleet kept no nudge by that id. A 422.
const NO_SUCH_NUDGE: &str = "fleet.no_such_nudge";

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
    /// The refusal for an id with no definition in `source`, or none that runs where `source` is absent.
    pub(crate) fn no_such_definition(&self, workflow_id: &str, source: Option<&str>) -> Refusal {
        let said = match source {
            Some(source) => format!(
                "no `{workflow_id}` is held from {source} in this repository; a definition that \
                 was left out is on `list_left_out_workflows`"
            ),
            None => format!("no workflow `{workflow_id}` runs in this repository"),
        };
        Refusal::Unacceptable(
            WireError::raised(NO_SUCH_DEFINITION, said, self.run_id())
                .with_field("workflow_id", WireValue::Str(workflow_id.to_string())),
        )
    }

    /// One definition step by step, with the nudges that still count against each.
    pub(crate) async fn workflow_steps_read(
        &self,
        served: &Served,
        workflow_id: &str,
        source: Option<&str>,
    ) -> Result<WorkflowSteps, Refusal> {
        let files = served.files();
        let file = files
            .iter()
            .find(|file| {
                file.id().as_str() == workflow_id
                    && match source {
                        Some(source) => file.source().as_wire() == source,
                        None => file.overridden_by().is_none(),
                    }
            })
            .ok_or_else(|| self.no_such_definition(workflow_id, source))?;
        let steps = rail::steps_of(&self.resolved_for_page(served, file)?);
        let place = file.source().as_wire();
        let mut read = Vec::with_capacity(steps.len());
        for at in 0..steps.len() {
            read.push(self.step_reading(&steps, at, workflow_id, place).await);
        }
        Ok(WorkflowSteps {
            workflow_id: ipc::WorkflowId::from(file.id()),
            source: place.to_string(),
            steps: read,
        })
    }

    /// The file as a Job would freeze it. The one that runs is held resolved; one a more specific place
    /// replaced never is, so it is resolved here against this repository's Checks.
    fn resolved_for_page(&self, served: &Served, file: &WorkflowFile) -> Result<ResolvedWorkflow, Refusal> {
        if file.overridden_by().is_none() {
            if let Some(held) = served.workflows().get(file.id()) {
                return Ok(held.clone());
            }
        }
        ResolvedWorkflow::resolve(file.def(), served.manifest()).map_err(|why| {
            let said = why.to_string();
            Refusal::Unacceptable(WireError::raised(UNFIT, said.clone(), self.run_id()).with_field("reason", WireValue::Str(said)))
        })
    }

    async fn step_reading(&self, steps: &[RailStep], at: usize, workflow_id: &str, place: &str) -> StepReading {
        let step = &steps[at];
        let mut nudges = Vec::new();
        for kept in self.step_nudges(workflow_id, place, &step.id).await.into_iter().filter(|one| one.counts()) {
            let job_title = self
                .store()
                .lock()
                .await
                .session(&kept.session_id)
                .ok()
                .flatten()
                .and_then(|session| session.title);
            nudges.push(StepNudge {
                id: kept.id,
                job_id: SessionId::carried(kept.session_id),
                job_title,
                fact: kept.fact,
                at: Instant::carried(kept.at),
            });
        }
        StepReading {
            id: StepId::carried(step.id.clone()),
            label: step.label.clone(),
            instructions: step.instructions.clone(),
            ends_when: ends_when(steps, at).map(str::to_string),
            passed_over_when: passed_over_when(step).map(str::to_string),
            nudges,
        }
    }

    /// Which folder a copy is in, or why it is not one.
    fn copy_scope(&self, copy: &WorkflowCopy) -> Result<WorkflowScope, Refusal> {
        match copy.source.as_str() {
            "repository" => Ok(WorkflowScope::Repository),
            "kit" => Ok(WorkflowScope::Kit),
            "armada" => Err(Refusal::Unacceptable(
                WireError::raised(
                    IS_ARMADAS,
                    format!(
                        "`{}` from Armada is carried in the binary; only a repository's or Kit's copy is changed here",
                        copy.workflow_id.as_str()
                    ),
                    self.run_id(),
                )
                .with_field("workflow_id", WireValue::Str(copy.workflow_id.as_str().to_string())),
            )),
            other => Err(self.no_such_definition(copy.workflow_id.as_str(), Some(other))),
        }
    }

    /// Whether this repository holds the copy: a definition of the id in that place or, where the copy
    /// names its file, one that was left out for not fitting and is at that file in that place.
    fn holds_copy(&self, served: &Served, copy: &WorkflowCopy) -> bool {
        match &copy.file {
            Some(file) => served.left_out().iter().any(|left| left.source == copy.source && &left.file == file),
            None => served
                .files()
                .iter()
                .any(|file| file.id().as_str() == copy.workflow_id.as_str() && file.source().as_wire() == copy.source),
        }
    }

    /// Delete the copy's file, and hold what is left.
    pub(crate) fn delete_workflow_copy_file(
        &self,
        copy: WorkflowCopy,
        served: &Served,
    ) -> Result<WorkflowCopyDeleted, Refusal> {
        let scope = self.copy_scope(&copy)?;
        let id = copy.workflow_id.as_str();
        if !self.holds_copy(served, &copy) {
            return Err(self.no_such_definition(id, Some(&copy.source)));
        }
        let removed = match &copy.file {
            Some(file) => self.locating().remove_workflow_file(Path::new(served.root()), scope, Path::new(file)),
            None => self.locating().remove_workflow(Path::new(served.root()), scope, id),
        };
        let file = removed
            .map_err(|why| match why {
                WorkflowNotRemoved::NotHere => self.no_such_definition(id, Some(&copy.source)),
                WorkflowNotRemoved::Unwritable { file, cause } => Refusal::Fault(
                    WireError::raised(UNWRITABLE, format!("{file} could not be deleted: {cause}"), self.run_id())
                        .with_field("file", WireValue::Str(file)),
                ),
            })?;
        self.read_workflows_again(served);
        let runs_from = served
            .workflows()
            .iter()
            .find(|(held, _)| held.as_str() == id)
            .map(|(_, workflow)| workflow.source().as_wire().to_string());
        Ok(WorkflowCopyDeleted { workflow_id: copy.workflow_id, source: copy.source, file, runs_from })
    }

    /// Rewrite the copy's file with Armada's definition of the id, checked as a save is.
    pub(crate) fn reset_workflow_copy_file(&self, copy: WorkflowCopy, served: &Served) -> Result<WorkflowSaved, Refusal> {
        let scope = self.copy_scope(&copy)?;
        let id = copy.workflow_id.as_str();
        if !self.holds_copy(served, &copy) {
            return Err(self.no_such_definition(id, Some(&copy.source)));
        }
        let files = served.files();
        let Some(armadas) = files.iter().find(|file| file.id().as_str() == id && file.source().as_wire() == "armada") else {
            return Err(Refusal::Unacceptable(
                WireError::raised(
                    NOT_CARRIED,
                    format!("Armada carries no `{id}`, so the {} copy has nothing to be reset to", copy.source),
                    self.run_id(),
                )
                .with_field("workflow_id", WireValue::Str(id.to_string())),
            ));
        };
        let save = ipc::SaveWorkflow { scope, definition: armadas.text().to_string(), overwrite: true };
        if let Some(left_out) = &copy.file {
            // **Checked before the copy goes**, so Armada's not fitting here leaves what was there.
            self.check_workflow_file(&save, served)?;
            self.locating()
                .remove_workflow_file(Path::new(served.root()), scope, Path::new(left_out))
                .map_err(|why| match why {
                    WorkflowNotRemoved::NotHere => self.no_such_definition(id, Some(&copy.source)),
                    WorkflowNotRemoved::Unwritable { file, cause } => Refusal::Fault(
                        WireError::raised(UNWRITABLE, format!("{file} could not be deleted: {cause}"), self.run_id())
                            .with_field("file", WireValue::Str(file)),
                    ),
                })?;
        }
        self.save_workflow_file(save, served)
    }

    /// Mark a nudge not a drift.
    pub(crate) async fn nudge_marked(&self, marked: MarkNotADrift) -> Result<NotADriftMarked, Refusal> {
        if self.nudge_not_a_drift(marked.nudge).await {
            return Ok(NotADriftMarked { nudge: marked.nudge });
        }
        Err(Refusal::Unacceptable(
            WireError::raised(NO_SUCH_NUDGE, format!("no nudge {} is kept", marked.nudge), self.run_id())
                .with_field("nudge", WireValue::Str(marked.nudge.to_string())),
        ))
    }
}
