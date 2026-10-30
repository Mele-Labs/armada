//! The repositories a Fleet serves, and adding one by folder.

use axum::body::Bytes;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::Response;
use ipc::{AddRepository, CloneRepository, CreateProject};

use crate::answers::{answer, refused, undecodable};
use crate::daemon::{Authoring, Commands, Queries};
use crate::scoped::InManifest;
use crate::served::Served;

pub(crate) async fn list_repositories<D: Queries>(State(served): State<Served<D>>) -> Response {
    match served.daemon().list_repositories().await {
        Ok(listed) => answer(StatusCode::OK, &listed, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// `?q=` on `read_request_repository`: the words typed so far, absent reading as none.
#[derive(serde::Deserialize)]
pub(crate) struct OfRequest {
    #[serde(default)]
    q: String,
}

/// Which served repository a request's words are about. Never refuses.
pub(crate) async fn read_request_repository<D: Queries>(
    State(served): State<Served<D>>,
    Query(OfRequest { q }): Query<OfRequest>,
) -> Response {
    match served.daemon().read_request_repository(q).await {
        Ok(read) => answer(StatusCode::OK, &read, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Each served repository's merge line, where it has one.
pub(crate) async fn get_merge_lines<D: Queries>(State(served): State<Served<D>>) -> Response {
    match served.daemon().get_merge_lines().await {
        Ok(lines) => answer(StatusCode::OK, &lines, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// `?manifest_id=&path=`, both asked for.
#[derive(serde::Deserialize)]
pub(crate) struct CheckoutDiffQuery {
    manifest_id: String,
    path: String,
}

/// One uncommitted file of a held checkout, as a diff: `Queries::checkout_diff`.
pub(crate) async fn checkout_diff<D: Queries>(
    State(served): State<Served<D>>,
    Query(asked): Query<CheckoutDiffQuery>,
) -> Response {
    let ask = ipc::CheckoutDiffAsk { manifest_id: ipc::ManifestId::carried(asked.manifest_id), path: asked.path };
    match served.daemon().checkout_diff(ask).await {
        Ok(diff) => answer(StatusCode::OK, &diff, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// 201: the repository is served when this answers.
pub(crate) async fn add_repository<D: Commands>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let asked: AddRepository = match ipc::decode_body("a folder to serve", &body) {
        Ok(asked) => asked,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().add_repository(asked).await {
        Ok(added) => answer(StatusCode::CREATED, &added, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// 201: cloned and served when this answers, which may be minutes.
pub(crate) async fn clone_repository<D: Commands>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let asked: CloneRepository = match ipc::decode_body("a repository to clone", &body) {
        Ok(asked) => asked,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.shared().clone_repository(asked).await {
        Ok(added) => answer(StatusCode::CREATED, &added, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// 201: the folder is made, started as a repository and served when this answers.
pub(crate) async fn create_project<D: Commands>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let asked: CreateProject = match ipc::decode_body("a project to create", &body) {
        Ok(asked) => asked,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.shared().create_project(asked).await {
        Ok(added) => answer(StatusCode::CREATED, &added, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Check one workflow definition against the repository and write it in the
/// scope named — protocol 23.26. **200 and not 201**, for `save_manifest_file`'s
/// reason: the file is on disk and held when this answers.
pub(crate) async fn save_workflow<D: Authoring>(
    State(served): State<Served<D>>,
    Query(scope): Query<InManifest>,
    body: Bytes,
) -> Response {
    let asked: ipc::SaveWorkflow = match ipc::decode_body("a workflow definition to save", &body) {
        Ok(asked) => asked,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().save_workflow(asked, scope.manifest()).await {
        Ok(saved) => answer(StatusCode::OK, &saved, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// `?workflow_id=` and `?source=` on `get_workflow`, beside `?manifest_id=`.
#[derive(serde::Deserialize)]
pub(crate) struct OneWorkflow {
    workflow_id: String,
    #[serde(default)]
    source: Option<String>,
    #[serde(default)]
    manifest_id: Option<String>,
}

/// One definition as its file holds it. A 422 where this repository holds none
/// by that id and source.
pub(crate) async fn get_workflow<D: Queries>(
    State(served): State<Served<D>>,
    Query(asked): Query<OneWorkflow>,
) -> Response {
    match served
        .daemon()
        .get_workflow(
            ipc::WorkflowId::carried(asked.workflow_id),
            asked.source,
            asked.manifest_id.map(ipc::ManifestId::carried),
        )
        .await
    {
        Ok(found) => answer(StatusCode::OK, &found, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// One definition step by step: the words, what ends each, and the nudges that still count.
/// A 422 where this repository holds none by that id and source.
pub(crate) async fn get_workflow_steps<D: Queries>(
    State(served): State<Served<D>>,
    Query(asked): Query<OneWorkflow>,
) -> Response {
    match served
        .daemon()
        .get_workflow_steps(
            ipc::WorkflowId::carried(asked.workflow_id),
            asked.source,
            asked.manifest_id.map(ipc::ManifestId::carried),
        )
        .await
    {
        Ok(found) => answer(StatusCode::OK, &found, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Delete a repository's or Kit's copy of a workflow.
pub(crate) async fn delete_workflow_copy<D: Authoring>(
    State(served): State<Served<D>>,
    Query(scope): Query<InManifest>,
    body: Bytes,
) -> Response {
    let asked: ipc::WorkflowCopy = match ipc::decode_body("a workflow copy to delete", &body) {
        Ok(asked) => asked,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().delete_workflow_copy(asked, scope.manifest()).await {
        Ok(deleted) => answer(StatusCode::OK, &deleted, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Rewrite a copy with Armada's definition of the same id. **200**, for `save_workflow`'s reason.
pub(crate) async fn reset_workflow_copy<D: Authoring>(
    State(served): State<Served<D>>,
    Query(scope): Query<InManifest>,
    body: Bytes,
) -> Response {
    let asked: ipc::WorkflowCopy = match ipc::decode_body("a workflow copy to reset", &body) {
        Ok(asked) => asked,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().reset_workflow_copy(asked, scope.manifest()).await {
        Ok(saved) => answer(StatusCode::OK, &saved, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Mark one nudge not a drift.
pub(crate) async fn mark_not_a_drift<D: Authoring>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let asked: ipc::MarkNotADrift = match ipc::decode_body("a nudge to mark", &body) {
        Ok(asked) => asked,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().mark_not_a_drift(asked).await {
        Ok(marked) => answer(StatusCode::OK, &marked, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// `?workflow_id=` and `?source=` on `list_workflow_changes`, beside `?manifest_id=`.
#[derive(serde::Deserialize)]
pub(crate) struct ChangesOf {
    workflow_id: String,
    source: String,
    #[serde(default)]
    manifest_id: Option<String>,
}

/// The open changes to one workflow in one place.
pub(crate) async fn list_workflow_changes<D: Authoring>(
    State(served): State<Served<D>>,
    Query(asked): Query<ChangesOf>,
) -> Response {
    match served
        .daemon()
        .list_workflow_changes(
            ipc::WorkflowId::carried(asked.workflow_id),
            asked.source,
            asked.manifest_id.map(ipc::ManifestId::carried),
        )
        .await
    {
        Ok(found) => answer(StatusCode::OK, &found, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Ask an agent to rewrite a workflow as the owner words it. **200**: the change is drafting.
pub(crate) async fn ask_workflow_change<D: Authoring>(
    State(served): State<Served<D>>,
    Query(scope): Query<InManifest>,
    body: Bytes,
) -> Response {
    let asked: ipc::AskWorkflowChange = match ipc::decode_body("a change to ask for", &body) {
        Ok(asked) => asked,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.shared().ask_workflow_change(asked, scope.manifest()).await {
        Ok(change) => answer(StatusCode::OK, &change, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Keep a workflow a Job proposes, ready for the owner to approve.
pub(crate) async fn propose_new_workflow<D: Authoring>(
    State(served): State<Served<D>>,
    Query(scope): Query<InManifest>,
    body: Bytes,
) -> Response {
    let asked: ipc::ProposeNewWorkflow = match ipc::decode_body("a workflow to propose", &body) {
        Ok(asked) => asked,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.shared().propose_new_workflow(asked, scope.manifest()).await {
        Ok(change) => answer(StatusCode::OK, &change, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Keep a Job's proposal of new text for a workflow that exists, ready for the owner to approve.
pub(crate) async fn propose_workflow_change<D: Authoring>(
    State(served): State<Served<D>>,
    Query(scope): Query<InManifest>,
    body: Bytes,
) -> Response {
    let asked: ipc::ProposeWorkflowChange = match ipc::decode_body("a change to propose", &body) {
        Ok(asked) => asked,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.shared().propose_workflow_change(asked, scope.manifest()).await {
        Ok(change) => answer(StatusCode::OK, &change, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Ask for new words for one step.
pub(crate) async fn suggest_workflow_change<D: Authoring>(
    State(served): State<Served<D>>,
    Query(scope): Query<InManifest>,
    body: Bytes,
) -> Response {
    let asked: ipc::SuggestWorkflowChange = match ipc::decode_body("a change to suggest", &body) {
        Ok(asked) => asked,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.shared().suggest_workflow_change(asked, scope.manifest()).await {
        Ok(change) => answer(StatusCode::OK, &change, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Write a ready change.
pub(crate) async fn approve_workflow_change<D: Authoring>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let asked: ipc::WorkflowChangeRef = match ipc::decode_body("a change to approve", &body) {
        Ok(asked) => asked,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().approve_workflow_change(asked).await {
        Ok(approved) => answer(StatusCode::OK, &approved, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Put an open change aside.
pub(crate) async fn dismiss_workflow_change<D: Authoring>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let asked: ipc::WorkflowChangeRef = match ipc::decode_body("a change to dismiss", &body) {
        Ok(asked) => asked,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().dismiss_workflow_change(asked).await {
        Ok(change) => answer(StatusCode::OK, &change, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Send a change back with notes.
pub(crate) async fn send_back_workflow_change<D: Authoring>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let asked: ipc::SendBackWorkflowChange = match ipc::decode_body("a change to send back", &body) {
        Ok(asked) => asked,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.shared().send_back_workflow_change(asked).await {
        Ok(change) => answer(StatusCode::OK, &change, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Start a Job on Armada's own repository that lands a definition.
pub(crate) async fn propose_workflow_to_armada<D: Authoring>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let asked: ipc::ProposeToArmada = match ipc::decode_body("a workflow to propose", &body) {
        Ok(asked) => asked,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.shared().propose_workflow_to_armada(asked).await {
        Ok(proposed) => answer(StatusCode::OK, &proposed, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Start a Job on Armada's own repository that builds what a change lacks.
pub(crate) async fn build_workflow_support<D: Authoring>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let asked: ipc::WorkflowChangeRef = match ipc::decode_body("a change to build support for", &body) {
        Ok(asked) => asked,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.shared().build_workflow_support(asked).await {
        Ok(building) => answer(StatusCode::OK, &building, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// What one repository's catalogue left out of its workflows, each with why — #425.
pub(crate) async fn list_left_out_workflows<D: Queries>(
    State(served): State<Served<D>>,
    Query(scope): Query<InManifest>,
) -> Response {
    match served
        .daemon()
        .list_left_out_workflows(scope.manifest())
        .await
    {
        Ok(left_out) => answer(StatusCode::OK, &left_out, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}



