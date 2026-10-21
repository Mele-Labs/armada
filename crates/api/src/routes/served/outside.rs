//! The rows of [`SERVED`](super::SERVED) about what lies outside any Job: the
//! machine's sleep, a repository's open issues read from its tracker, and Requests: an address
//! anywhere, sent to filled from a repository's environments.

use super::Route;

pub(super) const ROUTES: &[Route] = &[
    // The plan's windows, from the newest reading of any session.
    Route {
        operation: "get_plan_usage",
        method: "GET",
        path: "/plan_usage",
    },
    // The default model's step down as the weekly window runs out, and the four things the owner does to it.
    Route {
        operation: "get_model_downshift",
        method: "GET",
        path: "/model_downshift",
    },
    Route {
        operation: "accept_downshift",
        method: "POST",
        path: "/model_downshift/accept",
    },
    Route {
        operation: "restore_downshift",
        method: "POST",
        path: "/model_downshift/restore",
    },
    Route {
        operation: "keep_downshift",
        method: "POST",
        path: "/model_downshift/keep",
    },
    Route {
        operation: "settle_downshift",
        method: "POST",
        path: "/model_downshift/settle",
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
    // One slot's leases and checkouts: the repository and the slot are both asked for, so a query string.
    Route {
        operation: "get_slot_history",
        method: "GET",
        path: "/slot_history",
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
    // One issue by reference, closed ones included, for a # that names one the list does not hold.
    Route {
        operation: "get_issue",
        method: "GET",
        path: "/issues/:repository/:reference",
    },
    // Requests. A repository's two files, read on each call; a secret's value is the Keychain's.
    Route {
        operation: "send_request",
        method: "POST",
        path: "/requests/send",
    },
    Route {
        operation: "get_requests",
        method: "GET",
        path: "/requests",
    },
    Route {
        operation: "save_request_collection",
        method: "POST",
        path: "/requests/collections",
    },
    Route {
        operation: "save_request_to_collection",
        method: "POST",
        path: "/requests/collections/request",
    },
    Route {
        operation: "save_request_environment",
        method: "POST",
        path: "/requests/environments",
    },
];
