//! The rows of [`SERVED`](super::SERVED) a pull request's panel reads and
//! writes through. Its own table for the parent's reason: past 1200 lines.

use super::Route;

pub(super) const ROUTES: &[Route] = &[
    Route {
        operation: "get_pull_request_detail",
        method: "GET",
        path: "/pull_request_details/:repository/:number",
    },
    Route {
        operation: "get_pull_request_file",
        method: "GET",
        path: "/pull_request_files/:repository/:number",
    },
    Route {
        operation: "write_pull_request",
        method: "POST",
        path: "/pull_request_writes/:repository",
    },
    Route {
        operation: "get_review_lists",
        method: "GET",
        path: "/review_lists",
    },
    Route {
        operation: "resolve_review_thread",
        method: "POST",
        path: "/review_threads/:repository",
    },
    Route {
        operation: "list_review_tools",
        method: "GET",
        path: "/review_tools/:repository",
    },
    Route {
        operation: "save_review_tool",
        method: "POST",
        path: "/review_tools/:repository",
    },
    Route {
        operation: "start_review_run",
        method: "POST",
        path: "/review_runs/:repository",
    },
    Route {
        operation: "get_review_runs",
        method: "GET",
        path: "/review_runs/:repository/:number",
    },
];
