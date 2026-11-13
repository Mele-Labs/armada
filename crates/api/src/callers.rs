//! Who this listener answers, decided before a handler sees the request.
//!
//! **Fleet answers no page in a browser.** The port is on loopback, which is a
//! boundary around the machine and not around the machine's browsers: a page
//! in any tab can open a loopback port, and a WebSocket upgrade is not subject
//! to the same-origin policy at all — the browser sends `Origin` and leaves
//! the server to decide. Nothing decided, so the event stream was readable by
//! any page that found the port and the routes that change state were
//! reachable by one (`#1460`).
//!
//! **Presence is the refusal, and an allowlist would be the wrong shape.** A
//! browser sets `Origin` and a page cannot forge it; anything that is not a
//! browser sends whatever it likes, so naming trusted values buys nothing.
//! Absence stays allowed, and absence is what every caller Fleet has sends.

use std::net::SocketAddr;

use axum::extract::{ConnectInfo, MatchedPath, Request, State};
use axum::http::{header, StatusCode};
use axum::middleware::Next;
use axum::response::Response;
use ipc::{RunId, WireError, WireValue};

use crate::answers::problem;
use crate::daemon::Admitting;
use crate::mcp::Caller;
use crate::served::Served;

/// A request or an upgrade that carried an `Origin` header, which is a page in
/// a browser saying it is one.
///
/// **Not a caller that got its credentials wrong.** There are no credentials
/// on this seam; what this names is a kind of caller Fleet serves nothing to,
/// so the fix is never to retry with something added. `origin` on the error
/// carries the page's own address, which is the one fact that says which tab.
pub(crate) const FROM_A_PAGE: &str = "api.from_a_page";

/// The one doorman, over the whole listener.
///
/// **A layer and not a check in each handler**, which is the difference
/// between a property of the seam and a convention every new route has to
/// remember. The HTTP half and the upgrade half are the same decision here:
/// an upgrade is an ordinary request until the handler says otherwise, so a
/// page is refused before [`axum::extract::ws::WebSocketUpgrade`] is reached
/// and no socket opens.
pub(crate) async fn refuse_a_page(
    State(run_id): State<RunId>,
    request: Request,
    next: Next,
) -> Response {
    let Some(origin) = request.headers().get(header::ORIGIN) else {
        return next.run(request).await;
    };
    let said = origin.to_str().unwrap_or("bytes that are not text");
    problem(
        StatusCode::FORBIDDEN,
        &WireError::raised(
            FROM_A_PAGE,
            "this request carried an Origin header, so it came from a page in a browser, and \
             Fleet answers no page. Bridge's main process, the armada CLI, an agent's relay and \
             a Drone all reach Fleet without one",
            run_id,
        )
        .with_field("origin", WireValue::Str(said.to_string())),
    )
}

/// A call from inside a Job to a route that is the person's alone.
///
/// **Placed, never claimed**: by the process holding the connection, as the door places a Job
/// (`crate::daemon::Admitting::job_at`). A header saying `bridge` is something any process can
/// send, and a credential Bridge could read from disk a Job running as the same user could read
/// too; which process holds the socket is the kernel's record, and a Job cannot write it.
pub(crate) const A_JOBS_CALL: &str = "api.the_persons_own";

