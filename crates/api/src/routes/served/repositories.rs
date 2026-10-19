//! Repositories and what is written about them: the Manifest Fleet runs on, its drift and
//! its file, Scan and the proposals it makes, every repository served with its merge line,
//! and Kit's servers and allowlist. Rows of [`SERVED`](super::SERVED).

use super::Route;

pub(super) const ROUTES: &[Route] = &[
    // Fleet's reading of its own Manifest, and singular where `/manifests` is
    // plural on purpose: that route lists what Fleet holds, and this one is the
    // single `armada.yml` Fleet is running on and watching. Not under `/jobs`
    // for `get_capacity`'s reason — a reload belongs to no Job at all.
    Route {
        operation: "get_manifest_reading",
        method: "GET",
        path: "/manifest/reading",
    },
    // The other half of the reading above, and a different question: that one
    // is what Fleet could not adopt from the file, this is whether the
    // repository still has what the file names. Under `/manifest` beside it for
    // its reason — a Fleet serves one repository and neither belongs to a Job.
    Route {
        operation: "get_manifest_drift",
        method: "GET",
        path: "/manifest/drift",
    },
    // Scan, and not under `/manifest`: it reads a repository nobody has written
    // an `armada.yml` for, so there is no Manifest for the path to be under.
    // Every repository served, and one more by folder. Beside Scan, which
    // reads one of them by the root these answer with.
    Route {
        operation: "list_repositories",
        method: "GET",
        path: "/repositories",
    },
    Route {
        operation: "add_repository",
        method: "POST",
        path: "/repositories/add",
    },
    // Each served repository's merge line, beside the list it is keyed by. Not
    // under `/repositories`: that is the list and its two acts.
    Route {
        operation: "get_merge_lines",
        method: "GET",
        path: "/merge_lines",
    },
    // One Check's log on that line, as it is written. Named by the line's own
    // three words in the query, never by a path: a root is a path itself.
    Route {
        operation: "observe_land_check",
        method: "GET",
        path: "/merge_lines/checks/observe",
    },
    // One uncommitted file of a held checkout: the repository and the path are both asked for, so a query string.
    Route {
        operation: "checkout_diff",
        method: "GET",
        path: "/merge_lines/checkout_diff",
    },
    // The one pull a held checkout is offered, from the card on that line.
    Route {
        operation: "catch_up_checkout",
        method: "POST",
        path: "/merge_lines/catch_up",
    },
    // The same hold's other act: its uncommitted files up as a pull request.
    Route {
        operation: "checkout_to_pull_request",
        method: "POST",
        path: "/merge_lines/checkout_to_pull_request",
    },
    Route {
        operation: "clone_repository",
        method: "POST",
        path: "/repositories/clone",
    },
    Route {
        operation: "create_project",
        method: "POST",
        path: "/repositories/create",
    },
    Route {
        operation: "get_repository_scan",
        method: "GET",
        path: "/repository/scan",
    },
    // What Scan's findings become, beside it. The edit spells its act in the last segment.
    Route {
        operation: "get_manifest_proposals",
        method: "GET",
        path: "/repository/proposals",
    },
    Route {
        operation: "edit_manifest_proposal",
        method: "POST",
        path: "/repository/edit_proposal",
    },
    // The only route here that makes a file in the main checkout where there was none.
    Route {
        operation: "write_manifest_proposal",
        method: "POST",
        path: "/repository/write_proposal",
    },
    // The file itself, where the two rows above are readings of it. Singular
    // and beside them for their reason, and never `/manifests/:id/file`: a
    // Fleet resolves its own `armada.yml`, so there is no id to carry and no
    // path a caller could compose.
    Route {
        operation: "get_manifest_file",
        method: "GET",
        path: "/manifest/file",
    },
    // The write, and the one route on this table that changes a tracked file in
    // the main checkout. It spells the act in its last segment the way
    // `start_checkout_run` does, so the read above stays the bare noun.
    Route {
        operation: "save_manifest_file",
        method: "POST",
        path: "/manifest/save_file",
    },
    // A form's edits to the same file, as edits: keys and values, never text.
    // Beside the save and for its reason, with the act in the last segment.
    Route {
        operation: "edit_manifest",
        method: "POST",
        path: "/manifest/edit",
    },
    // The `@` mention popup's read. `?q=` rather than a path segment: the
    // query is a person's typed text, empty the instant they type `@` and
    // before anything follows it, and a path segment cannot carry that. A
    // Fleet serves one repository, so nothing here names one.
    Route {
        operation: "search_files",
        method: "GET",
        path: "/manifest/files",
    },
    // A rule a person always-allowed for the repository is Fleet's own, kept
    // per Manifest rather than any one Job's — `#836`. The remove spells the
    // act in its last segment for `save_manifest_file`'s reason.
    Route {
        operation: "get_repository_allowed_commands",
        method: "GET",
        path: "/manifest/allowed_commands",
    },
    Route {
        operation: "remove_repository_allowed_command",
        method: "POST",
        path: "/manifest/allowed_commands/remove",
    },
    // Kit's servers, machine-wide, with the Manifest scoping the second tier
    // rather than naming a resource — `#1275`. Each act spells itself in its
    // last segment, as the remove above does.
    // The setup a person already has, read to be shown — `#1491`. Machine-wide
    // like the servers above, and under `/kit` for the surface that draws both.
    Route {
        operation: "get_kit_inventory",
        method: "GET",
        path: "/kit/inventory",
    },
    Route {
        operation: "get_kit_servers",
        method: "GET",
        path: "/kit/servers",
    },
    Route {
        operation: "add_kit_server",
        method: "POST",
        path: "/kit/servers/add",
    },
    Route {
        operation: "forget_kit_server",
        method: "POST",
        path: "/kit/servers/forget",
    },
    Route {
        operation: "set_kit_server_reach",
        method: "POST",
        path: "/kit/servers/reach",
    },
    Route {
        operation: "set_manifest_server_reach",
        method: "POST",
        path: "/kit/servers/manifest_reach",
    },
    // The flight controls are the machine's too: one file in Kit's home.
    Route {
        operation: "get_flight_controls",
        method: "GET",
        path: "/kit/flight_controls",
    },
    Route {
        operation: "save_flight_controls",
        method: "POST",
        path: "/kit/flight_controls",
    },
    // Kit's allowlist is the machine's, so no Manifest scopes it. Read through
    // `get_kit_inventory`, which is why only the remove has a route here.
    Route {
        operation: "remove_kit_allowed_command",
        method: "POST",
        path: "/kit/allowed_commands/remove",
    },
];
