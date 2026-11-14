//! His other Macs: the rows of [`SERVED`](super::SERVED) this Mac's Phone Gateway calls. `/other_mac/*` is
//! what this Mac shares, read to be sent; `/other_macs/*` is what another Mac shared, handed in to be kept.

use super::Route;

pub(super) const ROUTES: &[Route] = &[
    Route {
        operation: "share_sessions",
        method: "GET",
        path: "/other_mac/sessions",
    },
    Route {
        operation: "share_bug_reports",
        method: "GET",
        path: "/other_mac/bug_reports",
    },
    Route {
        operation: "share_projects",
        method: "GET",
        path: "/other_mac/projects",
    },
    Route {
        operation: "share_events",
        method: "GET",
        path: "/other_mac/events",
    },
    Route {
        operation: "share_job_thread",
        method: "GET",
        path: "/other_mac/thread",
    },
    Route {
        operation: "share_job_page",
        method: "GET",
        path: "/other_mac/page",
    },
    Route {
        operation: "open_other_mac_page",
        method: "POST",
        path: "/sessions/other_mac_page",
    },
    Route {
        operation: "share_job_terminal",
        method: "POST",
        path: "/other_mac/terminal",
    },
    Route {
        operation: "share_job_terminal_output",
        method: "GET",
        path: "/other_mac/terminal/observe",
    },
    // A whole list in place of the one held, so `PUT`.
    Route {
        operation: "hold_other_mac_sessions",
        method: "PUT",
        path: "/other_macs/sessions",
    },
    Route {
        operation: "hold_other_mac_projects",
        method: "PUT",
        path: "/other_macs/projects",
    },
    Route {
        operation: "hold_other_mac_bug_reports",
        method: "PUT",
        path: "/other_macs/bug_reports",
    },
    Route {
        operation: "hold_other_mac_event",
        method: "POST",
        path: "/other_macs/event",
    },
    Route {
        operation: "project_act_from_mac",
        method: "POST",
        path: "/other_mac/project_act",
    },
    Route {
        operation: "act_from_mac",
        method: "POST",
        path: "/other_mac/act",
    },
    Route {
        operation: "list_queued",
        method: "GET",
        path: "/queued",
    },
    Route {
        operation: "take_back_queued",
        method: "POST",
        path: "/queued/take_back",
    },
    Route {
        operation: "forget_other_mac",
        method: "POST",
        path: "/other_macs/forget",
    },
    Route {
        operation: "hold_mac_claims",
        method: "PUT",
        path: "/other_macs/claims",
    },
];
