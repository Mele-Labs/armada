//! The published event kinds: the rows of [`SERVED`](super::SERVED) on `/events`.

use super::Route;

pub(super) const ROUTES: &[Route] = &[
    // Every event kind is served on the one socket, and every one is named:
    // `SERVED` is what a rule compares to the inventory, so a kind published
    // and not listed here is a kind no rule can see. The rule also compares
    // this group to `crates/ipc/src/event.rs`'s `Event` enum, which is the
    // closed set of kinds that can actually be published — a variant there
    // with no row here now fails the gate rather than reading as complete
    // while two kinds went unlisted.
    Route {
        operation: "job.created",
        method: "GET",
        path: "/events",
    },
    Route {
        operation: "job.state_changed",
        method: "GET",
        path: "/events",
    },
    Route {
        operation: "job.step_advanced",
        method: "GET",
        path: "/events",
    },
    Route {
        operation: "drone.spawned",
        method: "GET",
        path: "/events",
    },
    Route {
        operation: "drone.exited",
        method: "GET",
        path: "/events",
    },
    Route {
        operation: "job.files_changed",
        method: "GET",
        path: "/events",
    },
    Route {
        operation: "job.judging",
        method: "GET",
        path: "/events",
    },
    Route {
        operation: "job.checking",
        method: "GET",
        path: "/events",
    },
    Route {
        operation: "job.dry_run",
        method: "GET",
        path: "/events",
    },
    Route {
        operation: "evidence.submitted",
        method: "GET",
        path: "/events",
    },
    Route {
        operation: "job.asking",
        method: "GET",
        path: "/events",
    },
    Route {
        operation: "job.command_waiting",
        method: "GET",
        path: "/events",
    },
    Route {
        operation: "job.forgotten",
        method: "GET",
        path: "/events",
    },
    Route {
        operation: "job.landed",
        method: "GET",
        path: "/events",
    },
    // The forge's comments on a Job's pull request changed since the last
    // sweep found it open. `#661`.
    Route {
        operation: "job.remarks_changed",
        method: "GET",
        path: "/events",
    },
    // A Job was paused or resumed, which moves no status at a gate.
    Route {
        operation: "job.pause_changed",
        method: "GET",
        path: "/events",
    },
    // A Job's plan was recorded or a task changed. The counts ride along; the
    // plan is `get_job`'s. `#893`.
    Route {
        operation: "job.plan_changed",
        method: "GET",
        path: "/events",
    },
    // The one kind on this stream that names no Job. A proposal is the interval
    // before any exists, which is why it carries an id of its own.
    Route {
        operation: "proposal.moved",
        method: "GET",
        path: "/events",
    },
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
    // Helm writing a file in the repository's own checkout, which it does on a
    // person's ask and in no worktree. `#1373`.
    Route {
        operation: "helm.changed_checkout",
        method: "GET",
        path: "/events",
    },
    // A person's run ending. It names a Job and moves nothing on it; what the
    // run prints is `observe_run`'s, never this stream's.
    Route {
        operation: "run.finished",
        method: "GET",
        path: "/events",
    },
    // The same, for a run in the main checkout. Its own kind because it names
    // no Job, so a reader folding the one above by `job_id` cannot be given it.
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
];
