//! The rows of [`SERVED`](super::SERVED) about what lies outside any Job: the
//! machine's sleep, and a repository's open issues read from its tracker.

use super::Route;

pub(super) const ROUTES: &[Route] = &[
    // The plan's windows, from the newest reading of any session.
    Route {
        operation: "get_plan_usage",
        method: "GET",
        path: "/plan_usage",
    },
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
    // What happened across the Fleet, windowed by instant: three optional bounds, so a query string.
    Route {
        operation: "get_fleet_timeline",
        method: "GET",
        path: "/timeline",
    },
    // Which served repository a request's words are about: typed text, so a
    // query string for `search_files`' reason, and under `/repositories`
    // beside the list it picks from.
    Route {
        operation: "read_request_repository",
        method: "GET",
        path: "/repositories/of_request",
    },
    // A repository's open issues, read from its tracker. #91.
    Route {
        operation: "list_issues",
        method: "GET",
        path: "/issues/:repository",
    },
];
