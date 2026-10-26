//! The published event kinds: the rows of [`SERVED`](super::SERVED) on `/events`.

use super::Route;

pub(super) const ROUTES: &[Route] = &[
    // The other kind that names no Job, and it names no Drone or step either.
    // A Manifest is Fleet's own, so nothing on the Board moves when it arrives.
    Route {
        operation: "manifest.reread",
        method: "GET",
        path: "/events",
    },
    // Fleet's own list, carried whole, so a picker replaces it rather than patching.
    Route {
        operation: "repositories.changed",
        method: "GET",
        path: "/events",
    },
    // `get_merge_lines`' answer, whole, whenever a line moved on disk.
    Route {
        operation: "merge_lines.changed",
        method: "GET",
        path: "/events",
    },
    // The mods on this machine, whole, whenever one was written or switched.
    Route {
        operation: "mods.changed",
        method: "GET",
        path: "/events",
    },
    // settings.json, whole, after a save, a hand edit, or a hand edit refused.
    Route {
        operation: "settings.changed",
        method: "GET",
        path: "/events",
    },
    // A session after any fact about it, whole.
    Route {
        operation: "session.changed",
        method: "GET",
        path: "/events",
    },
    // Sleep mode after any change, whole.
    Route {
        operation: "sleep.changed",
        method: "GET",
        path: "/events",
    },
    // The plan's windows after a session read them moving, whole.
    Route {
        operation: "plan_usage.changed",
        method: "GET",
        path: "/events",
    },
    // A run of an agent about a pull request moved, whole. `docs/concepts/pull-review.md`.
    Route {
        operation: "review_run.changed",
        method: "GET",
        path: "/events",
    },
    // A pulled-down walk's place moved: checking out, starting, ready, failed or closed, whole.
    Route {
        operation: "review_walk.changed",
        method: "GET",
        path: "/events",
    },
    // The default model's step down after any change to it, whole.
    Route {
        operation: "model_downshift.changed",
        method: "GET",
        path: "/events",
    },
    // A repository's projects after any write to one, whole.
    Route {
        operation: "projects.changed",
        method: "GET",
        path: "/events",
    },
    // The agents' account signed out or in, or its sign-in started or ended, whole.
    Route {
        operation: "agent_sign_in.changed",
        method: "GET",
        path: "/events",
    },
    // A repository's Autopilot after any change to it, whole. #2092.
    Route {
        operation: "autopilot.changed",
        method: "GET",
        path: "/events",
    },
    // A Job's watches after any change to one, whole, as `list_watches` answers them.
    Route {
        operation: "watches.changed",
        method: "GET",
        path: "/events",
    },
    // A background act failed after its reply.
    Route {
        operation: "act.failed",
        method: "GET",
        path: "/events",
    },
    // One entry Fleet just kept for the timeline, for a client holding a window to append.
    Route {
        operation: "timeline.appended",
        method: "GET",
        path: "/events",
    },
    // One row of a hosted session's thread, appended or replaced.
    Route {
        operation: "session.row",
        method: "GET",
        path: "/events",
    },
    // A Job's shell opened or ended. Its output is `observe_session_run`'s.
    Route {
        operation: "terminal.opened",
        method: "GET",
        path: "/events",
    },
    Route {
        operation: "terminal.exited",
        method: "GET",
        path: "/events",
    },
    // A Studio after a write, whole, and a Studio deleted.
    Route {
        operation: "studio.changed",
        method: "GET",
        path: "/events",
    },
    Route {
        operation: "studio.deleted",
        method: "GET",
        path: "/events",
    },
    // Helm's act on a Studio, beside the `studio.changed` it made. `#1288`.
    Route {
        operation: "studio.helm_acted",
        method: "GET",
        path: "/events",
    },
    // A Helm session held inside a call the person's own settings do not cover,
    // and what became of it. `#1389`.
    Route {
        operation: "helm.asking_to_run",
        method: "GET",
        path: "/events",
    },
    Route {
        operation: "helm.call_answered",
        method: "GET",
        path: "/events",
    },
    // Helm's reading sent a person's words to a Session instead of to Helm.
    Route {
        operation: "helm.thought_routed",
        method: "GET",
        path: "/events",
    },
    // A person's run in the main checkout ending. What the run prints is
    // `observe_checkout_run`'s, never this stream's.
    Route {
        operation: "checkout_run.finished",
        method: "GET",
        path: "/events",
    },
    // A server's three lifecycle facts. Its output is `observe_server`'s.
    Route {
        operation: "server.starting",
        method: "GET",
        path: "/events",
    },
    Route {
        operation: "server.serving",
        method: "GET",
        path: "/events",
    },
    Route {
        operation: "server.exited",
        method: "GET",
        path: "/events",
    },
    // A run after any of its servers' three facts, whole.
    Route {
        operation: "app_run.changed",
        method: "GET",
        path: "/events",
    },
];
