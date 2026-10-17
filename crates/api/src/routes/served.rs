//! The inventory: every operation M1 serves, declared once as data.
//!
//! **Split out of `crate::routes` when that file crossed the 900-line rule.**
//! The router and the table it is compared against are two different things,
//! and the file named for the router now holds the router. The gate rule reads
//! both halves as one text — `xtask/src/rules_protocol.rs`.
//!
//! **Split again at the 1200-line rule**, into tables joined at compile time: Studios' in
//! [`studios`](mod@studios), event kinds in [`events`](mod@events), workflows' and Triggers' in [`authoring`](mod@authoring), settings in
//! [`settings`](mod@settings), Autopilot's in [`autopilot`](mod@autopilot), a pull request panel's
//! in [`pull_request_panel`](mod@pull_request_panel), repositories and Kit in
//! [`repositories`](mod@repositories), retros and Lessons in [`retro`](mod@retro), runs and Servers
//! in [`runs`](mod@runs), and every other row here. Each is a subject a reader opens on its own;
//! [`SERVED`] stays one slice. The gate rule reads every file — `xtask/src/rules_protocol.rs`.

mod authoring;
mod autopilot;
mod events;
mod hosted_sessions;
mod outside;
mod pull_request_panel;
mod repositories;
mod retro;
mod runs;
mod settings;
mod studios;

/// One operation, and where it is served.
///
/// `operation` is the key in `crates/ipc/operations/`, spelled exactly as
/// that file spells it, so comparing the two needs a set lookup and no mapping.
#[derive(Clone, Copy)]
pub struct Route {
    pub operation: &'static str,
    pub method: &'static str,
    pub path: &'static str,
}

/// The operations M1 serves — a deliberate subset of the inventory, not all of it.
///
/// The rest of the inventory, the `/v0` lifeboat and protocol-mismatch handling are
/// the Ship milestone's. Nothing here stubs them: a route that answers with a
/// placeholder is worse than one that 404s, because a client cannot tell the
/// difference between not built and not working.
///
/// An act's row spells its key in the last segment where it can: not
/// `redispatch`, with no `redispatch_drone` to tell apart, nor Pulse's kills,
/// on paths Bridge set first (`crate::processes`). The lifeboat's `POST
/// /v0/jobs/:id/kill` is `kill_job` under a frozen prefix, by design.
pub const SERVED: &[Route] = &joined::<
    {
        ROUTES.len()
            + studios::ROUTES.len()
            + events::ROUTES.len()
            + authoring::ROUTES.len()
            + settings::ROUTES.len()
            + outside::ROUTES.len()
            + hosted_sessions::ROUTES.len()
            + autopilot::ROUTES.len()
            + pull_request_panel::ROUTES.len()
            + repositories::ROUTES.len()
            + retro::ROUTES.len()
            + runs::ROUTES.len()
    },
>([
    ROUTES,
    studios::ROUTES,
    events::ROUTES,
    authoring::ROUTES,
    settings::ROUTES,
    outside::ROUTES,
    hosted_sessions::ROUTES,
    autopilot::ROUTES,
    pull_request_panel::ROUTES,
    repositories::ROUTES,
    retro::ROUTES,
    runs::ROUTES,
]);

/// The twelve tables as one, in order. A count that disagrees with `N` fails the build.
const fn joined<const N: usize>(tables: [&[Route]; 12]) -> [Route; N] {
    // Filled from the first row: a literal `Route` here would read as a row to the gate rule.
    let mut out = [tables[0][0]; N];
    let (mut table, mut at) = (0, 0);
    while table < tables.len() {
        let mut row = 0;
        while row < tables[table].len() {
            out[at] = tables[table][row];
            at += 1;
            row += 1;
        }
        table += 1;
    }
    assert!(at == N, "the joined table is the sum of its parts");
    out
}

