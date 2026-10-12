//! The rows of [`SERVED`](super::SERVED) about what lies outside any Job: the
//! machine's sleep, and a repository's open issues read from its tracker.

use super::Route;

pub(super) const ROUTES: &[Route] = &[
    // Sleep mode: the switch, the night it keeps, and a correction of one decision.
    Route {
        operation: "get_sleep",
        method: "GET",
        path: "/sleep",
    },
    Route {
        operation: "set_sleep",
        method: "POST",
        path: "/sleep",
    },
    Route {
        operation: "override_sleep",
        method: "POST",
        path: "/sleep/override",
    },
    Route {
        operation: "review_sleep",
        method: "POST",
        path: "/sleep/review",
    },
    // A repository's open issues, read from its tracker. #91.
    Route {
        operation: "list_issues",
        method: "GET",
        path: "/issues/:repository",
    },
];
