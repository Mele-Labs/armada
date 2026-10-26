//! What a person runs by hand and what stays running: the run sheet and its runs for a Job and
//! for the main checkout, and Servers. Rows of [`SERVED`](super::SERVED).

use super::Route;

pub(super) const ROUTES: &[Route] = &[
    // The same sheet for the main checkout, under `/manifest` beside
    // `get_manifest_reading` and for its reason: no Job is involved at all.
    Route {
        operation: "get_checkout_run_sheet",
        method: "GET",
        path: "/manifest/run_sheet",
    },
    Route {
        operation: "list_checkout_runs",
        method: "GET",
        path: "/manifest/runs",
    },
    Route {
        operation: "get_checkout_run_output",
        method: "GET",
        path: "/manifest/runs/:run_id/output",
    },
    Route {
        operation: "get_checkout_run_diff",
        method: "GET",
        path: "/manifest/runs/:run_id/diff",
    },
    Route {
        operation: "start_checkout_run",
        method: "POST",
        path: "/manifest/start_run",
    },
    Route {
        operation: "stop_checkout_run",
        method: "POST",
        path: "/manifest/stop_run",
    },
    Route {
        operation: "undo_checkout_run",
        method: "POST",
        path: "/manifest/undo_run",
    },
    Route {
        operation: "start_checkout_verify",
        method: "POST",
        path: "/manifest/start_verify",
    },
    // Servers: a Command that stays running. Not under `/jobs` because one
    // started with no Job belongs to none; the start and stop are segments of
    // the collection, `stop_proposal`'s shape, since an instance's id is all a
    // stop needs.
    Route {
        operation: "list_servers",
        method: "GET",
        path: "/servers",
    },
    Route {
        operation: "start_server",
        method: "POST",
        path: "/servers/start",
    },
    Route {
        operation: "stop_server",
        method: "POST",
        path: "/servers/stop",
    },
    // A repository's app, run from Bridge: several servers held as one run. Each process's
    // output is `observe_server`'s.
    Route {
        operation: "list_app_runs",
        method: "GET",
        path: "/app_runs",
    },
    Route {
        operation: "get_app_run_draft",
        method: "GET",
        path: "/app_runs/draft",
    },
    Route {
        operation: "start_app_run",
        method: "POST",
        path: "/app_runs/start",
    },
    Route {
        operation: "stop_app_run",
        method: "POST",
        path: "/app_runs/stop",
    },
    Route {
        operation: "restart_app_run",
        method: "POST",
        path: "/app_runs/restart",
    },
    Route {
        operation: "save_app_run",
        method: "POST",
        path: "/app_runs/save",
    },
];