/// Every row no other table here holds.
const ROUTES: &[Route] = &[
    Route {
        operation: "list_jobs",
        method: "GET",
        path: "/jobs",
    },
    Route {
        operation: "get_job",
        method: "GET",
        path: "/jobs/:job_id",
    },
    // The build Fleet runs on, beside `/capacity` and for its reason: it is
    // Fleet's, not a Job's. The change spells its act in the last segment the
    // way `/limits/save` does.
    Route {
        operation: "get_fleet_build",
        method: "GET",
        path: "/fleet/build",
    },
    Route {
        operation: "change_fleet_build",
        method: "POST",
        path: "/fleet/build/change",
    },
    // One repository's Helm Session, named by `?manifest_id=`.
    Route {
        operation: "get_helm_session",
        method: "GET",
        path: "/helm/session",
    },
    Route {
        operation: "ask_helm",
        method: "POST",
        path: "/helm/ask",
    },
    // The session as one record, `#1367`. Beside the Session it reports
    // on, not under `/debug`: there is no such surface, and the subject is the
    // Session.
    Route {
        operation: "get_helm_debug_info",
        method: "GET",
        path: "/helm/debug",
    },
    Route {
        operation: "start_helm_fresh",
        method: "POST",
        path: "/helm/start_fresh",
    },
    // A thought typed into Helm's box, read for where it belongs and then recorded where it went.
    Route {
        operation: "read_thought",
        method: "POST",
        path: "/helm/read_thought",
    },
    Route {
        operation: "record_thought_sent",
        method: "POST",
        path: "/helm/thought_sent",
    },
    // The permission path, `#1389`: the door calls the first, a person answers
    // the third, and the second is what a Bridge started mid-wait reads.
    Route {
        operation: "ask_the_person",
        method: "POST",
        path: "/helm/permission",
    },
    Route {
        operation: "list_helm_calls",
        method: "GET",
        path: "/helm/calls",
    },
    Route {
        operation: "answer_helm_call",
        method: "POST",
        path: "/helm/calls/answer",
    },
    // The path taken, under the Job that took it. `get_job_events` drops
    // `get_` and `job_` for the reason `redispatch` drops `_job`: the segment
    // before it already names the Job. It is not `/events`, which is the
    // global stream and carries no history at all.
    Route {
        operation: "get_job_events",
        method: "GET",
        path: "/jobs/:job_id/events",
    },
    // What Fleet did to the Job, settled: the socket's backfill, answered once.
    // **The one row whose last segment is not its key**, and the reason is the
    // route next door. Elsewhere a log has two routes — `<noun>/output` for the
    // reading and `<noun>/observe` for the stream — and here the stream took
    // the bare noun before there was a reading to tell it from. So the noun
    // stays where Bridge already calls it and the settled half names itself,
    // rather than moving a route Bridge is drawing from.
    Route {
        operation: "get_job_log",
        method: "GET",
        path: "/jobs/:job_id/log/read",
    },
    Route {
        operation: "get_diff",
        method: "GET",
        path: "/jobs/:job_id/diff",
    },
    // The ledger of agent sessions a person runs: the harness's intake, then the
    // two reads. `docs/concepts/session.md`.
    // `armada need`'s act and the read beside it, on one path. `docs/capabilities/needs.md`.
    Route {
        operation: "act_on_need",
        method: "POST",
        path: "/needs",
    },
    Route {
        operation: "list_needs",
        method: "GET",
        path: "/needs",
    },
    // A pull request by repository and number, and the acts a Session takes on
    // one. Beside `merge_pull_request`, which is a Job's and not this.
    Route {
        operation: "get_pull_request",
        method: "GET",
        path: "/pull_requests/:repository/:number",
    },
    Route {
        operation: "ready_pull_request",
        method: "POST",
        path: "/pull_requests/:repository/:number/ready",
    },
    Route {
        operation: "merge_pull_request_by_number",
        method: "POST",
        path: "/pull_requests/:repository/:number/merge",
    },
    Route {
        operation: "enable_auto_merge",
        method: "POST",
        path: "/pull_requests/:repository/:number/auto_merge",
    },
    Route {
        operation: "report_session",
        method: "POST",
        path: "/sessions/report",
    },
    Route {
        operation: "list_sessions",
        method: "GET",
        path: "/sessions",
    },
    Route {
        operation: "who_owns",
        method: "GET",
        path: "/sessions/owner",
    },
    Route {
        operation: "rename_session",
        method: "POST",
        path: "/sessions/rename",
    },
    Route {
        operation: "start_session_run",
        method: "POST",
        path: "/sessions/run/start",
    },
    Route {
        operation: "stop_session_run",
        method: "POST",
        path: "/sessions/run/stop",
    },
    Route {
        operation: "write_session_run",
        method: "POST",
        path: "/sessions/run/write",
    },
    Route {
        operation: "resize_session_run",
        method: "POST",
        path: "/sessions/run/resize",
    },
    Route {
        operation: "observe_session_run",
        method: "GET",
        path: "/sessions/run/:run_id/observe",
    },
    Route {
        operation: "show_window",
        method: "POST",
        path: "/sessions/window",
    },
    Route {
        operation: "show_sketch",
        method: "POST",
        path: "/sessions/sketch",
    },
    Route {
        operation: "record_plan",
        method: "POST",
        path: "/sessions/plan",
    },
    Route {
        operation: "approve_plan",
        method: "POST",
        path: "/sessions/plan/approve",
    },
    Route {
        operation: "judge_session",
        method: "POST",
        path: "/sessions/judge",
    },
    Route {
        operation: "send_concern",
        method: "POST",
        path: "/sessions/judge/send",
    },
    Route {
        operation: "dismiss_concern",
        method: "POST",
        path: "/sessions/judge/dismiss",
    },
    Route {
        operation: "send_plan_changes",
        method: "POST",
        path: "/sessions/plan/changes",
    },
    Route {
        operation: "report_session_check",
        method: "POST",
        path: "/sessions/checks",
    },
    Route {
        operation: "ask_for_slot",
        method: "POST",
        path: "/sessions/slot_wanted",
    },
    Route {
        operation: "get_session_check_output",
        method: "GET",
        path: "/sessions/checks/:run/output",
    },
    Route {
        operation: "observe_session_check",
        method: "GET",
        path: "/sessions/checks/:run/observe",
    },
    Route {
        operation: "stop_session_check",
        method: "POST",
        path: "/sessions/checks/:run/stop",
    },
    Route {
        operation: "force_stop_session_check",
        method: "POST",
        path: "/sessions/checks/:run/force_stop",
    },
    Route {
        operation: "ask_in_window",
        method: "POST",
        path: "/sessions/window/ask",
    },
    Route {
        operation: "window_closed",
        method: "POST",
        path: "/sessions/window/closed",
    },
    Route {
        operation: "take_window_sign_in",
        method: "POST",
        path: "/sessions/window/sign_in",
    },
    Route {
        operation: "window_failed",
        method: "POST",
        path: "/sessions/window/failed",
    },
    Route {
        operation: "claim_pull_request",
        method: "POST",
        path: "/sessions/claim_pull_request",
    },
    Route {
        operation: "take_main_red",
        method: "POST",
        path: "/sessions/take_main_red",
    },
    Route {
        operation: "waiting_for",
        method: "POST",
        path: "/sessions/waiting",
    },
    Route {
        operation: "all_done",
        method: "POST",
        path: "/sessions/done",
    },
    Route {
        operation: "finish_up",
        method: "POST",
        path: "/sessions/finish_up",
    },
    Route {
        operation: "mark_annotation_done",
        method: "POST",
        path: "/sessions/annotation_done",
    },
    Route {
        operation: "answer_waiting",
        method: "POST",
        path: "/sessions/waiting/answer",
    },
    Route {
        operation: "dismiss_waiting",
        method: "POST",
        path: "/sessions/waiting/dismiss",
    },
    Route {
        operation: "list_manifests",
        method: "GET",
        path: "/manifests",
    },
    Route {
        operation: "list_models",
        method: "GET",
        path: "/models",
    },
    // What Fleet is holding disk for, read across every Job at once. Not under
    // `/jobs/:job_id` and not a field on one: the question is which of these to
    // give back, which is asked of the set and answered by comparing them.
    Route {
        operation: "list_worktrees",
        method: "GET",
        path: "/worktrees",
    },
    // The pool beside them, reshaped on this machine: a slot added, removed,
    // closed or reopened.
    Route {
        operation: "change_slot_pool",
        method: "POST",
        path: "/worktrees/slots",
    },
    // An act on a stranded slot of that pool: the Scout that reads it, and
    // what a person then does with the work.
    Route {
        operation: "rescue_slot",
        method: "POST",
        path: "/worktrees/slots/rescue",
    },
    Route {
        operation: "kill_job",
        method: "POST",
        path: "/jobs/:job_id/kill_job",
    },
    // Real deletion, and the only route on this table that is: every other
    // command moves a Job further, and this removes the row.
    Route {
        operation: "forget_job",
        method: "POST",
        path: "/jobs/:job_id/forget_job",
    },
    // The other half of the row above, and its own route because that row says
    // why: one call with two unrelated things to fail at is worse than two
    // calls. This one takes the disk and leaves the record.
    Route {
        operation: "reclaim_worktree",
        method: "POST",
        path: "/jobs/:job_id/reclaim_worktree",
    },
    // The branch a reclaim keeps, deleted by a person against the tip they saw.
    Route {
        operation: "delete_branch",
        method: "POST",
        path: "/jobs/:job_id/delete_branch",
    },
    // What a person says went wrong, under the Job it is about, and every
    // report filed, which is not under one — a report outlives the Job it
    // names, so a listing reachable only through a Job would lose exactly the
    // reports that most need reading.
    Route {
        operation: "file_report",
        method: "POST",
        path: "/jobs/:job_id/report",
    },
    Route {
        operation: "list_reports",
        method: "GET",
        path: "/reports",
    },
    // The Job's own log, beside its Drone's transcript and not inside it. The
    // path drops `observe_` and `_job` for the reason `redispatch` drops one:
    // the segment before it already names the Job, and `observe` is taken by
    // the socket next door. **A socket rather than a query** for that one's
    // reason — what it answers is what has already been written followed by
    // what is written next, which no request-response shape carries.
    Route {
        operation: "observe_job_log",
        method: "GET",
        path: "/jobs/:job_id/log",
    },
    // One checkout run's output — `observe_run`'s socket, one owner over.
    Route {
        operation: "observe_checkout_run",
        method: "GET",
        path: "/manifest/runs/:run_id/observe",
    },
    // One server's output, as it prints — `observe_run`'s shape per instance.
    Route {
        operation: "observe_server",
        method: "GET",
        path: "/servers/:server_id/observe",
    },
    // The four narrowings of `/jobs`, each its own route rather than a filter
    // on the list: what counts as waiting on a person is a rule, and a rule
    // stated by every caller is a rule nobody owns.
    Route {
        operation: "list_job_board",
        method: "GET",
        path: "/jobs/board",
    },
    Route {
        operation: "list_reviews",
        method: "GET",
        path: "/jobs/reviews",
    },
    Route {
        operation: "get_activity_feed",
        method: "GET",
        path: "/jobs/activity",
    },
    // Not under `/jobs`, because it is not a list of Jobs: two buckets, each
    // holding a row about one.
    Route {
        operation: "list_alerts",
        method: "GET",
        path: "/alerts",
    },
    // Fleet's own three, beside `/capacity` and for its reason: none of them is
    // a fact about any Job. `/manifests/:manifest_id` is the member read under
    // the collection `list_manifests` already serves.
    Route {
        operation: "get_health",
        method: "GET",
        path: "/health",
    },
    Route {
        operation: "get_usage",
        method: "GET",
        path: "/usage",
    },
    Route {
        operation: "get_manifest",
        method: "GET",
        path: "/manifests/:manifest_id",
    },
    Route {
        operation: "get_manifest_spend",
        method: "GET",
        path: "/manifest/spend",
    },
    // What crossed the stream, counted. **Not `/events`**, which is the socket:
    // this is the read for a caller that cannot hold one, and `?since=` carries
    // a position rather than naming a resource.
    Route {
        operation: "get_events_since",
        method: "GET",
        path: "/events/since",
    },
];
