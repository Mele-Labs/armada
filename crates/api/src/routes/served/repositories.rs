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
    // What a person already has in their agent CLI, to be brought in: the folders, how one was run,
    // and the install of Fleet's mod. Under `/import` and not `/repositories`, which is what Fleet
    // serves; these are folders it does not serve yet.
    Route {
        operation: "list_import_folders",
        method: "GET",
        path: "/import/folders",
    },
    Route {
        operation: "get_import_runs",
        method: "GET",
        path: "/import/runs",
    },
    Route {
        operation: "install_agent_mod",
        method: "POST",
        path: "/import/install_mod",
    },
    // How a served repository runs, as its Jobs showed it, and where that is kept.
    Route {
        operation: "get_run_memory",
        method: "GET",
        path: "/repositories/runs",
    },
    Route {
        operation: "get_run_memory_place",
        method: "GET",
        path: "/repositories/runs/place",
    },
    Route {
        operation: "set_run_memory_place",
        method: "POST",
        path: "/repositories/runs/place",
    },
    Route {
        operation: "drop_run_fact",
        method: "POST",
        path: "/repositories/runs/drop",
    },
    Route {
        operation: "set_notice_mute",
        method: "POST",
        path: "/notices/mute",
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
    // The test map is the repository's, read for a list of paths that rides
    // the query string newline-separated — `docs/concepts/test-map.md`.
    Route {
        operation: "get_tests_reaching",
        method: "GET",
        path: "/test_map/reaching",
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
    // A repository's own tiles are kept where its runs are, so they are named as its runs are.
    Route {
        operation: "get_repository_flight_controls",
        method: "GET",
        path: "/repositories/flight_controls",
    },
    Route {
        operation: "save_repository_flight_controls",
        method: "POST",
        path: "/repositories/flight_controls",
    },
    Route {
        operation: "approve_repository_tile",
        method: "POST",
        path: "/repositories/flight_controls/approve",
    },
    // The person's own `armada.yml`, kept where their tiles are, so a setup Session can write it.
    Route {
        operation: "save_kit_manifest",
        method: "POST",
        path: "/repositories/kit_manifest",
    },
    // One route for every kind of piece: the body says which.
    Route {
        operation: "commit_to_repository",
        method: "POST",
        path: "/repositories/commit",
    },
    // Kit's allowlist is the machine's, so no Manifest scopes it. Read through
    // `get_kit_inventory`, which is why only the remove has a route here.
    Route {
        operation: "remove_kit_allowed_command",
        method: "POST",
        path: "/kit/allowed_commands/remove",
    },
    // Kit's Add and Remove for a plugin or a skill. Machine-wide, as the allowlist is.
    Route {
        operation: "look_at_kit_source",
        method: "POST",
        path: "/kit/pieces/look",
    },
    Route {
        operation: "install_kit_piece",
        method: "POST",
        path: "/kit/pieces/install",
    },
    Route {
        operation: "remove_kit_piece",
        method: "POST",
        path: "/kit/pieces/remove",
    },
    Route {
        operation: "restore_kit_piece",
        method: "POST",
        path: "/kit/pieces/restore",
    },
    // Share: pieces made into a package, read, added to the person's Kit, and the ones kept
    // unbound until a repository is chosen. A read of a package is a POST: the text is the body.
    Route {
        operation: "share_package",
        method: "POST",
        path: "/share/package",
    },
    Route {
        operation: "read_package",
        method: "POST",
        path: "/share/read",
    },
    Route {
        operation: "add_package",
        method: "POST",
        path: "/share/add",
    },
    Route {
        operation: "list_unbound_pieces",
        method: "GET",
        path: "/share/unbound",
    },
    Route {
        operation: "bind_unbound_piece",
        method: "POST",
        path: "/share/bind",
    },
    // What applies to a repository of the pieces a Workspace scopes: Workspaces, lifecycles and rules.
    Route {
        operation: "list_repository_workspaces",
        method: "GET",
        path: "/workspaces",
    },
    Route {
        operation: "list_workspaces",
        method: "GET",
        path: "/workspaces/all",
    },
    Route {
        operation: "list_lifecycles",
        method: "GET",
        path: "/lifecycles",
    },
    Route {
        operation: "list_lifecycle_rules",
        method: "GET",
        path: "/lifecycles/rules",
    },
    Route {
        operation: "list_autonomy_defaults",
        method: "GET",
        path: "/autonomy",
    },
    Route {
        operation: "list_keep_drafts",
        method: "GET",
        path: "/lifecycles/keep_drafts",
    },
    // Saved in Kit's folders and nowhere else: the repository's own copy is `commit_to_repository`'s.
    Route {
        operation: "save_lifecycle_piece",
        method: "POST",
        path: "/lifecycles/pieces/save",
    },
    Route {
        operation: "delete_lifecycle_piece",
        method: "POST",
        path: "/lifecycles/pieces/delete",
    },
];
