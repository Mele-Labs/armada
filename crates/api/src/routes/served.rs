//! The inventory: every operation M1 serves, declared once as data.
//!
//! **Split out of `crate::routes` when that file crossed the 900-line rule.**
//! The router and the table it is compared against are two different things,
//! and the file named for the router now holds the router. The gate rule reads
//! both halves as one text — `xtask/src/rules_protocol.rs`.
//!
//! **Split again at the 1200-line rule**, into tables joined at compile time: Studios' in
//! [`studios`](mod@studios), event kinds in [`events`](mod@events), a pilot's in
//! [`piloting`](mod@piloting), workflows' and Triggers' in [`authoring`](mod@authoring), settings in
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
mod piloting;
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
            + piloting::ROUTES.len()
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
    piloting::ROUTES,
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

/// The thirteen tables as one, in order. A count that disagrees with `N` fails the build.
const fn joined<const N: usize>(tables: [&[Route]; 13]) -> [Route; N] {
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
    // Fleet's own capacity, not a Job's. It is `/capacity` and not
    // `/jobs/capacity` because nothing about it is scoped to a Job — the whole
    // point is the answer no per-Job field could give.
    Route {
        operation: "get_capacity",
        method: "GET",
        path: "/capacity",
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
    // The two halves of the work product, on two routes and not one. Evidence
    // is a handful of sentences per step and the patch is however large the
    // work is, so a surface wanting only the claims does not fetch the bytes.
    // Neither is on `get_job`, which is read on every open to draw a summary.
    Route {
        operation: "get_evidence",
        method: "GET",
        path: "/jobs/:job_id/evidence",
    },
    Route {
        operation: "get_diff",
        method: "GET",
        path: "/jobs/:job_id/diff",
    },
    // What people wrote on the pull request, beside the two halves of the work
    // product and for their reason: it is read by the surface where somebody is
    // deciding and by nothing else. **The one query on this table that talks to
    // a forge**, which is why it is not a field on `get_job`.
    Route {
        operation: "get_remarks",
        method: "GET",
        path: "/jobs/:job_id/remarks",
    },
    // The machine reading, and the act above it. Not fields on `get_job`,
    // which is re-read on every event naming the Job: these walk a process
    // table and a directory. The act is a POST because it does work and leaves
    // a line in the Job's own log, which is where its answer lives.
    Route {
        operation: "get_job_resources",
        method: "GET",
        path: "/jobs/:job_id/resources",
    },
    Route {
        operation: "list_job_drones",
        method: "GET",
        path: "/jobs/:job_id/drones",
    },
    Route {
        operation: "examine_job",
        method: "POST",
        path: "/jobs/:job_id/examine",
    },
    // `#1041`. Helm's own approval-card ask. Writes nothing; see the operation.
    Route {
        operation: "ask_person_to_approve",
        method: "POST",
        path: "/jobs/:job_id/ask_person_to_approve",
    },
    // What one call would do, read by a model for the person deciding whether
    // to allow it. Under the call: the segment before `explain` is the call id,
    // the same id `answer_command` takes. A query,
    // because it decides nothing — the offers are unchanged and the Drone stays
    // held whatever it says.
    Route {
        operation: "explain_command",
        method: "GET",
        path: "/jobs/:job_id/calls/:call_id/explain",
    },
    // One Check's output, read into the app rather than handed to the operating
    // system. `:kept` is a row's own file name — the last component of
    // `output_path` — and never a path the caller composed: Fleet resolves it
    // against the rows it holds for the Job before it opens anything.
    Route {
        operation: "get_check_output",
        method: "GET",
        path: "/jobs/:job_id/checks/:kept/output",
    },
    // One frame a step's harness produced, answered as the file. `:kept` is the
    // run directory and the file name joined — composed by Fleet, handed to the
    // client on the row, and resolved against the rows this Job holds before
    // anything is opened. Two components rather than one because a frame's name
    // is the harness's own and two steps may both have written `home.png`.
    Route {
        operation: "get_frame",
        method: "GET",
        path: "/jobs/:job_id/frames/:run/:name",
    },
    // One brief a Judge or a gaming check was asked. `:name` is the last part
    // of a `brief_path`, resolved inside this Job's briefs directory.
    Route {
        operation: "get_brief",
        method: "GET",
        path: "/jobs/:job_id/briefs/:name",
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
        operation: "review_pull_request",
        method: "POST",
        path: "/pull_request_reviews/:repository",
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
        operation: "propose_job",
        method: "POST",
        path: "/jobs",
    },
    // A second POST under `/jobs`, and a segment rather than a mode flag on the
    // first: what a caller sends is a different shape, and one route taking
    // either would be the two paths sharing a body that means two things.
    Route {
        operation: "propose_from_request",
        method: "POST",
        path: "/jobs/from_request",
    },
    // Not under `/jobs`, because a proposal is not one yet — that is the whole
    // of what it is deciding. The id is in the body rather than the path for
    // the reason `ipc::StopProposal` states: a proposal has exactly one
    // operation and is not a resource.
    Route {
        operation: "stop_proposal",
        method: "POST",
        path: "/proposals/stop",
    },
    Route {
        operation: "approve_dispatch",
        method: "POST",
        path: "/jobs/:job_id/approve_dispatch",
    },
    // A proposal's words, saved without releasing it: the route of #1699.
    Route {
        operation: "edit_job",
        method: "POST",
        path: "/jobs/:job_id/edit",
    },
    // The three answers at a human gate, and three routes rather than one with
    // a decision in the body: each does something different to the Job, and one
    // route taking any of them would be a body that means three things. The
    // reject path drops `_job` for the reason `redispatch` does.
    Route {
        operation: "approve_review",
        method: "POST",
        path: "/jobs/:job_id/approve_review",
    },
    Route {
        operation: "request_changes",
        method: "POST",
        path: "/jobs/:job_id/request_changes",
    },
    Route {
        operation: "reject_job",
        method: "POST",
        path: "/jobs/:job_id/reject",
    },
    // A fifth thing to do at the same gate, and its own route for the reason
    // the three above have three: what it does to the Job is `request_changes`,
    // and what it takes is a set of handles off a forge rather than a person's
    // words. One route taking either would be a body that means two things.
    Route {
        operation: "take_up_remarks",
        method: "POST",
        path: "/jobs/:job_id/take_up_remarks",
    },
    // A person ruling on the review, not on the Job: it moves nothing, so it is not a sixth answer. #907.
    Route {
        operation: "dismiss_finding",
        method: "POST",
        path: "/jobs/:job_id/dismiss_finding",
    },
    // What a person pointed at while walking the Job's mock, and taking one
    // back. Two routes for the reason `dismiss_finding` is not a sixth answer:
    // neither moves the Job. Since 23.18.
    Route {
        operation: "capture_walk_note",
        method: "POST",
        path: "/jobs/:job_id/walk_notes",
    },
    Route {
        operation: "remove_walk_note",
        method: "POST",
        path: "/jobs/:job_id/walk_notes/remove",
    },
    // What a For context finding becomes: a Job queued behind this one, or an issue. #906.
    Route {
        operation: "queue_after_finding",
        method: "POST",
        path: "/jobs/:job_id/queue_after_finding",
    },
    Route {
        operation: "file_finding_issue",
        method: "POST",
        path: "/jobs/:job_id/file_finding_issue",
    },
    // The fourth answer, and the only route on this table that writes into a
    // repository Fleet did not make. Its own route for the reason the three
    // above have three: it does something different to the world, and one route
    // taking a decision in the body would be a body that means four things.
    Route {
        operation: "merge_pull_request",
        method: "POST",
        path: "/jobs/:job_id/merge",
    },
    Route {
        operation: "enable_job_auto_merge",
        method: "POST",
        path: "/jobs/:job_id/auto_merge",
    },
    Route {
        operation: "rerun_failed_checks",
        method: "POST",
        path: "/jobs/:job_id/rerun_failed_checks",
    },
    Route {
        operation: "investigate_failed_checks",
        method: "POST",
        path: "/jobs/:job_id/investigate_failed_checks",
    },
    // The answer at a gate that refused, which is a different place from the
    // three above: those answer `awaiting_review` and this answers `escalated`.
    // Its own route rather than a flag on `approve_review` — one route taking
    // either would let a refusal be taken with the act built for work nothing
    // objected to, which is exactly the confusion the record has to keep apart.
    Route {
        operation: "override_verdict",
        method: "POST",
        path: "/jobs/:job_id/override_verdict",
    },
    // The answer at a gate that could not rule, which is a different place
    // again: `override_verdict` lifts a decision and this asks for one. Its own
    // route rather than a flag on that one — the two triggers do not overlap,
    // and one route taking either would let a step nothing weighed be advanced
    // by the act built for disagreeing with a machine that did.
    Route {
        operation: "rerun_gate",
        method: "POST",
        path: "/jobs/:job_id/rerun_gate",
    },
    // A failed Check's re-run, beneath `awaiting_repair`. Its own route rather
    // than `rerun_gate`'s: that one answers a gate that could not rule, and
    // this one a Check that did.
    Route {
        operation: "rerun_checks",
        method: "POST",
        path: "/jobs/:job_id/rerun_checks",
    },
    // A person asking a Job to show its work. A POST because it runs a
    // repository's harness and keeps what it captured, and its own route
    // because it moves nothing on the Job — what comes back is a set of frames
    // rather than a row.
    Route {
        operation: "show_again",
        method: "POST",
        path: "/jobs/:job_id/show_again",
    },
    // The one act on this table that changes what a Job may spend, and its own
    // route because nothing else on the Job is a number a person sets. It is
    // not a field on some general update: there is no general update, and the
    // reason there is not is that every other row here is a named act with its
    // own refusals.
    Route {
        operation: "raise_cost_cap",
        method: "POST",
        path: "/jobs/:job_id/raise_cost_cap",
    },
    // The other ceiling, on a route of its own for the reason the row above
    // is not a field on some general update: the two are refused separately
    // and a person clears one of them at a time.
    Route {
        operation: "raise_turn_cap",
        method: "POST",
        path: "/jobs/:job_id/raise_turn_cap",
    },
    Route {
        operation: "kill_drone",
        method: "POST",
        path: "/jobs/:job_id/kill_drone",
    },
    Route {
        operation: "kill_job",
        method: "POST",
        path: "/jobs/:job_id/kill_job",
    },
    Route {
        operation: "kill_process",
        method: "POST",
        path: "/jobs/:job_id/processes/:pid/kill",
    },
    Route {
        operation: "kill_processes",
        method: "POST",
        path: "/jobs/:job_id/processes/kill",
    },
    // One Drone of a Job, by its id, where a Job runs several at once. #1666.
    Route {
        operation: "kill_one_drone",
        method: "POST",
        path: "/jobs/:job_id/drones/:drone_id/kill",
    },
    Route {
        operation: "redirect_one_drone",
        method: "POST",
        path: "/jobs/:job_id/drones/:drone_id/redirect",
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
    Route {
        operation: "redispatch_job",
        method: "POST",
        path: "/jobs/:job_id/redispatch",
    },
    // A person's two edits to the Job's plan, `#897`. Two routes rather than
    // one with a kind in the body: add and drop take different fields, and
    // one route taking either would be a body that means two things.
    Route {
        operation: "add_task",
        method: "POST",
        path: "/jobs/:job_id/add_task",
    },
    Route {
        operation: "drop_task",
        method: "POST",
        path: "/jobs/:job_id/drop_task",
    },
    // A person's two acts on a plan's groups, spike 022 slice 2: restart one
    // failed task, and move a task or a group by `after`.
    Route {
        operation: "restart_task",
        method: "POST",
        path: "/jobs/:job_id/tasks/:task_id/restart",
    },
    Route {
        operation: "move_plan",
        method: "POST",
        path: "/jobs/:job_id/plan/move",
    },
    // A person changes one task: spike 022 slice 3, `#1657`.
    Route {
        operation: "edit_task",
        method: "POST",
        path: "/jobs/:job_id/tasks/:task_id/edit",
    },
    // The two acts that resume a step without redispatching. Two routes and
    // not one with a mode: which applies is decided by whether the Job holds a
    // Drone, and a caller that asked for the wrong one is told which is right
    // rather than silently given it.
    Route {
        operation: "redirect_drone",
        method: "POST",
        path: "/jobs/:job_id/redirect",
    },
    // A pause is not a kill and not a hold on a Drone: the Job's work is
    // parked and its slot freed, and the Job itself is kept.
    Route {
        operation: "park_job",
        method: "POST",
        path: "/jobs/:job_id/park_job",
    },
    Route {
        operation: "resume_job",
        method: "POST",
        path: "/jobs/:job_id/resume_job",
    },
    Route {
        operation: "restart_step",
        method: "POST",
        path: "/jobs/:job_id/restart_step",
    },
    // Its own route and not a shape `redirect` also takes: one carries a
    // person's words and this a label the Drone offered, and one route taking
    // either would make the closed set optional.
    Route {
        operation: "answer_question",
        method: "POST",
        path: "/jobs/:job_id/answer_question",
    },
    // Not a shape `answer_question` also takes: that one is a label a Drone
    // offered, and this one of three answers Fleet offers about a command.
    Route {
        operation: "answer_command",
        method: "POST",
        path: "/jobs/:job_id/answer_command",
    },
    Route {
        operation: "set_when_blocked",
        method: "POST",
        path: "/jobs/:job_id/set_when_blocked",
    },
    Route {
        operation: "answer_judge",
        method: "POST",
        path: "/jobs/:job_id/answer_judge",
    },
    Route {
        operation: "set_when_refused",
        method: "POST",
        path: "/jobs/:job_id/set_when_refused",
    },
    Route {
        operation: "set_model",
        method: "POST",
        path: "/jobs/:job_id/set_model",
    },
    Route {
        operation: "set_review_model",
        method: "POST",
        path: "/jobs/:job_id/set_review_model",
    },
    Route {
        operation: "set_tiers",
        method: "POST",
        path: "/jobs/:job_id/set_tiers",
    },
    Route {
        operation: "to_proposer",
        method: "POST",
        path: "/jobs/:job_id/to_proposer",
    },
    Route {
        operation: "set_landing_target",
        method: "POST",
        path: "/jobs/:job_id/set_landing_target",
    },
    Route {
        operation: "remove_allowed_command",
        method: "POST",
        path: "/jobs/:job_id/remove_allowed_command",
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
    // A socket rather than a body: it opens with what has already happened and
    // then continues, which no request-response shape carries. The path drops
    // `_job` for the reason `redispatch` does — the segment before it already
    // names the Job.
    Route {
        operation: "observe_job",
        method: "GET",
        path: "/jobs/:job_id/observe",
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
    // One running Check's log, as it is written. `get_check_output`'s `:kept`
    // one route over, resolved against the Checks the Job's gate is running
    // rather than against the rows; a socket for `observe_job_log`'s reason.
    Route {
        operation: "observe_check_output",
        method: "GET",
        path: "/jobs/:job_id/checks/:kept/observe",
    },
    // One person's run, as it prints: a socket per run, for `observe_job`'s
    // reason — output on `/events` would evict the state the Board is drawn from.
    Route {
        operation: "observe_run",
        method: "GET",
        path: "/jobs/:job_id/runs/:run_id/observe",
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
    // The roster and one slot in it. `/drones/:drone_id` is the collection and
    // the member, naming no act, which is `list_jobs` and `get_job`'s shape one
    // record over — and not under `/jobs`, because a Drone outlives the step
    // pointer that names it.
    Route {
        operation: "list_drones",
        method: "GET",
        path: "/drones",
    },
    Route {
        operation: "get_drone",
        method: "GET",
        path: "/drones/:drone_id",
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
