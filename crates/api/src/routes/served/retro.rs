//! What got in the way, and what a person made of it: a Job's and a Session's retro, and the
//! Lessons page over every retro's items. Rows of [`SERVED`](super::SERVED).

use super::Route;

pub(super) const ROUTES: &[Route] = &[
    // A Session's retro: read, and written on a press. Under `/sessions` by the
    // Session's id, since a Session is not a Job and `/jobs/:job_id` would not
    // resolve it.
    Route {
        operation: "get_session_retro",
        method: "GET",
        path: "/sessions/:session_id/retro",
    },
    Route {
        operation: "write_session_retro",
        method: "POST",
        path: "/sessions/:session_id/retro",
    },
    // Every retro's items across Jobs, for the Lessons page. Not under
    // `/jobs`: an item is about a Job and the list is about none of them.
    Route {
        operation: "list_lessons",
        method: "GET",
        path: "/lessons",
    },
    Route {
        operation: "start_lesson_session",
        method: "POST",
        path: "/lessons/:lesson_id/session",
    },
    Route {
        operation: "disagree_lesson",
        method: "POST",
        path: "/lessons/:lesson_id/disagree",
    },
    // The Logbook: what Armada learned, and a person's acts on an entry.
    Route {
        operation: "get_logbook",
        method: "GET",
        path: "/logbook",
    },
    Route {
        operation: "teach_logbook_entry",
        method: "POST",
        path: "/logbook/entries",
    },
    Route {
        operation: "edit_logbook_entry",
        method: "POST",
        path: "/logbook/entries/:entry_id/edit",
    },
    Route {
        operation: "lock_logbook_entry",
        method: "POST",
        path: "/logbook/entries/:entry_id/lock",
    },
    Route {
        operation: "forget_logbook_entry",
        method: "POST",
        path: "/logbook/entries/:entry_id/forget",
    },
    // Teaching: words a person selected, read into a proposal, and the link rules they keep.
    Route {
        operation: "read_teaching",
        method: "POST",
        path: "/teachings/read",
    },
    Route {
        operation: "save_teaching",
        method: "POST",
        path: "/teachings",
    },
    Route {
        operation: "list_taught_links",
        method: "GET",
        path: "/teachings/links",
    },
    Route {
        operation: "forget_taught_link",
        method: "POST",
        path: "/teachings/links/:link_id/forget",
    },
    // Two model calls over the open items, neither of which stores anything.
    // `review` is a literal segment beside `:lesson_id`, which axum ranks first.
    Route {
        operation: "review_lessons",
        method: "POST",
        path: "/lessons/review",
    },
    Route {
        operation: "ask_lesson",
        method: "POST",
        path: "/lessons/:lesson_id/ask",
    },
];
