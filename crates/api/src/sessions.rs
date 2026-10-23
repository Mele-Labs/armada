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
    let report: ipc::SessionReport = match ipc::decode("a job report", &body) {
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
    let rename: ipc::RenameSession = match ipc::decode("a job rename", &body) {
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
    let start: ipc::StartSessionRun = match ipc::decode("a job run to start", &body) {
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
    let run: ipc::RunSessionShell = match ipc::decode("a shell action to run", &body) {
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
    let stop: ipc::StopSessionRun = match ipc::decode("a job run to stop", &body) {
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
    let write: ipc::WriteSessionRun = match ipc::decode("keys for a job run", &body) {
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
    let resize: ipc::ResizeSessionRun = match ipc::decode("a job run to resize", &body) {
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

/// A session takes back a walk it showed. 200 with the row, or a [`Receipt`] through the agent's door; 422 for an address on no window
/// the session showed, or a call that places no session. **The session is placed by the connection**, as `show_window`'s is.
pub(crate) async fn withdraw_window<D: Sessions>(
    State(served): State<Served<D>>,
    door: Option<Extension<DoorCalled>>,
    body: Bytes,
) -> Response {
    let withdraw: ipc::WithdrawWindow = match ipc::decode("a window to withdraw", &body) {
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

/// A session records its plan. 200 with the row, or a [`Receipt`] through the agent's door naming the version and
/// each group and task by the id Fleet gave it; 422 for a plan that breaks a rule, naming each, or a call that places
/// no session. **The session is placed by the connection**, as `show_window`'s is.
pub(crate) async fn record_plan<D: Sessions>(
    State(served): State<Served<D>>,
    door: Option<Extension<DoorCalled>>,
    body: Bytes,
) -> Response {
    let plan: ipc::RecordPlan = match ipc::decode("a plan to record", &body) {
        Ok(plan) => plan,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().record_plan(crate::acting::asking(), plan).await {
        Ok(record) if door.is_some() => answer(StatusCode::OK, &Receipt::plan(&record), served.run_id()),
        Ok(record) => answer(StatusCode::OK, &record, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// The person approves the plan at the version he read. 200 with the row; 409 for a plan that is not that version or
/// none; 422 for a session Fleet does not know.
pub(crate) async fn approve_plan<D: Sessions>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let approve: ipc::ApprovePlan = match ipc::decode("a plan to approve", &body) {
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
    let changes: ipc::SendPlanChanges = match ipc::decode("changes to a plan", &body) {
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
    let judge: ipc::JudgeSession = match ipc::decode("a Job to judge", &body) {
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
    let send: ipc::SendConcern = match ipc::decode("a concern to send", &body) {
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
    let dismiss: ipc::DismissConcern = match ipc::decode("a concern to dismiss", &body) {
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
    let show: ipc::ShowFile = match ipc::decode("a file to show", &body) {
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

/// Bridge, opening a session's window, takes the sign-in addresses held for it. 200 with the addresses, empty where none are held.
pub(crate) async fn take_window_sign_in<D: Sessions>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let take: ipc::TakeWindowSignIn = match ipc::decode("a window to sign in", &body) {
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
    let failed: ipc::WindowFailed = match ipc::decode("a failed window", &body) {
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
    let set: ipc::SetWaitingFor = match ipc::decode("what a job waits on", &body) {
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
    let done: ipc::AllDone = match ipc::decode("that a job is done", &body) {
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

/// A follow-up the session kept for later is settled. 200 with what was settled and what is still open; 422 for a
/// filed with no issue, an agent's drop with no reason, or a call that places no session; 409 for a follow-up not open.
pub(crate) async fn settle_later<D: Sessions>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let settle: ipc::SettleLater = match ipc::decode("a follow-up to settle", &body) {
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
    let dispute: ipc::DisputeFleet = match ipc::decode("which notice is disputed", &body) {
        Ok(dispute) => dispute,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.shared().dispute_fleet(crate::acting::asking(), dispute).await {
        Ok(disputed) => answer(StatusCode::OK, &disputed, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// The agent gives back its slots, merged branches and servers. 200 with what went and what stayed; 422 for a call that places
/// no session. **The session is placed by the connection**, as `all_done`'s is.
pub(crate) async fn finish_up<D: Sessions>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let finish: ipc::FinishUp = match ipc::decode("that a job is finished", &body) {
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
    let mark: ipc::MarkAnnotationDone = match ipc::decode("which note is done", &body) {
        Ok(mark) => mark,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().mark_annotation_done(crate::acting::asking(), mark).await {
        Ok(marked) => answer(StatusCode::OK, &marked, served.run_id()),
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

/// A session works an issue, by number or by a tracker's key. 200 with the claim; 422 with the
/// reason.
pub(crate) async fn claim_issue<D: Sessions>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let claim: ipc::ClaimIssue = match ipc::decode("an issue to claim", &body) {
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
    let take: ipc::TakeMainRed = match ipc::decode("a red main to take", &body) {
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
    let start: ipc::StartAnotherSession = match ipc::decode("a job to start", &body) {
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
    let change: ipc::ChangeWaiting = match ipc::decode("a change to a waiting job", &body) {
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
    let fork: ipc::ForkThisSession = match ipc::decode("a job to fork", &body) {
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
    let call: ipc::SessionCheckCall = match ipc::decode("a Job's Check run", &body) {
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
    let call: ipc::AskForSlot = match ipc::decode("a lease waiting for a slot", &body) {
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
    let asked: ipc::SubagentAsk = match ipc::decode("a subagent's start", &body) {
        Ok(asked) => asked,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().get_logbook_for_subagent(asked.session_id, asked.prompt).await {
        Ok(handed) => answer(StatusCode::OK, &handed, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}
