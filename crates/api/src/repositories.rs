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

/// What one repository runs at each moment of a Job, and the files it left out.
pub(crate) async fn list_triggers<D: Queries>(
    State(served): State<Served<D>>,
    Query(scope): Query<InManifest>,
) -> Response {
    match served.daemon().list_triggers(scope.manifest()).await {
        Ok(listed) => answer(StatusCode::OK, &listed, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// `?when=`, `?step=`, `?name=` and `?source=` on `get_trigger`, beside
/// `?manifest_id=`.
#[derive(serde::Deserialize)]
pub(crate) struct OneTrigger {
    when: ipc::TriggerMoment,
    #[serde(default)]
    step: Option<String>,
    name: String,
    #[serde(default)]
    source: Option<ipc::TriggerLevel>,
    #[serde(default)]
    manifest_id: Option<String>,
}

/// One Trigger as its file holds it. A 422 where this repository holds none.
pub(crate) async fn get_trigger<D: Queries>(
    State(served): State<Served<D>>,
    Query(asked): Query<OneTrigger>,
) -> Response {
    match served
        .daemon()
        .get_trigger(
            asked.when,
            asked.step.map(ipc::StepId::carried),
            asked.name,
            asked.source,
            asked.manifest_id.map(ipc::ManifestId::carried),
        )
        .await
    {
        Ok(found) => answer(StatusCode::OK, &found, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Check one Trigger and write it in the scope named. **200 and not 201**, for
/// `save_workflow`'s reason.
pub(crate) async fn save_trigger<D: Authoring>(
    State(served): State<Served<D>>,
    Query(scope): Query<InManifest>,
    body: Bytes,
) -> Response {
    let asked: ipc::SaveTrigger = match ipc::decode_body("a Trigger to save", &body) {
        Ok(asked) => asked,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().save_trigger(asked, scope.manifest()).await {
        Ok(saved) => answer(StatusCode::OK, &saved, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Delete the file that holds one Trigger in a scope.
pub(crate) async fn remove_trigger<D: Authoring>(
    State(served): State<Served<D>>,
    Query(scope): Query<InManifest>,
    body: Bytes,
) -> Response {
    let asked: ipc::RemoveTrigger = match ipc::decode_body("a Trigger to remove", &body) {
        Ok(asked) => asked,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served
        .daemon()
        .remove_trigger(asked, scope.manifest())
        .await
    {
        Ok(removed) => answer(StatusCode::OK, &removed, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}



