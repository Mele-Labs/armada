//! What a person authors: the rows of [`SERVED`](super::SERVED) under
//! `/workflows`, `/triggers` and `/mods`. The list, one definition, and the acts
//! in the last segment, beside `/manifest/save_file`'s. `docs/concepts/trigger.md`,
//! `docs/concepts/mods.md`.

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
];
