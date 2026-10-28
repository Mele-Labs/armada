//! Saving a workflow, as the fake answers it. The scope and the overwrite flag
//! are echoed so a route test can tell the body arrived. Nothing is written:
//! what a save checks and writes is `armada::authoring`'s, against real files.

use std::sync::Arc;

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

    async fn delete_workflow_copy(
        &self,
        copy: ipc::WorkflowCopy,
        _manifest_id: Option<ManifestId>,
    ) -> Result<ipc::WorkflowCopyDeleted, Refusal> {
        Ok(ipc::WorkflowCopyDeleted {
            workflow_id: copy.workflow_id,
            source: copy.source,
            file: String::from("bug.json"),
            runs_from: None,
        })
    }

    async fn reset_workflow_copy(
        &self,
        copy: ipc::WorkflowCopy,
        _manifest_id: Option<ManifestId>,
    ) -> Result<WorkflowSaved, Refusal> {
        Ok(WorkflowSaved {
            workflow_id: copy.workflow_id,
            scope: ipc::WorkflowScope::Kit,
            file: String::from("bug.json"),
            replaced: true,
            runs_from: None,
        })
    }

    async fn mark_not_a_drift(
        &self,
        marked: ipc::MarkNotADrift,
    ) -> Result<ipc::NotADriftMarked, Refusal> {
        Ok(ipc::NotADriftMarked { nudge: marked.nudge })
    }

    async fn ask_workflow_change(
        self: Arc<Self>,
        asked: ipc::AskWorkflowChange,
        _manifest_id: Option<ManifestId>,
    ) -> Result<ipc::WorkflowChange, Refusal> {
        Ok(drafting(asked.workflow_id, asked.source, ipc::ChangeOrigin::Asked { text: asked.text }, None))
    }

    async fn suggest_workflow_change(
        self: Arc<Self>,
        asked: ipc::SuggestWorkflowChange,
        _manifest_id: Option<ManifestId>,
    ) -> Result<ipc::WorkflowChange, Refusal> {
        Ok(drafting(asked.workflow_id, asked.source, ipc::ChangeOrigin::Drift { nudges: vec![1] }, Some(asked.step_id)))
    }

    async fn list_workflow_changes(
        &self,
        _workflow_id: WorkflowId,
        _source: String,
        _manifest_id: Option<ManifestId>,
    ) -> Result<ipc::WorkflowChanges, Refusal> {
        Ok(ipc::WorkflowChanges { changes: Vec::new() })
    }

    async fn approve_workflow_change(&self, change: ipc::WorkflowChangeRef) -> Result<ipc::WorkflowChangeApproved, Refusal> {
        let mut approved = drafting(WorkflowId::carried("01WF"), String::from("kit"), ipc::ChangeOrigin::Drift { nudges: Vec::new() }, None);
        approved.id = change.change;
        approved.state = ipc::ChangeState::Approved;
        Ok(ipc::WorkflowChangeApproved {
            change: approved,
            saved: WorkflowSaved {
                workflow_id: WorkflowId::carried("01WF"),
                scope: ipc::WorkflowScope::Kit,
                file: String::from("bug.json"),
                replaced: true,
                runs_from: None,
            },
        })
    }

    async fn dismiss_workflow_change(&self, change: ipc::WorkflowChangeRef) -> Result<ipc::WorkflowChange, Refusal> {
        let mut dismissed = drafting(WorkflowId::carried("01WF"), String::from("kit"), ipc::ChangeOrigin::Drift { nudges: Vec::new() }, None);
        dismissed.id = change.change;
        dismissed.state = ipc::ChangeState::Dismissed;
        Ok(dismissed)
    }

    async fn send_back_workflow_change(
        self: Arc<Self>,
        back: ipc::SendBackWorkflowChange,
    ) -> Result<ipc::WorkflowChange, Refusal> {
        let mut again = drafting(WorkflowId::carried("01WF"), String::from("kit"), ipc::ChangeOrigin::Drift { nudges: Vec::new() }, None);
        again.id = back.change;
        again.notes = Some(back.notes);
        Ok(again)
    }

    async fn propose_workflow_to_armada(
        self: Arc<Self>,
        proposed: ipc::ProposeToArmada,
    ) -> Result<ipc::ArmadaProposed, Refusal> {
        Ok(ipc::ArmadaProposed {
            workflow_id: proposed.workflow_id,
            session_id: ipc::SessionId::carried("session"),
            send_failed: None,
        })
    }






}

impl FakeDaemon {
}

/// A change as it stands when begun.
fn drafting(
    workflow_id: WorkflowId,
    source: String,
    origin: ipc::ChangeOrigin,
    step_id: Option<ipc::StepId>,
) -> ipc::WorkflowChange {
    ipc::WorkflowChange {
        id: 1,
        workflow_id,
        source,
        step_id,
        origin,
        before: String::from("{}"),
        after: None,
        state: ipc::ChangeState::Drafting,
        reason: None,
        notes: None,
        at: ipc::Instant::carried("2026-10-27T09:00:00.000Z"),
    }
}
