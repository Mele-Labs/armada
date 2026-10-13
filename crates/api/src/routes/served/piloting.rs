//! A person taking a Job over and the three ways back: the rows of
//! [`SERVED`](super::SERVED) under `/jobs/:job_id` that `docs/concepts/pilot.md`
//! names.

use super::Route;

pub(super) const ROUTES: &[Route] = &[
    Route {
        operation: "take_over",
        method: "POST",
        path: "/jobs/:job_id/take_over",
    },
    Route {
        operation: "get_handoff",
        method: "GET",
        path: "/jobs/:job_id/handoff",
    },
    Route {
        operation: "submit_for_verification",
        method: "POST",
        path: "/jobs/:job_id/submit_for_verification",
    },
    Route {
        operation: "attest_complete",
        method: "POST",
        path: "/jobs/:job_id/attest_complete",
    },
    Route {
        operation: "close_as_superseded",
        method: "POST",
        path: "/jobs/:job_id/close_as_superseded",
    },
    // A person's hand on the Checks a gate is running, not a take-over: the
    // table is full at the 1200-line rule and this is the one beside it.
    Route {
        operation: "retry_check",
        method: "POST",
        path: "/jobs/:job_id/retry_check",
    },
    Route {
        operation: "skip_check",
        method: "POST",
        path: "/jobs/:job_id/skip_check",
    },
    Route {
        operation: "skip_checks",
        method: "POST",
        path: "/jobs/:job_id/skip_checks",
    },
];