/// The acts only the person takes, from Bridge, each with what its refusal says the person
/// does. **The door already offers none of them to an ordinary Job**; this closes the same routes
/// to a Job calling the port around the door, as Job `8fd49e7c` turned the Operator on.
pub(crate) const THE_PERSONS_OWN: &[(&str, &str)] = &[
    ("save_settings", "changes settings"),
    ("save_limits", "changes settings"),
    ("save_preferences", "changes settings"),
    ("set_phone", "changes settings"),
    ("set_up_mac", "changes settings"),
    ("set_repository_profile", "changes settings"),
    ("set_notice_mute", "changes settings"),
    ("accept_downshift", "changes settings"),
    ("restore_downshift", "changes settings"),
    ("keep_downshift", "changes settings"),
    ("settle_downshift", "changes settings"),
    ("save_flight_controls", "changes settings"),
    ("save_repository_flight_controls", "changes settings"),
    ("approve_repository_tile", "changes settings"),
    ("change_slot_pool", "changes the slot pool"),
    ("confirm_operator_order", "gives the Operator its orders"),
    ("remove_operator_order", "gives the Operator its orders"),
    ("move_on_operator_board", "moves the Operator's board"),
    ("answer_operator_suggestion", "answers the Operator"),
    ("operator_answer", "answers the Operator"),
    ("operator_turn_order", "orders the Operator's Your turn"),
    ("operator_later", "puts an item of the Operator's Your turn last"),
    ("talk_to_operator", "talks to the Operator"),
    ("start_autopilot", "runs the Autopilot"),
    ("stop_autopilot", "runs the Autopilot"),
    ("pause_autopilot", "runs the Autopilot"),
    ("resume_autopilot", "runs the Autopilot"),
    ("update_autopilot", "runs the Autopilot"),
    ("override_autopilot_decision", "runs the Autopilot"),
    ("review_autopilot_decision", "runs the Autopilot"),
    ("change_fleet_build", "chooses the build Fleet runs"),
    ("update_mac", "chooses the build Fleet runs"),
    ("swap_fleet_build", "chooses the build Fleet runs"),
    ("keep_fleet_build", "chooses the build Fleet runs"),
    ("revert_fleet_build", "chooses the build Fleet runs"),
    ("pick_fleet_build_job", "chooses the build Fleet runs"),
    ("skip_preview_refresh", "chooses the build Fleet runs"),
    ("start_sign_in", "signs this Mac in and out"),
    ("sign_out", "signs this Mac in and out"),
    // What his other Macs hold, as this Mac's Phone Gateway hands it in: a Job planting rows would be drawn
    // as another Mac's. The Gateway runs in Fleet's own process, which no Job holds.
    ("hold_other_mac_sessions", "pairs his other Macs"),
    ("hold_other_mac_bug_reports", "pairs his other Macs"),
    ("hold_other_mac_projects", "pairs his other Macs"),
    ("project_act_from_mac", "pairs his other Macs"),
    ("act_from_mac", "pairs his other Macs"),
    ("take_back_queued", "takes back what waits for his other Macs"),
    ("hold_other_mac_event", "pairs his other Macs"),
    ("forget_other_mac", "pairs his other Macs"),
];

/// The operation a matched route serves and what its refusal says, where it is the person's.
pub(crate) fn the_persons(method: &str, path: &str) -> Option<(&'static str, &'static str)> {
    let route = crate::SERVED
        .iter()
        .find(|route| route.method == method && route.path == path)?;
    THE_PERSONS_OWN
        .iter()
        .find(|(operation, _)| *operation == route.operation)
        .copied()
}

/// The layer: a call to a route in [`THE_PERSONS_OWN`] from inside a Job is refused, plainly and
/// before the handler, and every other call passes. **A call with no peer passes**, which is a
/// test's router; the served listener always records one.
pub(crate) async fn refuse_a_job<D: Admitting>(
    State(served): State<Served<D>>,
    request: Request,
    next: Next,
) -> Response {
    let persons = request
        .extensions()
        .get::<MatchedPath>()
        .and_then(|matched| the_persons(request.method().as_str(), matched.as_str()));
    let peer = request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|ConnectInfo(peer)| *peer);
    let (Some((operation, does)), Some(peer)) = (persons, peer) else {
        return next.run(request).await;
    };
    if !served.daemon().job_at(Caller::at(peer)) {
        return next.run(request).await;
    }
    problem(
        StatusCode::FORBIDDEN,
        &WireError::raised(
            A_JOBS_CALL,
            format!("`{operation}` is refused to a Job: only the person {does}, from Bridge"),
            served.run_id().clone(),
        )
        .with_field("operation", WireValue::Str(operation.to_string())),
    )
}
