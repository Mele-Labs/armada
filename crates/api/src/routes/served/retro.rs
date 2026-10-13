//! What got in the way, and what a person made of it: a Job's and a Session's retro, and the
//! Lessons page over every retro's items. Rows of [`SERVED`](super::SERVED).

use super::Route;

pub(super) const ROUTES: &[Route] = &[
    // What got in the way while the Job ran, and the record it was read from.
    // `docs/concepts/retro.md`.
    Route {
        operation: "get_job_retro",
        method: "GET",
        path: "/jobs/:job_id/retro",
    },
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
    // A person's answer to one item. Under `/lessons`, by the item's own id,
    // because the item is what is acted on and its Job is only where it came
    // from.
    Route {
        operation: "agree_lesson",
        method: "POST",
        path: "/lessons/:lesson_id/agree",
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
