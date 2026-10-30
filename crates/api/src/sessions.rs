//! `report_session`, `list_sessions` and `who_owns`. Since 23.43. `rename_session`, since 23.52.
//! `recall` and `get_logbook_for_subagent`, the Logbook's two mid-task reads, beside them.
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
    let report: ipc::SessionReport = match ipc::decode_body("a job report", &body) {
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
    let rename: ipc::RenameSession = match ipc::decode_body("a job rename", &body) {
        Ok(rename) => rename,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().rename_session(rename).await {
        Ok(record) => answer(StatusCode::OK, &record, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Armada's name for a hosted session. 200 with the row; 422 for a session that is not
/// hosted, has no message, or that the proposer could not name.
pub(crate) async fn name_session<D: Sessions>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let name: ipc::NameSession = match ipc::decode_body("a job to name", &body) {
        Ok(name) => name,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().name_session(name).await {
        Ok(record) => answer(StatusCode::OK, &record, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// 202: the run is underway. 422 for a Session that has ended or that Fleet does not know.
pub(crate) async fn start_session_run<D: Sessions>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let start: ipc::StartSessionRun = match ipc::decode_body("a job run to start", &body) {
        Ok(start) => start,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.shared().start_session_run(start).await {
        Ok(underway) => answer(StatusCode::ACCEPTED, &underway, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// 200 once the command has ended; 422 for a Session that holds no slot, has ended, or that Fleet
/// does not know.
pub(crate) async fn run_session_shell<D: Sessions>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let run: ipc::RunSessionShell = match ipc::decode_body("a shell action to run", &body) {
        Ok(run) => run,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.shared().run_session_shell(run).await {
        Ok(ran) => answer(StatusCode::OK, &ran, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// 200 with the run's id; 422 for a run that is not in flight.
pub(crate) async fn stop_session_run<D: Sessions>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let stop: ipc::StopSessionRun = match ipc::decode_body("a job run to stop", &body) {
        Ok(stop) => stop,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().stop_session_run(stop).await {
        Ok(stopped) => answer(StatusCode::OK, &stopped, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// 200 with the run's id; 422 for a run that is not in flight.
pub(crate) async fn write_session_run<D: Sessions>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let write: ipc::WriteSessionRun = match ipc::decode_body("keys for a job run", &body) {
        Ok(write) => write,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().write_session_run(write).await {
        Ok(written) => answer(StatusCode::OK, &written, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// 200 with the run's id; 422 for a run that is not in flight, or a size with no cells.
pub(crate) async fn resize_session_run<D: Sessions>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let resize: ipc::ResizeSessionRun = match ipc::decode_body("a job run to resize", &body) {
        Ok(resize) => resize,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().resize_session_run(resize).await {
        Ok(resized) => answer(StatusCode::OK, &resized, served.run_id()),
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
    let show: ipc::ShowWindow = match ipc::decode_body("a window to show", &body) {
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

/// A session takes back a walk it showed. 200 with the row, or a [`Receipt`] through the agent's door; 422 for an address on no window
/// the session showed, or a call that places no session. **The session is placed by the connection**, as `show_window`'s is.
pub(crate) async fn withdraw_window<D: Sessions>(
    State(served): State<Served<D>>,
    door: Option<Extension<DoorCalled>>,
    body: Bytes,
) -> Response {
    let withdraw: ipc::WithdrawWindow = match ipc::decode_body("a window to withdraw", &body) {
        Ok(withdraw) => withdraw,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served
        .daemon()
        .withdraw_window(crate::acting::asking(), withdraw.clone())
        .await
    {
        Ok(record) if door.is_some() => {
            answer(StatusCode::OK, &Receipt::withdrawn(&record, &withdraw), served.run_id())
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
    let show: ipc::ShowSketch = match ipc::decode_body("a sketch to show", &body) {
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

/// What `record_plan` carries beside [`ipc::RecordPlan`]: the agent's word that the person would see the change. Read
/// here, off the same body, so it is no field of the wire type. It holds nothing.
#[derive(Deserialize)]
struct PlanSeen {
    #[serde(default)]
    seen: bool,
}

/// A session records its plan. 200 with the row, or a [`Receipt`] through the agent's door naming the version and
/// each group and task by the id Fleet gave it; 422 for a plan that breaks a rule, naming each, or a call that places
/// no session. **The session is placed by the connection**, as `show_window`'s is.
pub(crate) async fn record_plan<D: Sessions>(
    State(served): State<Served<D>>,
    door: Option<Extension<DoorCalled>>,
    body: Bytes,
) -> Response {
    let plan: ipc::RecordPlan = match ipc::decode_body("a plan to record", &body) {
        Ok(plan) => plan,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    let seen = ipc::decode::<PlanSeen>("a plan to record", &body).is_ok_and(|beside| beside.seen);
    match served.daemon().record_plan(crate::acting::asking(), plan, seen).await {
        Ok(record) if door.is_some() => answer(StatusCode::OK, &Receipt::plan(&record), served.run_id()),
        Ok(record) => answer(StatusCode::OK, &record, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// The person approves a sketch, or sends it back with a note. 200 with the row; 409 for a sketch drawn again since
/// he read it; 422 for one not open to an answer, or a send-back with no note.
pub(crate) async fn answer_sketch<D: Sessions>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let said: ipc::AnswerSketch = match ipc::decode_body("an answer to a sketch", &body) {
        Ok(said) => said,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.shared().answer_sketch(said).await {
        Ok(record) => answer(StatusCode::OK, &record, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// The person approves the plan at the version he read. 200 with the row; 409 for a plan that is not that version or
/// none; 422 for a session Fleet does not know.
pub(crate) async fn approve_plan<D: Sessions>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let approve: ipc::ApprovePlan = match ipc::decode_body("a plan to approve", &body) {
        Ok(approve) => approve,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.shared().approve_plan(approve).await {
        Ok(record) => answer(StatusCode::OK, &record, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// The person sends the changes he marked on the plan to its agent. 200 with the row; 409 for a plan that is not the
/// version the marks are against; 422 for a mark that names nothing the plan holds, or none to send.
pub(crate) async fn send_plan_changes<D: Sessions>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let changes: ipc::SendPlanChanges = match ipc::decode_body("changes to a plan", &body) {
        Ok(changes) => changes,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.shared().send_plan_changes(changes).await {
        Ok(record) => answer(StatusCode::OK, &record, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// The person asks a judge to read a Session's work now. 200 with the row at once, `judging` set; 409 for a Session
/// that ended; 422 for a terminal one.
pub(crate) async fn judge_session<D: Sessions>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let judge: ipc::JudgeSession = match ipc::decode_body("a Job to judge", &body) {
        Ok(judge) => judge,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.shared().judge_session(judge).await {
        Ok(record) => answer(StatusCode::OK, &record, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// The person sends a judge's concern to the agent as his message. 200 with the row; 409 for an item that no longer
/// waits.
pub(crate) async fn send_concern<D: Sessions>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let send: ipc::SendConcern = match ipc::decode_body("a concern to send", &body) {
        Ok(send) => send,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.shared().send_concern(send).await {
        Ok(record) => answer(StatusCode::OK, &record, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// The person dismisses a judge's concern; the agent is told nothing. 200 with the row; 409 for an item that no longer
/// waits.
pub(crate) async fn dismiss_concern<D: Sessions>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let dismiss: ipc::DismissConcern = match ipc::decode_body("a concern to dismiss", &body) {
        Ok(dismiss) => dismiss,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.shared().dismiss_concern(dismiss).await {
        Ok(record) => answer(StatusCode::OK, &record, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// A session puts a file in front of the person. 200 with the row, or a [`Receipt`] through the agent's door; 422 for a path that
/// is missing, a folder or too large, or a call that places no session. **The session is placed by the connection**, as
/// `show_window`'s is.
pub(crate) async fn show_file<D: Sessions>(
    State(served): State<Served<D>>,
    door: Option<Extension<DoorCalled>>,
    body: Bytes,
) -> Response {
    let show: ipc::ShowFile = match ipc::decode_body("a file to show", &body) {
        Ok(show) => show,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served
        .daemon()
        .show_file(crate::acting::asking(), show.clone())
        .await
    {
        Ok(record) if door.is_some() => {
            answer(StatusCode::OK, &Receipt::file(&record, &show), served.run_id())
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
    let ask: ipc::AskInWindow = match ipc::decode_body("a question to ask in the window", &body) {
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

/// A session puts a draft to the person. 200 with the row, or a [`Receipt`] through the door; 422 for
/// a blank title or text, or a call that places no session.
pub(crate) async fn propose_draft<D: Sessions>(
    State(served): State<Served<D>>,
    door: Option<Extension<DoorCalled>>,
    body: Bytes,
) -> Response {
    let draft: ipc::ProposeDraft = match ipc::decode("a draft to propose", &body) {
        Ok(draft) => draft,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served
        .daemon()
        .propose_draft(crate::acting::asking(), draft.clone())
        .await
    {
        Ok(record) if door.is_some() => {
            answer(StatusCode::OK, &Receipt::drafted(&record, &draft), served.run_id())
        }
        Ok(record) => answer(StatusCode::OK, &record, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// A session takes back a draft it proposed. 200 with the row, or a [`Receipt`] through the door; 422 for a draft
/// that is not this session's or not open, or a call that places no session. **The session is placed by the connection**.
pub(crate) async fn withdraw_draft<D: Sessions>(
    State(served): State<Served<D>>,
    door: Option<Extension<DoorCalled>>,
    body: Bytes,
) -> Response {
    let withdraw: ipc::WithdrawDraft = match ipc::decode_body("a draft to withdraw", &body) {
        Ok(withdraw) => withdraw,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served
        .daemon()
        .withdraw_draft(crate::acting::asking(), withdraw.clone())
        .await
    {
        Ok(record) if door.is_some() => {
            answer(StatusCode::OK, &Receipt::draft_withdrawn(&record, &withdraw), served.run_id())
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
    let closed: ipc::WindowClosed = match ipc::decode_body("a closed window", &body) {
        Ok(closed) => closed,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().window_closed(closed).await {
        Ok(record) => answer(StatusCode::OK, &record, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Bridge, opening a session's window, takes the sign-in addresses held for it. 200 with the addresses, empty where none are held.
pub(crate) async fn take_window_sign_in<D: Sessions>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let take: ipc::TakeWindowSignIn = match ipc::decode_body("a window to sign in", &body) {
        Ok(take) => take,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().take_window_sign_in(take).await {
        Ok(taken) => answer(StatusCode::OK, &taken, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// A session's window could not show its page. 200 with whether the session was told; 422 for a
/// session Fleet does not know.
pub(crate) async fn window_failed<D: Sessions>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let failed: ipc::WindowFailed = match ipc::decode_body("a failed window", &body) {
        Ok(failed) => failed,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.shared().window_failed(failed).await {
        Ok(told) => answer(StatusCode::OK, &told, served.run_id()),
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
    let set: ipc::SetWaitingFor = match ipc::decode_body("what a job waits on", &body) {
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

/// `?session_id=`, read only where the connection places no session.
#[derive(Deserialize)]
pub(crate) struct ForClosedWaiting {
    #[serde(default)]
    session_id: Option<String>,
}

/// What the person closed from a session's list, newest first. 200; 422 for a call that places no session.
/// **The session is placed by the connection**, as `waiting_for`'s is.
pub(crate) async fn list_closed_waiting<D: Sessions>(
    State(served): State<Served<D>>,
    Query(asked): Query<ForClosedWaiting>,
) -> Response {
    let named = asked
        .session_id
        .filter(|id| !id.trim().is_empty())
        .map(|id| ipc::SessionId::carried(id.trim()));
    match served.daemon().list_closed_waiting(crate::acting::asking(), named).await {
        Ok(list) => answer(StatusCode::OK, &list, served.run_id()),
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
    let done: ipc::AllDone = match ipc::decode_body("that a job is done", &body) {
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

/// The agent keeps a follow-up for later. 200 with what was kept and what is open; 422 for an empty sentence or a
/// call that places no session; 409 for a session that has ended.
pub(crate) async fn defer_later<D: Sessions>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let defer: ipc::DeferLater = match ipc::decode("a follow-up to keep", &body) {
        Ok(defer) => defer,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().defer_later(crate::acting::asking(), defer).await {
        Ok(kept) => answer(StatusCode::OK, &kept, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// A follow-up the session kept for later is settled. 200 with what was settled and what is still open; 422 for a
/// filed with no issue, an agent's drop with no reason, or a call that places no session; 409 for a follow-up not open.
pub(crate) async fn settle_later<D: Sessions>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let settle: ipc::SettleLater = match ipc::decode_body("a follow-up to settle", &body) {
        Ok(settle) => settle,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().settle_later(crate::acting::asking(), settle).await {
        Ok(settled) => answer(StatusCode::OK, &settled, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// The agent says Fleet's latest notice was wrong. 200 with the dispute and its review job; 422 for a call that
/// places no hosted session or a session Fleet told nothing. **The session is placed by the connection**, as `all_done`'s is.
pub(crate) async fn dispute_fleet<D: Sessions>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let dispute: ipc::DisputeFleet = match ipc::decode_body("which notice is disputed", &body) {
        Ok(dispute) => dispute,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.shared().dispute_fleet(crate::acting::asking(), dispute).await {
        Ok(disputed) => answer(StatusCode::OK, &disputed, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// The agent says Armada itself got something wrong. 200 with the id of the report Fleet kept; 422 for a call that places
/// no session or leaves out the title or what went wrong. **The session is placed by the connection**, as `all_done`'s is.
pub(crate) async fn report_bug<D: Sessions>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let report: ipc::ReportBug = match ipc::decode_body("a bug report", &body) {
        Ok(report) => report,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().report_bug(crate::acting::asking(), report).await {
        Ok(reported) => answer(StatusCode::OK, &reported, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// The bug reports on this Mac, newest first.
pub(crate) async fn list_bug_reports<D: Sessions>(State(served): State<Served<D>>) -> Response {
    match served.daemon().list_bug_reports().await {
        Ok(reports) => answer(StatusCode::OK, &reports, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// The report an act names.
#[derive(Deserialize)]
pub(crate) struct NamedReport {
    report_id: String,
}

/// The owner keeps a bug report. 200 with it; 404 for an id that names none.
pub(crate) async fn keep_bug_report<D: Sessions>(
    State(served): State<Served<D>>,
    axum::extract::Path(NamedReport { report_id }): axum::extract::Path<NamedReport>,
) -> Response {
    match served.daemon().keep_bug_report(report_id).await {
        Ok(report) => answer(StatusCode::OK, &report, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// The owner drops a bug report. 200 with its id; 404 for an id that names none.
pub(crate) async fn drop_bug_report<D: Sessions>(
    State(served): State<Served<D>>,
    axum::extract::Path(NamedReport { report_id }): axum::extract::Path<NamedReport>,
) -> Response {
    match served.daemon().drop_bug_report(report_id).await {
        Ok(dropped) => answer(StatusCode::OK, &dropped, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// The agent gives back its slots, merged branches and servers. 200 with what went and what stayed; 422 for a call that places
/// no session. **The session is placed by the connection**, as `all_done`'s is.
pub(crate) async fn finish_up<D: Sessions>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let finish: ipc::FinishUp = match ipc::decode_body("that a job is finished", &body) {
        Ok(finish) => finish,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().finish_up(crate::acting::asking(), finish).await {
        Ok(finished) => answer(StatusCode::OK, &finished, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// The agent marks done a note Bridge sent it. 200 with the path written; 422 for a note that is not
/// there or not this session's. **The session is placed by the connection**, as `finish_up`'s is.
pub(crate) async fn mark_annotation_done<D: Sessions>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let mark: ipc::MarkAnnotationDone = match ipc::decode_body("which note is done", &body) {
        Ok(mark) => mark,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().mark_annotation_done(crate::acting::asking(), mark).await {
        Ok(marked) => answer(StatusCode::OK, &marked, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// The agent resolves a pull request note Bridge sent it. 200 with the note; 422 for one that is not
/// there, not this session's, or already resolved. **The session is placed by the connection.**
pub(crate) async fn resolve_pull_request_note<D: Sessions>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let resolve: ipc::ResolvePullRequestNote = match ipc::decode_body("which note is resolved", &body) {
        Ok(resolve) => resolve,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().resolve_pull_request_note(crate::acting::asking(), resolve).await {
        Ok(note) => answer(StatusCode::OK, &note, served.run_id()),
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

/// `?session_id=` on `get_session_loadout` and `get_attachment_history`.
#[derive(Deserialize)]
pub(crate) struct WhichSession {
    session_id: String,
}

/// `?session_id=&since=&until=` on `get_session_usage`.
#[derive(Deserialize)]
pub(crate) struct UsageWindow {
    session_id: String,
    since: Option<String>,
    until: Option<String>,
}

pub(crate) async fn get_session_usage<D: Sessions>(
    State(served): State<Served<D>>,
    Query(window): Query<UsageWindow>,
) -> Response {
    match served
        .daemon()
        .get_session_usage(ipc::SessionId::carried(window.session_id), window.since, window.until)
        .await
    {
        Ok(usage) => answer(StatusCode::OK, &usage, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// `?session_id=` on `get_session_system`.
#[derive(Deserialize)]
pub(crate) struct SystemAsk {
    session_id: String,
}

pub(crate) async fn get_session_system<D: Sessions>(State(served): State<Served<D>>, Query(ask): Query<SystemAsk>) -> Response {
    match served.shared().get_session_system(ipc::SessionId::carried(ask.session_id)).await {
        Ok(system) => answer(StatusCode::OK, &system, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// One process the job started is ended. 200 with the pid and command; 422 for neither or both of pid and port, a
/// process the job did not start, or a call that places no session; 409 for a session that has ended.
pub(crate) async fn stop_process<D: Sessions>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let stop: ipc::StopProcess = match ipc::decode("a process to stop", &body) {
        Ok(stop) => stop,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.shared().stop_process(crate::acting::asking(), stop).await {
        Ok(stopped) => answer(StatusCode::OK, &stopped, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// `?since=&until=&tz_offset_minutes=` on `get_usage_summary`.
#[derive(Deserialize)]
pub(crate) struct SummaryWindow {
    since: Option<String>,
    until: Option<String>,
    #[serde(default)]
    tz_offset_minutes: i32,
}

pub(crate) async fn get_usage_summary<D: Sessions>(
    State(served): State<Served<D>>,
    Query(window): Query<SummaryWindow>,
) -> Response {
    match served.daemon().get_usage_summary(window.since, window.until, window.tz_offset_minutes).await {
        Ok(summary) => answer(StatusCode::OK, &summary, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// `?since=&until=` on `get_usage_rules`.
#[derive(Deserialize)]
pub(crate) struct RulesWindow {
    since: Option<String>,
    until: Option<String>,
}

pub(crate) async fn get_usage_rules<D: Sessions>(
    State(served): State<Served<D>>,
    Query(window): Query<RulesWindow>,
) -> Response {
    match served.daemon().get_usage_rules(window.since, window.until).await {
        Ok(rules) => answer(StatusCode::OK, &rules, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn get_session_loadout<D: Sessions>(
    State(served): State<Served<D>>,
    Query(which): Query<WhichSession>,
) -> Response {
    match served
        .daemon()
        .get_session_loadout(ipc::SessionId::carried(which.session_id))
        .await
    {
        Ok(loadout) => answer(StatusCode::OK, &loadout, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn get_attachment_history<D: Sessions>(
    State(served): State<Served<D>>,
    Query(which): Query<WhichSession>,
) -> Response {
    match served
        .daemon()
        .get_attachment_history(ipc::SessionId::carried(which.session_id))
        .await
    {
        Ok(history) => answer(StatusCode::OK, &history, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// `?session_id=&kind=&name=&source=&from=`: one row of a session's Loadout, as it names it.
pub(crate) async fn read_loadout_item<D: Sessions>(
    State(served): State<Served<D>>,
    Query(asked): Query<ipc::LoadoutItemAsked>,
) -> Response {
    match served.daemon().read_loadout_item(asked).await {
        Ok(body) => answer(StatusCode::OK, &body, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// A session or Job takes an orphaned pull request. 200 with the claim; 422 with the reason.
pub(crate) async fn claim_pull_request<D: Sessions>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let claim: ipc::ClaimPullRequest = match ipc::decode_body("a pull request to claim", &body) {
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

/// A session works an issue, by number or by a tracker's key. 200 with the claim; 422 with the
/// reason.
pub(crate) async fn claim_issue<D: Sessions>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let claim: ipc::ClaimIssue = match ipc::decode_body("an issue to claim", &body) {
        Ok(claim) => claim,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().claim_issue(crate::acting::asking(), claim).await {
        Ok(claimed) => answer(StatusCode::OK, &claimed, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// A Session takes its repository's red main. 200 with `MainRedTaken`; 409 naming the holder
/// where it is held elsewhere; 422 for a call that places no Session.
pub(crate) async fn take_main_red<D: Sessions>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let take: ipc::TakeMainRed = match ipc::decode_body("a red main to take", &body) {
        Ok(take) => take,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served
        .daemon()
        .take_main_red(crate::acting::asking(), take)
        .await
    {
        Ok(taken) => answer(StatusCode::OK, &taken, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// A Session's agent starts a Session, held for the person. 200 with `SessionHeld`; 422 for a call that
/// places no hosted Session, a blank title or message, or a repository Fleet does not serve.
pub(crate) async fn start_another_session<D: Sessions>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let start: ipc::StartAnotherSession = match ipc::decode_body("a job to start", &body) {
        Ok(start) => start,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served
        .shared()
        .start_another_session(crate::acting::asking(), start)
        .await
    {
        Ok(held) => answer(StatusCode::OK, &held, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// The agent of a Session, or the person, changes when a waiting Session starts. 200 with `SessionWait`;
/// 422 for a Session that does not wait on the caller, an unplaced agent or a plan part nothing holds; 409
/// where it has started.
pub(crate) async fn change_waiting<D: Sessions>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let change: ipc::ChangeWaiting = match ipc::decode_body("a change to a waiting job", &body) {
        Ok(change) => change,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.shared().change_waiting(crate::acting::asking(), change).await {
        Ok(wait) => answer(StatusCode::OK, &wait, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// A Session's agent forks its own conversation, held for the person. 200 with `SessionHeld`; 422 for a
/// call that places no hosted Session, a blank title or message, or a message the conversation does not hold.
pub(crate) async fn fork_this_session<D: Sessions>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let fork: ipc::ForkThisSession = match ipc::decode_body("a job to fork", &body) {
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
    let call: ipc::SessionCheckCall = match ipc::decode_body("a Job's Check run", &body) {
        Ok(call) => call,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().report_session_check(call).await {
        Ok(answered) => answer(StatusCode::OK, &answered, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// `armada worktree lease` tells Fleet every slot is held, or that it is done waiting. 200 with
/// where the person's choice stands.
pub(crate) async fn ask_for_slot<D: Sessions>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let call: ipc::AskForSlot = match ipc::decode_body("a lease waiting for a slot", &body) {
        Ok(call) => call,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().ask_for_slot(call).await {
        Ok(answered) => answer(StatusCode::OK, &answered, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// One Session run's log as it is written, on a socket of its own. A run no Session kept is a 422
/// before the socket opens.
pub(crate) async fn observe_session_check<D: Sessions>(
    State(served): State<Served<D>>,
    axum::extract::Path(run): axum::extract::Path<u64>,
    upgrade: axum::extract::WebSocketUpgrade,
) -> Response {
    match served.daemon().observe_session_check(run).await {
        Ok(live) => upgrade.on_upgrade(move |socket| crate::following::relay_session_check(socket, live)),
        Err(refusal) => refused(refusal),
    }
}

/// A person asks a running Session Check to stop, gracefully.
pub(crate) async fn stop_session_check<D: Sessions>(
    State(served): State<Served<D>>,
    axum::extract::Path(run): axum::extract::Path<u64>,
) -> Response {
    match served.daemon().stop_session_check(run, false).await {
        Ok(answered) => answer(StatusCode::OK, &answered, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// [`stop_session_check`], by force.
pub(crate) async fn force_stop_session_check<D: Sessions>(
    State(served): State<Served<D>>,
    axum::extract::Path(run): axum::extract::Path<u64>,
) -> Response {
    match served.daemon().stop_session_check(run, true).await {
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

/// `?query=` and `?section=` on `recall`, beside `?manifest_id=`.
#[derive(Deserialize)]
pub(crate) struct Recalling {
    query: String,
    #[serde(default)]
    section: Option<String>,
    #[serde(default)]
    manifest_id: Option<String>,
}

/// A search of the whole Logbook for the caller's repository, its habits and this Mac.
pub(crate) async fn recall<D: Sessions>(
    State(served): State<Served<D>>,
    Query(asked): Query<Recalling>,
) -> Response {
    let manifest = asked.manifest_id.map(ipc::ManifestId::carried);
    match served.daemon().recall(manifest, asked.query, asked.section).await {
        Ok(recalled) => answer(StatusCode::OK, &recalled, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// The entries a subagent starting under a Session is handed, for the paths its prompt names.
pub(crate) async fn get_logbook_for_subagent<D: Sessions>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let asked: ipc::SubagentAsk = match ipc::decode_body("a subagent's start", &body) {
        Ok(asked) => asked,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().get_logbook_for_subagent(asked.session_id, asked.prompt).await {
        Ok(handed) => answer(StatusCode::OK, &handed, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}
