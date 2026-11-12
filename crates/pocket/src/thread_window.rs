//! `/api/sessions/one` with `last`: the Job's record and only the newest of its rows, so a long Job
//! opens on its latest turns. `before` pages back from a row the phone holds. Without `last` the
//! read is Fleet's, passed on whole, as [`crate::forward`] passes it.

use axum::body::Bytes;
use axum::extract::{Query, State};
use axum::http::{header, Method, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use ipc::{SessionRecord, SessionRow, SessionThread};
use serde::{Deserialize, Serialize};

use crate::reads::{json, port, refusal, NOT_ANSWERING, NOT_READABLE};
use crate::{fleet_client, forward, Gateway};

#[derive(Deserialize)]
pub struct Which {
    last: Option<usize>,
    before: Option<String>,
}

#[derive(Serialize)]
struct Window<'a> {
    session: &'a SessionRecord,
    rows: &'a [SessionRow],
    /// Rows stand before the first of `rows`, for a later read with `before`.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    older: bool,
}

/// The `last` rows that stand before the row `before`, or before the end where it is absent or not
/// in the thread, and whether any stand before those.
pub(crate) fn window<'a>(rows: &'a [SessionRow], last: usize, before: Option<&str>) -> (&'a [SessionRow], bool) {
    let end = before.and_then(|id| rows.iter().position(|row| row.id() == id)).unwrap_or(rows.len());
    let start = end.saturating_sub(last);
    (&rows[start..end], start > 0)
}

pub async fn one(State(gateway): State<Gateway>, method: Method, uri: Uri, Query(which): Query<Which>) -> Response {
    let Some(last) = which.last else {
        return forward::forward(State(gateway), method, uri, Bytes::new()).await;
    };
    let port = match port(&gateway) {
        Ok(port) => port,
        Err(refused) => return refused,
    };
    let answer = match fleet_client::send(port, "GET", forward::fleet_target(&uri), None).await {
        Ok(answer) => answer,
        Err(_) => return refusal(StatusCode::SERVICE_UNAVAILABLE, NOT_ANSWERING),
    };
    if answer.status != 200 {
        let status = StatusCode::from_u16(answer.status).unwrap_or(StatusCode::BAD_GATEWAY);
        let kind = answer.content_type.unwrap_or_else(|| String::from("application/json"));
        return (status, [(header::CONTENT_TYPE, kind)], answer.body).into_response();
    }
    let Ok(thread) = ipc::decode::<SessionThread>("a Job's thread", &answer.body) else {
        return refusal(StatusCode::BAD_GATEWAY, NOT_READABLE);
    };
    let (rows, older) = window(&thread.rows, last, which.before.as_deref());
    json(&Window { session: &thread.session, rows, older })
}
