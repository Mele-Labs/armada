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
        operation: "get_pull_request_patches",
        method: "GET",
        path: "/pull_request_patches/:repository/:number",
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
    Route {
        operation: "start_review_search",
        method: "POST",
        path: "/review_searches",
    },
    Route {
        operation: "get_review_searches",
        method: "GET",
        path: "/review_searches",
    },
    Route {
        operation: "forget_review_search",
        method: "POST",
        path: "/review_searches/forget",
    },
    Route {
        operation: "find_walk_place",
        method: "GET",
        path: "/walk_finds/:repository/:number",
    },
    Route {
        operation: "list_walk_places",
        method: "GET",
        path: "/walk_places",
    },
    Route {
        operation: "save_walk_place",
        method: "POST",
        path: "/walk_places/:repository",
    },
    Route {
        operation: "forget_walk_place",
        method: "POST",
        path: "/walk_places/:repository/:id/forget",
    },
    Route {
        operation: "list_pull_request_notes",
        method: "GET",
        path: "/pull_request_notes/:repository/:number",
    },
    Route {
        operation: "save_pull_request_note",
        method: "POST",
        path: "/pull_request_notes/:repository/:number",
    },
    Route {
        operation: "remove_pull_request_note",
        method: "POST",
        path: "/pull_request_notes/:repository/:number/:id/remove",
    },
    Route {
        operation: "send_pull_request_notes",
        method: "POST",
        path: "/pull_request_notes/:repository/:number/sent",
    },
    Route {
        operation: "open_review_walk",
        method: "POST",
        path: "/review_walks/:repository",
    },
    Route {
        operation: "close_review_walk",
        method: "POST",
        path: "/review_walks/:repository/:id/close",
    },
];
