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
        operation: "share_events",
        method: "GET",
        path: "/other_mac/events",
    },
    Route {
        operation: "share_job_thread",
        method: "GET",
        path: "/other_mac/thread",
    },
    // A whole list in place of the one held, so `PUT`.
    Route {
        operation: "hold_other_mac_sessions",
        method: "PUT",
        path: "/other_macs/sessions",
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
        operation: "forget_other_mac",
        method: "POST",
        path: "/other_macs/forget",
    },
];
