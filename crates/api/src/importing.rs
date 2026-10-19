//! Bringing in the folders a person already works in with their agent CLI: which they are, how one
//! was run, and installing Fleet's mod into the CLI.

use axum::body::Bytes;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::Response;

use crate::answers::{answer, refused, undecodable};
use crate::daemon::{Commands, Queries};
use crate::served::Served;

/// The folders the CLI kept sessions for.
pub(crate) async fn list_import_folders<D: Queries>(State(served): State<Served<D>>) -> Response {
    match served.daemon().list_import_folders().await {
        Ok(folders) => answer(StatusCode::OK, &folders, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// `?path=` on `get_import_runs`: a folder `list_import_folders` answered.
#[derive(serde::Deserialize)]
pub(crate) struct OfFolder {
    path: String,
}

/// How one of them was run.
pub(crate) async fn get_import_runs<D: Queries>(
    State(served): State<Served<D>>,
    Query(OfFolder { path }): Query<OfFolder>,
) -> Response {
    match served.daemon().get_import_runs(path).await {
        Ok(runs) => answer(StatusCode::OK, &runs, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// 200 whether the CLI installed it or not: the answer says which, in the CLI's words.
pub(crate) async fn install_agent_mod<D: Commands>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let asked: ipc::InstallAgentMod = match ipc::decode("a request to install the mod", &body) {
        Ok(asked) => asked,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().install_agent_mod(asked).await {
        Ok(installed) => answer(StatusCode::OK, &installed, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}
