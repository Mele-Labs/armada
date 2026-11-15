//! A Job's asides over the router: a side question asked, the Job's asides listed, and one told to the Job.

use axum::body::Bytes;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::Response;
use serde::Deserialize;

use crate::answers::{answer, refused, undecodable};
use crate::daemon::Asides;
use crate::served::Served;

/// The owner puts a side question. 202 with the aside, `asking`; 404 for no such Job; 422 for a blank question or a Job with no
/// finished turn yet.
pub(crate) async fn ask_aside<D: Asides>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let ask: ipc::AskAside = match ipc::decode_body("a side question", &body) {
        Ok(ask) => ask,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.shared().ask_aside(ask).await {
        Ok(aside) => answer(StatusCode::ACCEPTED, &aside, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// `?session_id=` on `list_asides`.
#[derive(Deserialize)]
pub(crate) struct OfJob {
    session_id: String,
}

/// The Job's asides, oldest first. 200; 404 for no such Job.
pub(crate) async fn list_asides<D: Asides>(State(served): State<Served<D>>, Query(of): Query<OfJob>) -> Response {
    match served.daemon().list_asides(ipc::SessionId::carried(of.session_id)).await {
        Ok(list) => answer(StatusCode::OK, &list, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// The owner tells an answered aside to the Job. 200 with the aside, `told_at` set; 404 for a Job or an aside it does not
/// hold; 409 for one still asking, failed, or already told.
pub(crate) async fn tell_aside<D: Asides>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let tell: ipc::TellAside = match ipc::decode_body("an aside to tell", &body) {
        Ok(tell) => tell,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.shared().tell_aside(tell).await {
        Ok(aside) => answer(StatusCode::OK, &aside, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}
