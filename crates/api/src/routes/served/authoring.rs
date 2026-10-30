//! What a person authors: the rows of [`SERVED`](super::SERVED) under
//! `/workflows` and `/mods`. The list, one definition, and the acts
//! in the last segment, beside `/manifest/save_file`'s. `docs/concepts/mods.md`.

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
    Route {
        operation: "get_workflow_steps",
        method: "GET",
        path: "/workflows/steps",
    },
    // The act in the last segment, beside `/manifest/save_file`'s. Under
    // `/workflows` with the reads, since what it writes is what they list.
    Route {
        operation: "save_workflow",
        method: "POST",
        path: "/workflows/save",
    },
    Route {
        operation: "delete_workflow_copy",
        method: "POST",
        path: "/workflows/delete",
    },
    Route {
        operation: "reset_workflow_copy",
        method: "POST",
        path: "/workflows/reset",
    },
    Route {
        operation: "mark_not_a_drift",
        method: "POST",
        path: "/workflows/not_a_drift",
    },
    Route {
        operation: "list_workflow_changes",
        method: "GET",
        path: "/workflows/changes",
    },
    Route {
        operation: "ask_workflow_change",
        method: "POST",
        path: "/workflows/changes/ask",
    },
    Route {
        operation: "suggest_workflow_change",
        method: "POST",
        path: "/workflows/changes/suggest",
    },
    Route {
        operation: "propose_new_workflow",
        method: "POST",
        path: "/workflows/changes/propose_new",
    },
    Route {
        operation: "propose_workflow_change",
        method: "POST",
        path: "/workflows/changes/propose",
    },
    Route {
        operation: "approve_workflow_change",
        method: "POST",
        path: "/workflows/changes/approve",
    },
    Route {
        operation: "dismiss_workflow_change",
        method: "POST",
        path: "/workflows/changes/dismiss",
    },
    Route {
        operation: "send_back_workflow_change",
        method: "POST",
        path: "/workflows/changes/send_back",
    },
    Route {
        operation: "build_workflow_support",
        method: "POST",
        path: "/workflows/changes/build",
    },
    Route {
        operation: "propose_workflow_to_armada",
        method: "POST",
        path: "/workflows/propose",
    },
    // A mod is a folder on this machine, so none of these names a Job or a
    // repository but the promote, which names one in its body. The acts spell
    // themselves in the last segment, and the read that takes a name takes it as
    // `?name=`, as `/sessions/owner` takes its target.
    Route {
        operation: "list_mods",
        method: "GET",
        path: "/mods",
    },
    Route {
        operation: "scaffold_mod",
        method: "POST",
        path: "/mods/scaffold",
    },
    Route {
        operation: "set_mod_enabled",
        method: "POST",
        path: "/mods/enable",
    },
    Route {
        operation: "validate_mod",
        method: "GET",
        path: "/mods/validate",
    },
    Route {
        operation: "promote_mod",
        method: "POST",
        path: "/mods/promote",
    },
    Route {
        operation: "try_tracker_mod",
        method: "POST",
        path: "/mods/try",
    },
    // A Job on Armada's own repository, which the start finds by its name and no body names.
    Route {
        operation: "start_mod_session",
        method: "POST",
        path: "/mods/session",
    },
];
