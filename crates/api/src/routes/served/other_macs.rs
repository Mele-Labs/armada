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
        operation: "share_preview_offers",
        method: "GET",
        path: "/other_mac/preview_offers",
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
        operation: "share_job_session",
        method: "GET",
        path: "/other_mac/session",
    },
    Route {
        operation: "share_job_artifacts",
        method: "GET",
        path: "/other_mac/artifacts",
    },
    Route {
        operation: "share_job_artifact_page",
        method: "GET",
        path: "/other_mac/artifact_page",
    },
    Route {
        operation: "share_job_margin",
        method: "GET",
        path: "/other_mac/margin",
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
        operation: "hold_other_mac_preview_offers",
        method: "PUT",
        path: "/other_macs/preview_offers",
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
        operation: "share_kit",
        method: "GET",
        path: "/other_mac/kit",
    },
    Route {
        operation: "hold_other_mac_kit",
        method: "PUT",
        path: "/other_macs/kit",
    },
    Route {
        operation: "share_home_layout",
        method: "GET",
        path: "/other_mac/home_layout",
    },
    Route {
        operation: "hold_other_mac_home_layout",
        method: "PUT",
        path: "/other_macs/home_layout",
    },
    Route {
        operation: "kit_settle_from_mac",
        method: "POST",
        path: "/other_mac/kit_settle",
    },
    Route {
        operation: "hold_refresh_claim",
        method: "POST",
        path: "/other_mac/refresh_claim",
    },
    Route {
        operation: "act_from_mac",
        method: "POST",
        path: "/other_mac/act",
    },
    Route {
        operation: "share_job_conversation",
        method: "GET",
        path: "/other_mac/conversation",
    },
    Route {
        operation: "move_job_to_mac",
        method: "POST",
        path: "/sessions/move_to_mac",
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
