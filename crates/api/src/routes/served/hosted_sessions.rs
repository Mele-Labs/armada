//! The rows of [`SERVED`](super::SERVED) for a session Fleet hosts for Bridge,
//! since 23.49: starting one, talking to it, what it holds, and its watches.

use super::Route;

pub(super) const ROUTES: &[Route] = &[
    Route {
        operation: "start_session",
        method: "POST",
        path: "/sessions/start",
    },
    Route {
        operation: "hand_to_session",
        method: "POST",
        path: "/sessions/hand_to_session",
    },
    Route {
        operation: "read_session_wait",
        method: "POST",
        path: "/sessions/read_wait",
    },
    Route {
        operation: "start_waiting_now",
        method: "POST",
        path: "/sessions/start_waiting",
    },
    Route {
        operation: "send_session_message",
        method: "POST",
        path: "/sessions/message",
    },
    Route {
        operation: "send_session_note",
        method: "POST",
        path: "/sessions/note",
    },
    Route {
        operation: "send_session_notes",
        method: "POST",
        path: "/sessions/notes",
    },
    // Helm passing words to a live Session; only Helm's door session is offered it.
    Route {
        operation: "tell_session",
        method: "POST",
        path: "/helm/tell_session",
    },
    Route {
        operation: "answer_session_ask",
        method: "POST",
        path: "/sessions/ask/answer",
    },
    Route {
        operation: "tune_session",
        method: "POST",
        path: "/sessions/tune",
    },
    Route {
        operation: "run_session_command",
        method: "POST",
        path: "/sessions/command",
    },
    Route {
        operation: "close_session",
        method: "POST",
        path: "/sessions/close",
    },
    Route {
        operation: "move_session",
        method: "POST",
        path: "/sessions/move",
    },
    Route {
        operation: "stop_session",
        method: "POST",
        path: "/sessions/stop",
    },
    Route {
        operation: "retry_handoff",
        method: "POST",
        path: "/sessions/handoff/retry",
    },
    Route {
        operation: "pause_session",
        method: "POST",
        path: "/sessions/pause",
    },
    Route {
        operation: "set_session_auto_land",
        method: "POST",
        path: "/sessions/auto_land",
    },
    Route {
        operation: "get_session_mcp",
        method: "GET",
        path: "/sessions/mcp",
    },
    Route {
        operation: "act_on_session_mcp",
        method: "POST",
        path: "/sessions/mcp/act",
    },
    Route {
        operation: "resume_session",
        method: "POST",
        path: "/sessions/resume",
    },
    Route {
        operation: "set_session_workflow",
        method: "POST",
        path: "/sessions/workflow",
    },
    Route {
        operation: "release_session_holds",
        method: "POST",
        path: "/sessions/release_holds",
    },
    Route {
        operation: "get_session",
        method: "GET",
        path: "/sessions/one",
    },
    Route {
        operation: "get_session_subagent",
        method: "GET",
        path: "/sessions/subagent",
    },
    Route {
        operation: "get_session_file",
        method: "GET",
        path: "/sessions/file",
    },
    Route {
        operation: "gate_session_call",
        method: "POST",
        path: "/sessions/gate",
    },
    Route {
        operation: "ask_from_terminal",
        method: "POST",
        path: "/sessions/ask/terminal",
    },
    Route {
        operation: "take_held_messages",
        method: "POST",
        path: "/sessions/held",
    },
    // Where two of the agents on a repository write the same lines. `docs/concepts/fleet.md`,
    // *Write clashes*.
    Route {
        operation: "list_write_clashes",
        method: "GET",
        path: "/clashes",
    },
    Route {
        operation: "read_clash_file",
        method: "GET",
        path: "/clashes/file",
    },
    // One file a Job has written, for the diff lens on its Field.
    Route {
        operation: "get_session_file_change",
        method: "GET",
        path: "/sessions/file_change",
    },
    // What a repository holds and where each agent has changed it, for the deck's map.
    // `docs/concepts/fleet.md`, *The territory*.
    Route {
        operation: "list_repository_areas",
        method: "GET",
        path: "/repository_areas",
    },
    Route {
        operation: "list_slot_touches",
        method: "GET",
        path: "/slot_touches",
    },
    Route {
        operation: "start_another_session",
        method: "POST",
        path: "/sessions/start_another",
    },
    Route {
        operation: "change_waiting",
        method: "POST",
        path: "/sessions/change_waiting",
    },
    Route {
        operation: "fork_this_session",
        method: "POST",
        path: "/sessions/fork_this",
    },
    // A command Fleet runs on a cadence for one Job, and the agent's weighing of what it reads.
    Route {
        operation: "list_watches",
        method: "GET",
        path: "/watches",
    },
    Route {
        operation: "start_watch",
        method: "POST",
        path: "/watches/start",
    },
    Route {
        operation: "weigh_watch",
        method: "POST",
        path: "/watches/weigh",
    },
    Route {
        operation: "stop_watch",
        method: "POST",
        path: "/watches/stop",
    },
];
