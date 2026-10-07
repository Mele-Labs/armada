//! What a person authors: the rows of [`SERVED`](super::SERVED) under
//! `/workflows` and `/triggers`. The list, one definition, and the acts in the
//! last segment, beside `/manifest/save_file`'s. `docs/concepts/trigger.md`.

use super::Route;

pub(super) const ROUTES: &[Route] = &[
    Route {
        operation: "list_workflows",
        method: "GET",
        path: "/workflows",
    },
    Route {
        operation: "get_workflow",
        method: "GET",
        path: "/workflows/definition",
    },
    Route {
        operation: "list_left_out_workflows",
        method: "GET",
        path: "/workflows/left_out",
    },
    // The act in the last segment, beside `/manifest/save_file`'s. Under
    // `/workflows` with the reads, since what it writes is what they list.
    Route {
        operation: "save_workflow",
        method: "POST",
        path: "/workflows/save",
    },
    Route {
        operation: "list_triggers",
        method: "GET",
        path: "/triggers",
    },
    Route {
        operation: "get_trigger",
        method: "GET",
        path: "/triggers/definition",
    },
    Route {
        operation: "save_trigger",
        method: "POST",
        path: "/triggers/save",
    },
    Route {
        operation: "remove_trigger",
        method: "POST",
        path: "/triggers/remove",
    },
    // A Job's own, so it is under the Job: where a failed Trigger's fix goes.
    Route {
        operation: "choose_trigger_fix",
        method: "POST",
        path: "/jobs/:job_id/choose_trigger_fix",
    },
    // The owner's two acts on a Trigger that holds the Job. Since 23.63.
    Route {
        operation: "rerun_trigger",
        method: "POST",
        path: "/jobs/:job_id/rerun_trigger",
    },
    Route {
        operation: "skip_trigger",
        method: "POST",
        path: "/jobs/:job_id/skip_trigger",
    },
    // Steps added to one Job. Since 23.59.
    Route {
        operation: "add_job_step",
        method: "POST",
        path: "/jobs/:job_id/add_job_step",
    },
    Route {
        operation: "remove_job_step",
        method: "POST",
        path: "/jobs/:job_id/remove_job_step",
    },
];
