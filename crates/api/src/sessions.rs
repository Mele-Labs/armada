//! `report_session`, `list_sessions` and `who_owns`. Since 23.43. `rename_session`, since 23.52.
//! `docs/concepts/session.md`.

use axum::body::Bytes;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::Response;
use axum::Extension;
use serde::Deserialize;

use crate::answers::{answer, refused, undecodable};
use crate::daemon::Sessions;
use crate::door::DoorCalled;
use crate::receipts::Receipt;
use crate::served::Served;

/// What a harness tells Fleet. 200: the row is kept when this answers.
pub(crate) async fn report_session<D: Sessions>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let report: ipc::SessionReport = match ipc::decode("a session report", &body) {
        Ok(report) => report,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().report_session(report).await {
        Ok(record) => answer(StatusCode::OK, &record, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// A person's name for a session. 200 with the row; 422 for a blank title or a
/// session Fleet does not know.
pub(crate) async fn rename_session<D: Sessions>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let rename: ipc::RenameSession = match ipc::decode("a session rename", &body) {
        Ok(rename) => rename,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().rename_session(rename).await {
        Ok(record) => answer(StatusCode::OK, &record, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// 202: the run is underway. 422 for a Session that has ended or that Fleet does not know.
pub(crate) async fn start_session_run<D: Sessions>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let start: ipc::StartSessionRun = match ipc::decode("a session run to start", &body) {
        Ok(start) => start,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().start_session_run(start).await {
        Ok(underway) => answer(StatusCode::ACCEPTED, &underway, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// 200 with the run's id; 422 for a run that is not in flight.
pub(crate) async fn stop_session_run<D: Sessions>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let stop: ipc::StopSessionRun = match ipc::decode("a session run to stop", &body) {
        Ok(stop) => stop,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().stop_session_run(stop).await {
        Ok(stopped) => answer(StatusCode::OK, &stopped, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

#[derive(Deserialize)]
pub(crate) struct SessionRunPath {
    run_id: String,
}

/// A Session run's output, as it prints, on a socket of its own.
pub(crate) async fn observe_session_run<D: Sessions>(
    State(served): State<Served<D>>,
    axum::extract::Path(SessionRunPath { run_id }): axum::extract::Path<SessionRunPath>,
    upgrade: axum::extract::WebSocketUpgrade,
) -> Response {
    match served.daemon().observe_session_run(run_id).await {
        Ok(observed) => upgrade
            .on_upgrade(move |socket| crate::watching_session_run::relay_session_run(socket, observed)),
        Err(refusal) => refused(refusal),
    }
}

/// A session shows the person a page. 200 with the row, or with a [`Receipt`] through the agent's door; 422 for an address that is not `http` or
/// `https`, or a call that places no session. **The session is placed by the connection** where the
/// call arrived through the agent's door.
pub(crate) async fn show_window<D: Sessions>(
    State(served): State<Served<D>>,
    door: Option<Extension<DoorCalled>>,
    body: Bytes,
) -> Response {
    let show: ipc::ShowWindow = match ipc::decode("a window to show", &body) {
        Ok(show) => show,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served
        .daemon()
        .show_window(crate::acting::asking(), show.clone())
        .await
    {
        Ok(record) if door.is_some() => {
            answer(StatusCode::OK, &Receipt::window(&record, &show), served.run_id())
        }
        Ok(record) => answer(StatusCode::OK, &record, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// A session draws the person a diagram. 200 with the row, or a [`Receipt`] through the agent's door; 422 for a scene that breaks a
/// rule, naming each, or a call that places no session. **The session is placed by the connection**, as `show_window`'s is.
pub(crate) async fn show_sketch<D: Sessions>(
    State(served): State<Served<D>>,
    door: Option<Extension<DoorCalled>>,
    body: Bytes,
) -> Response {
    let show: ipc::ShowSketch = match ipc::decode("a sketch to show", &body) {
        Ok(show) => show,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served
        .daemon()
        .show_sketch(crate::acting::asking(), show.clone())
        .await
    {
        Ok(record) if door.is_some() => {
            answer(StatusCode::OK, &Receipt::sketch(&record, &show), served.run_id())
        }
        Ok(record) => answer(StatusCode::OK, &record, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// A session asks the person about a place on its walk. 200 with the row, or a [`Receipt`] through the door; 422 for an address on no
/// window the session showed, a blank question or selector, or a call that places no session.
pub(crate) async fn ask_in_window<D: Sessions>(
    State(served): State<Served<D>>,
    door: Option<Extension<DoorCalled>>,
    body: Bytes,
) -> Response {
    let ask: ipc::AskInWindow = match ipc::decode("a question to ask in the window", &body) {
        Ok(ask) => ask,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served
        .daemon()
        .ask_in_window(crate::acting::asking(), ask.clone())
        .await
    {
        Ok(record) if door.is_some() => {
            answer(StatusCode::OK, &Receipt::asked(&record, &ask), served.run_id())
        }
        Ok(record) => answer(StatusCode::OK, &record, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// The person closed a session's window. 200 with the row; 422 for a session Fleet does not know.
pub(crate) async fn window_closed<D: Sessions>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let closed: ipc::WindowClosed = match ipc::decode("a closed window", &body) {
        Ok(closed) => closed,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().window_closed(closed).await {
        Ok(record) => answer(StatusCode::OK, &record, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// The agent sets what it is waiting on the person for. 200 with the row, or a [`Receipt`] through the door; 422 for a call that
/// places no session. **The session is placed by the connection**, as `show_window`'s is.
pub(crate) async fn waiting_for<D: Sessions>(
    State(served): State<Served<D>>,
    door: Option<Extension<DoorCalled>>,
    body: Bytes,
) -> Response {
    let set: ipc::SetWaitingFor = match ipc::decode("what a session waits on", &body) {
        Ok(set) => set,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served
        .daemon()
        .waiting_for(crate::acting::asking(), set)
        .await
    {
        Ok(record) if door.is_some() => {
            answer(StatusCode::OK, &Receipt::waiting_for(&record), served.run_id())
        }
        Ok(record) => answer(StatusCode::OK, &record, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// The agent says it has nothing left to work on. 200 with the row, or a [`Receipt`] through the door; 422 for a call that places no
/// session. **The session is placed by the connection**, as `waiting_for`'s is.
pub(crate) async fn all_done<D: Sessions>(
    State(served): State<Served<D>>,
    door: Option<Extension<DoorCalled>>,
    body: Bytes,
) -> Response {
    let done: ipc::AllDone = match ipc::decode("that a session is done", &body) {
        Ok(done) => done,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().all_done(crate::acting::asking(), done).await {
        Ok(record) if door.is_some() => {
            answer(StatusCode::OK, &Receipt::done(&record), served.run_id())
        }
        Ok(record) => answer(StatusCode::OK, &record, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// The agent gives back its slots, merged branches and servers. 200 with what went and what stayed; 422 for a call that places
/// no session. **The session is placed by the connection**, as `all_done`'s is.
pub(crate) async fn finish_up<D: Sessions>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let finish: ipc::FinishUp = match ipc::decode("that a session is finished", &body) {
        Ok(finish) => finish,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().finish_up(crate::acting::asking(), finish).await {
        Ok(finished) => answer(StatusCode::OK, &finished, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// `?manifest_id=`, `?q=`, `?state=` and `?view=` on `list_sessions`.
#[derive(Deserialize)]
pub(crate) struct Listing {
    #[serde(default)]
    manifest_id: Option<String>,
    #[serde(default)]
    q: Option<String>,
    #[serde(default)]
    state: Option<ipc::SessionState>,
    /// Absent is the whole record, which Bridge and Pocket read.
    #[serde(default)]
    view: ipc::SessionView,
}

pub(crate) async fn list_sessions<D: Sessions>(
    State(served): State<Served<D>>,
    Query(listing): Query<Listing>,
) -> Response {
    let manifest = listing.manifest_id.map(ipc::ManifestId::carried);
    match served
        .daemon()
        .list_sessions(manifest, listing.q, listing.state)
        .await
    {
        Ok(sessions) => match listing.view {
            ipc::SessionView::Full => answer(StatusCode::OK, &sessions, served.run_id()),
            ipc::SessionView::Summary => answer(
                StatusCode::OK,
                &ipc::SessionSummaries::of(&sessions),
                served.run_id(),
            ),
        },
        Err(refusal) => refused(refusal),
    }
}

/// `?kind=` and `?target=` on `who_owns`, beside `?manifest_id=`.
#[derive(Deserialize)]
pub(crate) struct Asked {
    kind: String,
    target: String,
    #[serde(default)]
    manifest_id: Option<String>,
}

pub(crate) async fn who_owns<D: Sessions>(
    State(served): State<Served<D>>,
    Query(asked): Query<Asked>,
) -> Response {
    let manifest = asked.manifest_id.map(ipc::ManifestId::carried);
    match served
        .daemon()
        .who_owns(asked.kind, asked.target, manifest)
        .await
    {
        Ok(owners) => answer(StatusCode::OK, &owners, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// A session or Job takes an orphaned pull request. 200 with the claim; 422 with the reason.
pub(crate) async fn claim_pull_request<D: Sessions>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let claim: ipc::ClaimPullRequest = match ipc::decode("a pull request to claim", &body) {
        Ok(claim) => claim,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served
        .daemon()
        .claim_pull_request(crate::acting::asking(), claim)
        .await
    {
        Ok(claimed) => answer(StatusCode::OK, &claimed, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// A Session's agent starts a Session, held for the person. 200 with `SessionHeld`; 422 for a call that
/// places no hosted Session, a blank title or message, or a repository Fleet does not serve.
pub(crate) async fn start_another_session<D: Sessions>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let start: ipc::StartAnotherSession = match ipc::decode("a session to start", &body) {
        Ok(start) => start,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served
        .daemon()
        .start_another_session(crate::acting::asking(), start)
        .await
    {
        Ok(held) => answer(StatusCode::OK, &held, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// A Session's agent forks its own conversation, held for the person. 200 with `SessionHeld`; 422 for a
/// call that places no hosted Session, a blank title or message, or a message the conversation does not hold.
pub(crate) async fn fork_this_session<D: Sessions>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let fork: ipc::ForkThisSession = match ipc::decode("a session to fork", &body) {
        Ok(fork) => fork,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served
        .daemon()
        .fork_this_session(crate::acting::asking(), fork)
        .await
    {
        Ok(held) => answer(StatusCode::OK, &held, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// `armada check` tells Fleet a Session's run began or ended. 200 with the run, or none where no
/// Session holds the slot; 422 for a call that names neither a Check nor a run.
pub(crate) async fn report_session_check<D: Sessions>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let call: ipc::SessionCheckCall = match ipc::decode("a Session's Check run", &body) {
        Ok(call) => call,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().report_session_check(call).await {
        Ok(answered) => answer(StatusCode::OK, &answered, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// One Session run's log. 200 with the window; 422 for a run no Session kept.
pub(crate) async fn get_session_check_output<D: Sessions>(
    State(served): State<Served<D>>,
    axum::extract::Path(run): axum::extract::Path<u64>,
) -> Response {
    match served.daemon().get_session_check_output(run).await {
        Ok(output) => answer(StatusCode::OK, &output, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}
