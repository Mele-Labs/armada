//! A shared Job's terminals, as another Mac reaches them through this Mac's Phone Gateway: the acts the person
//! takes on a terminal at home, and its output on a socket. `docs/concepts/pocket.md`, *Another Mac*.

use axum::body::Bytes;
use axum::extract::ws::WebSocketUpgrade;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::Response;
use axum::routing::{get, post};
use axum::Router;

use crate::answers::{answer, refused, undecodable};
use crate::daemon::{OtherMacs, TerminalShared};
use crate::served::Served;

pub(crate) fn mounted<D: OtherMacs>() -> Router<Served<D>> {
    Router::new()
        .route("/other_mac/terminal", post(share_job_terminal::<D>))
        .route("/other_mac/terminal/observe", get(share_job_terminal_output::<D>))
}

async fn share_job_terminal<D: OtherMacs>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let asked: ipc::TerminalFromMac = match ipc::decode_body("an act on a terminal from another Mac", &body) {
        Ok(asked) => asked,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.shared().share_job_terminal(asked).await {
        Ok(TerminalShared::Run(run)) => answer(StatusCode::OK, &run, served.run_id()),
        Ok(TerminalShared::Terminals(terminals)) => answer(StatusCode::OK, &terminals, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

async fn share_job_terminal_output<D: OtherMacs>(
    State(served): State<Served<D>>,
    Query(which): Query<ipc::TerminalOfJob>,
    upgrade: WebSocketUpgrade,
) -> Response {
    match served.daemon().share_job_terminal_output(which).await {
        Ok(observed) => upgrade.on_upgrade(move |socket| crate::watching_session_run::relay_session_run(socket, observed)),
        Err(refusal) => refused(refusal),
    }
}
