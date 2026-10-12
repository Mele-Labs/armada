//! The rows of [`SERVED`](super::SERVED) about a repository working its own backlog. #2092.

use super::Route;

pub(super) const ROUTES: &[Route] = &[
    Route {
        operation: "list_autopilots",
        method: "GET",
        path: "/autopilots",
    },
    Route {
        operation: "get_autopilot",
        method: "GET",
        path: "/autopilots/:repository",
    },
    // One sentence read as settings, for a person to confirm. Starts nothing.
    Route {
        operation: "propose_autopilot",
        method: "POST",
        path: "/autopilots/:repository/proposal",
    },
    Route {
        operation: "start_autopilot",
        method: "POST",
        path: "/autopilots/:repository",
    },
    Route {
        operation: "pause_autopilot",
        method: "POST",
        path: "/autopilots/:repository/pause",
    },
    Route {
        operation: "resume_autopilot",
        method: "POST",
        path: "/autopilots/:repository/resume",
    },
    Route {
        operation: "stop_autopilot",
        method: "POST",
        path: "/autopilots/:repository/stop",
    },
    // A pick Autopilot set aside: a person dispatches it to the approval gate, or puts it away.
    Route {
        operation: "dispatch_set_aside",
        method: "POST",
        path: "/autopilots/:repository/picks/:pick_id/dispatch",
    },
    Route {
        operation: "dismiss_set_aside",
        method: "POST",
        path: "/autopilots/:repository/picks/:pick_id/dismiss",
    },
];
