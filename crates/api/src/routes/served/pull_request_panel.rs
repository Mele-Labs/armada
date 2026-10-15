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
];
