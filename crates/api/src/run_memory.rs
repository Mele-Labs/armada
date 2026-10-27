//! How a repository runs, as Fleet remembers it: the facts, the place they are kept, and dropping
//! one. `?repository=` is a root `list_repositories` names.

use axum::body::Bytes;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::Response;

use crate::answers::{answer, refused, undecodable};
use crate::daemon::{Commands, Queries};
use crate::served::Served;

#[derive(serde::Deserialize)]
pub(crate) struct OfRepository {
    repository: String,
}

/// Each workspace's remembered runs, with where each is kept.
pub(crate) async fn get_run_memory<D: Queries>(
    State(served): State<Served<D>>,
    Query(OfRepository { repository }): Query<OfRepository>,
) -> Response {
    match served.daemon().get_run_memory(repository).await {
        Ok(memory) => answer(StatusCode::OK, &memory, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Where the repository's runs are kept.
pub(crate) async fn get_run_memory_place<D: Queries>(
    State(served): State<Served<D>>,
    Query(OfRepository { repository }): Query<OfRepository>,
) -> Response {
    match served.daemon().get_run_memory_place(repository).await {
        Ok(place) => answer(StatusCode::OK, &place, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Choose the place; answers with it.
pub(crate) async fn set_run_memory_place<D: Commands>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let chosen: ipc::RunMemoryPlace = match ipc::decode_body("a place for a repository's runs", &body) {
        Ok(chosen) => chosen,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().set_run_memory_place(chosen).await {
        Ok(place) => answer(StatusCode::OK, &place, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Drop one fact for good; answers with what remains.
pub(crate) async fn drop_run_fact<D: Commands>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let dropping: ipc::DropRunFact = match ipc::decode_body("a run to drop", &body) {
        Ok(dropping) => dropping,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().drop_run_fact(dropping).await {
        Ok(memory) => answer(StatusCode::OK, &memory, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}
