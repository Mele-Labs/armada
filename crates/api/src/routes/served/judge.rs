//! The rows of [`SERVED`](super::SERVED) a Judge is asked and its notes settled through. Its own table for
//! the parent's reason: past 1200 lines.

use super::Route;

pub(super) const ROUTES: &[Route] = &[
    Route {
        operation: "ask_judge",
        method: "POST",
        path: "/sessions/judge",
    },
    Route {
        operation: "send_judge_note",
        method: "POST",
        path: "/sessions/judge/send",
    },
    Route {
        operation: "dismiss_judge_note",
        method: "POST",
        path: "/sessions/judge/dismiss",
    },
    Route {
        operation: "mark_judge_read",
        method: "POST",
        path: "/sessions/judge/read",
    },
];
