//! Saving a workflow, as the fake answers it. The scope and the overwrite flag
//! are echoed so a route test can tell the body arrived. Nothing is written:
//! what a save checks and writes is `armada::authoring`'s, against real files.

use ipc::{ManifestId, SaveWorkflow, WorkflowId, WorkflowSaved};

use super::FakeDaemon;
use crate::{Authoring, Refusal};

impl Authoring for FakeDaemon {
    async fn save_workflow(
        &self,
        save: SaveWorkflow,
        _manifest_id: Option<ManifestId>,
    ) -> Result<WorkflowSaved, Refusal> {
        Ok(WorkflowSaved {
            workflow_id: WorkflowId::carried("01WF"),
            scope: save.scope,
            file: String::from("bug.json"),
            replaced: save.overwrite,
            runs_from: None,
        })
    }

    async fn save_trigger(
        &self,
        save: ipc::SaveTrigger,
        _manifest_id: Option<ManifestId>,
    ) -> Result<ipc::TriggerSaved, Refusal> {
        Ok(ipc::TriggerSaved {
            name: String::from("tidy"),
            when: ipc::TriggerMoment::StepPasses,
            step: None,
            scope: save.scope,
            file: String::from("tidy.yml"),
            replaced: save.overwrite,
            runs_from: None,
            waits_for_main: false,
            skipped: None,
        })
    }

    async fn remove_trigger(
        &self,
        remove: ipc::RemoveTrigger,
        _manifest_id: Option<ManifestId>,
    ) -> Result<ipc::TriggerRemoved, Refusal> {
        Ok(ipc::TriggerRemoved {
            scope: remove.scope,
            file: String::from("tidy.yml"),
            runs_from: None,
            waits_for_main: false,
        })
    }

    async fn add_job_step(
        &self,
        _job_id: ipc::JobId,
        add: ipc::AddStep,
    ) -> Result<ipc::AddedStep, Refusal> {
        Ok(ipc::AddedStep {
            id: String::from("a1"),
            runs: add.runs,
            when: add.when,
            step: add.step,
            block: add.block,
            repair: add.repair,
            placed: ipc::AddedPlaced::Running,
            added_at: ipc::Instant::carried("2026-10-07T10:00:00.000Z"),
            state: ipc::TriggerFiringState::Pending,
            skipped: None,
            exit_code: None,
            started_at: None,
            ended_at: None,
            log_at: None,
            kept: None,
        })
    }

    async fn remove_job_step(
        &self,
        _job_id: ipc::JobId,
        remove: ipc::RemoveAddedStep,
    ) -> Result<ipc::AddedStepRemoved, Refusal> {
        Ok(ipc::AddedStepRemoved { id: remove.id })
    }
}
