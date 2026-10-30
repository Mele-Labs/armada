//! The claim: **a workflow a Job proposes waits on the owner as a draft that Fleet raised itself.**
//! It names its change, offers Approve and Dismiss, and leaves the list when the change is settled
//! from anywhere. `crates/ipc/operations/propose_new_workflow.toml`.

use std::sync::Arc;

use api::HostedSessions;
use ipc::{AnswerWaiting, WaitingItem, WaitingSource, WorkflowChangeRef};

use super::session_host::{rig_fitted, Rig};
use crate::repositories::{Locating, SavedWorkflow};
use crate::tests::repositories::Planted;
use crate::tests::workflow_page::carried_id;

const FRESH: &str = "version: 1\nworkflow_id: fresh\nname: fresh\nsteps: []";

/// A Job holding a proposal: its change kept ready and its call raised.
async fn proposing() -> (Rig, Arc<Planted>, ipc::SessionId, i64) {
    let planted = Arc::new(Planted::nothing());
    let held = Arc::clone(&planted);
    let rig = rig_fitted(move |fittings| fittings.locating = held as Arc<dyn Locating>);
    let id = rig.start().await;
    let change = {
        let at = rig.fleet.now();
        rig.fleet
            .store()
            .lock()
            .await
            .begin_new_workflow_change(rig.manifest.as_str(), "fresh", "kit", FRESH, at.as_str())
            .expect("kept")
    };
    rig.fleet.proposal_waits(id.as_str(), change, "fresh", FRESH, None).await;
    (rig, planted, id, change)
}

async fn waiting(rig: &Rig, id: &ipc::SessionId) -> Vec<WaitingItem> {
    Arc::clone(&rig.fleet).get_session(id.clone()).await.expect("read").session.waiting_for
}

fn proposal(items: &[WaitingItem]) -> Option<&WaitingItem> {
    items.iter().find(|one| one.source == WaitingSource::Draft)
}

fn answering(id: &ipc::SessionId, item: &str, choice: u32) -> AnswerWaiting {
    AnswerWaiting { session_id: id.clone(), item_id: item.to_string(), choice: Some(choice), text: None, mode: None, fixed: None }
}

#[tokio::test]
async fn a_held_proposal_waits_as_a_draft_offering_approve_and_dismiss() {
    let (rig, _, id, _) = proposing().await;
    let items = waiting(&rig, &id).await;
    let item = proposal(&items).unwrap_or_else(|| panic!("nothing waits: {items:?}"));
    assert_eq!(item.text, "Workflow fresh");
    assert_eq!(item.options.iter().map(|one| one.label.as_str()).collect::<Vec<_>>(), ["Approve", "Dismiss"]);
    assert!(item.act.as_ref().is_some_and(|act| act.target == item.id));
}

#[tokio::test]
async fn answering_approve_saves_the_workflow_and_clears_the_item() {
    let (rig, planted, id, change) = proposing().await;
    planted.will_save(Ok(SavedWorkflow { id: carried_id("fresh"), file: "/home/.armada/workflows/fresh.json".into(), replaced: false }));
    let item = proposal(&waiting(&rig, &id).await).unwrap().id.clone();

    Arc::clone(&rig.fleet).answer_waiting(answering(&id, &item, 0)).await.expect("answered");

    assert_eq!(planted.saves(), [ipc::SaveWorkflow { scope: ipc::WorkflowScope::Kit, definition: FRESH.into(), overwrite: false }]);
    assert!(proposal(&waiting(&rig, &id).await).is_none());
    let kept = rig.fleet.store().lock().await.workflow_change(change).unwrap().unwrap();
    assert_eq!(kept.state, store::CHANGE_APPROVED);
}

#[tokio::test]
async fn answering_dismiss_clears_the_item_and_writes_nothing() {
    let (rig, planted, id, change) = proposing().await;
    let item = proposal(&waiting(&rig, &id).await).unwrap().id.clone();

    Arc::clone(&rig.fleet).answer_waiting(answering(&id, &item, 1)).await.expect("answered");

    assert!(planted.saves().is_empty());
    assert!(proposal(&waiting(&rig, &id).await).is_none());
    let kept = rig.fleet.store().lock().await.workflow_change(change).unwrap().unwrap();
    assert_ne!(kept.state, store::CHANGE_READY);
}

#[tokio::test]
async fn a_dismiss_from_the_workflows_page_clears_the_item_too() {
    let (rig, _, id, change) = proposing().await;
    rig.fleet.workflow_change_dismissed(WorkflowChangeRef { change }).await.expect("dismissed");
    assert!(proposal(&waiting(&rig, &id).await).is_none());
}
