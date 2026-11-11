//! His projects over the wire. `docs/practices/protocol/projects.md`.
//!
//! Every route answers his projects and the loose tasks whole, so a handler is a decode and one call.

use axum::body::Bytes;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::Response;
use serde::de::DeserializeOwned;

use crate::answers::{answer, refused, undecodable};
use crate::daemon::{Projects, Refusal};
use crate::scoped::InManifest;
use crate::served::Served;

fn answered<D: Projects>(served: &Served<D>, result: Result<ipc::Projects, Refusal>) -> Response {
    match result {
        Ok(projects) => answer(StatusCode::OK, &projects, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

fn decoded<T: DeserializeOwned, D: Projects>(served: &Served<D>, what: &'static str, body: &Bytes) -> Result<T, Response> {
    ipc::decode_body(what, body).map_err(|why| undecodable(&why.to_string(), served.run_id()))
}

pub(crate) async fn list_projects<D: Projects>(State(served): State<Served<D>>) -> Response {
    let result = served.daemon().list_projects().await;
    answered(&served, result)
}

pub(crate) async fn add_project<D: Projects>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let project = match decoded(&served, "a project to add", &body) {
        Ok(project) => project,
        Err(response) => return response,
    };
    let result = served.daemon().add_project(project).await;
    answered(&served, result)
}

pub(crate) async fn edit_project<D: Projects>(
    State(served): State<Served<D>>,
    Path(project): Path<String>,
    body: Bytes,
) -> Response {
    let edit = match decoded(&served, "a project's edit", &body) {
        Ok(edit) => edit,
        Err(response) => return response,
    };
    let result = served.daemon().edit_project(project, edit).await;
    answered(&served, result)
}

pub(crate) async fn list_holding_projects<D: Projects>(
    State(served): State<Served<D>>,
    Query(scope): Query<InManifest>,
) -> Response {
    let result = served.daemon().list_holding_projects(scope.manifest()).await;
    answered(&served, result)
}

/// `?project=&manifest_id=` on `get_values`, each optional.
#[derive(serde::Deserialize)]
pub(crate) struct ValuesFor {
    #[serde(default)]
    project: Option<String>,
    #[serde(default)]
    manifest_id: Option<String>,
}

pub(crate) async fn get_values<D: Projects>(State(served): State<Served<D>>, Query(asked): Query<ValuesFor>) -> Response {
    let result = served.daemon().get_values(asked.project, asked.manifest_id.map(ipc::ManifestId::carried)).await;
    match result {
        Ok(values) => answer(StatusCode::OK, &values, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// `?refresh=true` on `list_project_issues` asks the tracker again whatever is held.
#[derive(serde::Deserialize)]
pub(crate) struct Refresh {
    #[serde(default)]
    refresh: bool,
}

pub(crate) async fn list_project_issues<D: Projects>(
    State(served): State<Served<D>>,
    Path(project): Path<String>,
    Query(asked): Query<Refresh>,
) -> Response {
    match served.daemon().list_project_issues(project, asked.refresh).await {
        Ok(list) => answer(StatusCode::OK, &list, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn save_kit_values<D: Projects>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let save = match decoded(&served, "Kit's values", &body) {
        Ok(save) => save,
        Err(response) => return response,
    };
    match served.daemon().save_kit_values(save).await {
        Ok(saved) => answer(StatusCode::OK, &saved, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn delete_project<D: Projects>(
    State(served): State<Served<D>>,
    Path(project): Path<String>,
) -> Response {
    let result = served.daemon().delete_project(project).await;
    answered(&served, result)
}

pub(crate) async fn add_project_task<D: Projects>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let task = match decoded(&served, "a project task", &body) {
        Ok(task) => task,
        Err(response) => return response,
    };
    let result = served.daemon().add_project_task(task).await;
    answered(&served, result)
}

pub(crate) async fn edit_project_task<D: Projects>(
    State(served): State<Served<D>>,
    Path(row): Path<String>,
    body: Bytes,
) -> Response {
    let task = match decoded(&served, "a project task", &body) {
        Ok(task) => task,
        Err(response) => return response,
    };
    let result = served.daemon().edit_project_task(row, task).await;
    answered(&served, result)
}

pub(crate) async fn link_project_issue<D: Projects>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let link = match decoded(&served, "an issue to link", &body) {
        Ok(link) => link,
        Err(response) => return response,
    };
    let result = served.daemon().link_project_issue(link).await;
    answered(&served, result)
}

pub(crate) async fn remove_project_row<D: Projects>(
    State(served): State<Served<D>>,
    Path(row): Path<String>,
) -> Response {
    let result = served.daemon().remove_project_row(row).await;
    answered(&served, result)
}

pub(crate) async fn move_project_row<D: Projects>(
    State(served): State<Served<D>>,
    Path(row): Path<String>,
    body: Bytes,
) -> Response {
    let to = match decoded(&served, "where a project row goes", &body) {
        Ok(to) => to,
        Err(response) => return response,
    };
    let result = served.daemon().move_project_row(row, to).await;
    answered(&served, result)
}

pub(crate) async fn move_project_task_repository<D: Projects>(
    State(served): State<Served<D>>,
    Path(row): Path<String>,
    body: Bytes,
) -> Response {
    let to = match decoded(&served, "which repository a project task goes to", &body) {
        Ok(to) => to,
        Err(response) => return response,
    };
    let result = served.daemon().move_project_task_repository(row, to).await;
    answered(&served, result)
}

pub(crate) async fn tick_project_task<D: Projects>(
    State(served): State<Served<D>>,
    Path(row): Path<String>,
    body: Bytes,
) -> Response {
    let tick = match decoded(&served, "a tick", &body) {
        Ok(tick) => tick,
        Err(response) => return response,
    };
    let result = served.daemon().tick_project_task(row, tick).await;
    answered(&served, result)
}

pub(crate) async fn add_project_subtask<D: Projects>(
    State(served): State<Served<D>>,
    Path(row): Path<String>,
    body: Bytes,
) -> Response {
    let task = match decoded(&served, "a project subtask", &body) {
        Ok(task) => task,
        Err(response) => return response,
    };
    let result = served.daemon().add_project_subtask(row, task).await;
    answered(&served, result)
}

pub(crate) async fn link_task_issue<D: Projects>(
    State(served): State<Served<D>>,
    Path(row): Path<String>,
    body: Bytes,
) -> Response {
    let link = match decoded(&served, "an issue to link", &body) {
        Ok(link) => link,
        Err(response) => return response,
    };
    let result = served.daemon().link_task_issue(row, link).await;
    answered(&served, result)
}

pub(crate) async fn unlink_task_issue<D: Projects>(
    State(served): State<Served<D>>,
    Path(row): Path<String>,
    body: Bytes,
) -> Response {
    let link = match decoded(&served, "an issue to unlink", &body) {
        Ok(link) => link,
        Err(response) => return response,
    };
    let result = served.daemon().unlink_task_issue(row, link).await;
    answered(&served, result)
}

pub(crate) async fn link_task_job<D: Projects>(
    State(served): State<Served<D>>,
    Path(row): Path<String>,
    body: Bytes,
) -> Response {
    let link = match decoded(&served, "a job to link", &body) {
        Ok(link) => link,
        Err(response) => return response,
    };
    let result = served.daemon().link_task_job(row, link).await;
    answered(&served, result)
}

pub(crate) async fn unlink_task_job<D: Projects>(
    State(served): State<Served<D>>,
    Path(row): Path<String>,
) -> Response {
    let result = served.daemon().unlink_task_job(row).await;
    answered(&served, result)
}

pub(crate) async fn link_task_pull<D: Projects>(
    State(served): State<Served<D>>,
    Path(row): Path<String>,
    body: Bytes,
) -> Response {
    let link = match decoded(&served, "a pull request to link", &body) {
        Ok(link) => link,
        Err(response) => return response,
    };
    let result = served.daemon().link_task_pull(row, link).await;
    answered(&served, result)
}

pub(crate) async fn unlink_task_pull<D: Projects>(
    State(served): State<Served<D>>,
    Path(row): Path<String>,
    body: Bytes,
) -> Response {
    let link = match decoded(&served, "a pull request to unlink", &body) {
        Ok(link) => link,
        Err(response) => return response,
    };
    let result = served.daemon().unlink_task_pull(row, link).await;
    answered(&served, result)
}
