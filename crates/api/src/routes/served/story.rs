//! The rows of [`SERVED`](super::SERVED) the owner's words, his asks and a Job's story are read and changed through.

use super::Route;

pub(super) const ROUTES: &[Route] = &[
    Route {
        operation: "list_message_kinds",
        method: "GET",
        path: "/sessions/message_kinds",
    },
    Route {
        operation: "set_message_kind",
        method: "POST",
        path: "/sessions/message_kind/set",
    },
    Route {
        operation: "list_job_asks",
        method: "GET",
        path: "/sessions/asks",
    },
    Route {
        operation: "get_job_story",
        method: "GET",
        path: "/sessions/story",
    },
    Route {
        operation: "change_ask",
        method: "POST",
        path: "/asks/change",
    },
    Route {
        operation: "list_feature_asks",
        method: "GET",
        path: "/projects/asks",
    },
];
