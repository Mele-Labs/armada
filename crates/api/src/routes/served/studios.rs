//! A repository's Studios: the rows of [`SERVED`](super::SERVED) under `/studios`.

use super::Route;

pub(super) const ROUTES: &[Route] = &[
    // A repository's Studios, `#1285`: the collection reads `?manifest_id=`,
    // a member is its id, and each act is spelled in the last segment without
    // `studio_`, which the segment before it already says.
    Route {
        operation: "list_studios",
        method: "GET",
        path: "/studios",
    },
    Route {
        operation: "create_studio",
        method: "POST",
        path: "/studios/create",
    },
    Route {
        operation: "get_studio",
        method: "GET",
        path: "/studios/:studio_id",
    },
    // The picture one Note kept, answered as the file. One segment and not two,
    // unlike a step's frame: a Studio keeps one frame per node under the node's
    // own id, so the node names the file and the record supplies its name.
    Route {
        operation: "get_studio_frame",
        method: "GET",
        path: "/studios/:studio_id/frames/:node_id",
    },
    Route {
        operation: "rename_studio",
        method: "POST",
        path: "/studios/:studio_id/rename",
    },
    Route {
        operation: "delete_studio",
        method: "POST",
        path: "/studios/:studio_id/delete",
    },
    Route {
        operation: "add_studio_node",
        method: "POST",
        path: "/studios/:studio_id/add_node",
    },
    Route {
        operation: "capture_studio_note",
        method: "POST",
        path: "/studios/:studio_id/capture_note",
    },
    Route {
        operation: "move_studio_node",
        method: "POST",
        path: "/studios/:studio_id/move_node",
    },
    Route {
        operation: "remove_studio_nodes",
        method: "POST",
        path: "/studios/:studio_id/remove_nodes",
    },
    Route {
        operation: "propose_studio_edge",
        method: "POST",
        path: "/studios/:studio_id/propose_edge",
    },
    Route {
        operation: "decide_studio_edge",
        method: "POST",
        path: "/studios/:studio_id/decide_edge",
    },
    Route {
        operation: "group_studio_nodes",
        method: "POST",
        path: "/studios/:studio_id/group_nodes",
    },
    Route {
        operation: "defer_on_studio",
        method: "POST",
        path: "/studios/:studio_id/defer",
    },
    Route {
        operation: "write_up_studio_node",
        method: "POST",
        path: "/studios/:studio_id/write_up",
    },
    Route {
        operation: "edit_studio_draft",
        method: "POST",
        path: "/studios/:studio_id/edit_draft",
    },
    Route {
        operation: "edit_studio_link",
        method: "POST",
        path: "/studios/:studio_id/edit_link",
    },
    Route {
        operation: "edit_studio_sketch",
        method: "POST",
        path: "/studios/:studio_id/edit_sketch",
    },
    Route {
        operation: "settle_contradiction",
        method: "POST",
        path: "/studios/:studio_id/settle",
    },
    Route {
        operation: "dispatch_studio_draft",
        method: "POST",
        path: "/studios/:studio_id/dispatch_draft",
    },
    Route {
        operation: "ask_scout",
        method: "POST",
        path: "/studios/:studio_id/ask_scout",
    },
    Route {
        operation: "start_scout",
        method: "POST",
        path: "/studios/:studio_id/start_scout",
    },
    Route {
        operation: "stop_scout",
        method: "POST",
        path: "/studios/:studio_id/stop_scout",
    },
    // A Link read in, `#1293`. `read_in` rather than `read_in_link`: the node
    // the body names is the Link.
    Route {
        operation: "read_in_link",
        method: "POST",
        path: "/studios/:studio_id/read_in",
    },
    // A run started from a Studio, `#1289`. `start_run` rather than
    // `start_studio_run`: the segment before it says which Studio.
    Route {
        operation: "start_studio_run",
        method: "POST",
        path: "/studios/:studio_id/start_run",
    },
    // A server started from a Studio, `#1345`. Beside `start_run` and not
    // folded into it: a server is held rather than run, and the two answers
    // are read back by different readers.
    Route {
        operation: "start_studio_server",
        method: "POST",
        path: "/studios/:studio_id/start_server",
    },
];
