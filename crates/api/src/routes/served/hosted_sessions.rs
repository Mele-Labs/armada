//! The rows of [`SERVED`](super::SERVED) for a session Fleet hosts for Bridge,
//! since 23.49: starting one, talking to it, and what it holds.

use super::Route;

pub(super) const ROUTES: &[Route] = &[
    Route {
        operation: "start_session",
        method: "POST",
        path: "/sessions/start",
    },
    Route {
        operation: "send_session_message",
        method: "POST",
        path: "/sessions/message",
    },
    // Helm passing words to a live Session; only Helm's door session is offered it.
    Route {
        operation: "tell_session",
        method: "POST",
        path: "/helm/tell_session",
    },
    // The model and effort Helm's replies run on, and a file sent with one of its messages.
    Route {
        operation: "tune_helm",
        method: "POST",
        path: "/helm/tune",
    },
    Route {
        operation: "get_helm_file",
        method: "GET",
        path: "/helm/file",
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
];
