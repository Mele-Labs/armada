//! Watches over the router: started, weighed, stopped and listed for one Job.
//!
//! **The Job is the connection's** where Fleet hosts it, as a request's is; a body's `session_id`
//! is read only where the connection places none.

use axum::body::Bytes;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::Response;
use serde::Deserialize;

use crate::answers::{answer, refused, undecodable};
use crate::daemon::Watches;
use crate::served::Served;

pub(crate) async fn start_watch<D: Watches>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let start: ipc::StartWatch = match ipc::decode("a watch to start", &body) {
        Ok(start) => start,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().start_watch(crate::acting::asking(), start).await {
        Ok(started) => answer(StatusCode::OK, &started, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn weigh_watch<D: Watches>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let weigh: ipc::WeighWatch = match ipc::decode("a watch weighed", &body) {
        Ok(weigh) => weigh,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().weigh_watch(crate::acting::asking(), weigh).await {
        Ok(weighed) => answer(StatusCode::OK, &weighed, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn stop_watch<D: Watches>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let stop: ipc::StopWatch = match ipc::decode("a watch to stop", &body) {
        Ok(stop) => stop,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().stop_watch(crate::acting::asking(), stop).await {
        Ok(stopped) => answer(StatusCode::OK, &stopped, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn delete_watch<D: Watches>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let delete: ipc::DeleteWatch = match ipc::decode("a watch to delete", &body) {
        Ok(delete) => delete,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().delete_watch(crate::acting::asking(), delete).await {
        Ok(left) => answer(StatusCode::OK, &left, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// `?session_id=`, read only where the connection places no Job.
#[derive(Deserialize)]
pub(crate) struct ForWatches {
    #[serde(default)]
    session_id: Option<String>,
}

/// `?session_id=` as [`ForWatches`], and `?ended=true` for the ended watches too.
#[derive(Deserialize)]
pub(crate) struct ListingWatches {
    #[serde(default)]
    session_id: Option<String>,
    #[serde(default)]
    ended: Option<String>,
}

pub(crate) async fn list_watches<D: Watches>(
    State(served): State<Served<D>>,
    Query(asked): Query<ListingWatches>,
) -> Response {
    let named = asked
        .session_id
        .filter(|id| !id.trim().is_empty())
        .map(|id| ipc::SessionId::carried(id.trim()));
    let ended = match asked.ended.as_deref().map(str::trim) {
        None | Some("" | "false") => false,
        Some("true") => true,
        Some(other) => return undecodable(&format!("`ended` is `true` or `false`, not `{other}`"), served.run_id()),
    };
    match served.daemon().list_watches(crate::acting::asking(), named, ended).await {
        Ok(list) => answer(StatusCode::OK, &list, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn try_watch<D: Watches>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let tried: ipc::TryWatch = match ipc::decode("a watch to try", &body) {
        Ok(tried) => tried,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().try_watch(crate::acting::asking(), tried).await {
        Ok(read) => answer(StatusCode::OK, &read, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn list_watch_recipes<D: Watches>(
    State(served): State<Served<D>>,
    Query(asked): Query<ForWatches>,
) -> Response {
    let named = asked
        .session_id
        .filter(|id| !id.trim().is_empty())
        .map(|id| ipc::SessionId::carried(id.trim()));
    match served.daemon().list_watch_recipes(crate::acting::asking(), named).await {
        Ok(list) => answer(StatusCode::OK, &list, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn save_watch_recipe<D: Watches>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let save: ipc::SaveWatchRecipe = match ipc::decode("a watch recipe to save", &body) {
        Ok(save) => save,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().save_watch_recipe(crate::acting::asking(), save).await {
        Ok(saved) => answer(StatusCode::OK, &saved, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}
