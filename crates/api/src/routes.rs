//! The route table, by hand, and the state every handler is given.
//!
//! # Hand-written, and that is the accepted cost
//!
//! A typo in a path here is a runtime 404, not a compile error. That trade was
//! made deliberately against carrying a codegen toolchain, and it is why
//! [`SERVED`] exists: every route is declared once as data beside the operation
//! name `crates/ipc/operations/` keys it under, so a test can walk the
//! table and prove each row is actually routed. **A route that exists in the
//! inventory and nowhere in the router is exactly the failure this shape is
//! paying for.**
//!
//! # One listener
//!
//! The WebSocket upgrade is an extractor in this same `Router`. There is no
//! second port: queries and commands answer over HTTP because they are
//! request-response, and only the unsolicited push needs the socket.
//!
//! # The handlers are next door, and the table is one module down
//!
//! `crate::queries` reads, `crate::commands` writes, `crate::sockets` upgrades.
//! [`SERVED`] is [`served`](mod@served)'s, having moved there when this file
//! crossed the 900-line rule; the gate reads the two halves as one text.

use axum::middleware::from_fn_with_state;
use axum::routing::{get, post};
use axum::Router;

use crate::amending::edit_manifest;
use crate::attention::{get_activity_feed, list_alerts, list_job_board, list_reviews};
use crate::commands::{
    add_task, answer_command, answer_judge, answer_question, approve_dispatch, approve_review,
    ask_person_to_approve, change_slot_pool, delete_branch, dismiss_finding, drop_task,
    examine_job, file_finding_issue, file_report, fix_main, forget_job, kill_drone, kill_job,
    merge_pull_request, move_plan, override_verdict, park_job, propose_from_request, propose_job,
    queue_after_finding, raise_cost_cap, raise_turn_cap, reclaim_worktree, redirect_drone,
    redispatch_job, reject_job, request_changes, rerun_checks, rerun_gate, rescue_slot,
    restart_step, restart_task, resume_job, set_when_blocked, set_when_refused, show_again,
    stop_proposal, take_up_remarks,
};
use crate::conversing::{
    answer_helm_call, ask_helm, ask_the_person, get_helm_debug_info, list_helm_calls, observe_helm,
    start_helm_fresh,
};
use crate::daemon::Daemon;
use crate::editing::{get_manifest_file, save_manifest_file};
use crate::fleetwide::{
    get_drone, get_events_since, get_health, get_manifest, get_manifest_spend, get_usage,
    list_drones,
};
use crate::building::{change_fleet_build, get_fleet_build};
use crate::limiting::{get_limits, save_limits};
use crate::preferring::{get_preferences, save_preferences};
use crate::processes::{kill_process, kill_processes};
use crate::queries::{
    explain_command, get_brief, get_call, get_capacity, get_check_output, get_diff, get_evidence,
    get_frame, get_job, get_job_events, get_job_log, get_job_resources, get_manifest_drift,
    get_manifest_reading, get_remarks, list_job_drones, list_jobs, list_manifest_checks,
    list_manifests, list_models, list_reports, list_workflows, list_worktrees, search_files,
};
use crate::rehearsing::{
    get_checkout_run_diff, get_checkout_run_output, get_checkout_run_sheet, get_run_output,
    get_run_sheet, list_checkout_runs, list_runs, observe_checkout_run, observe_run,
    start_checkout_run, start_checkout_verify, start_run, stop_checkout_run, stop_run,
    undo_checkout_run, undo_run,
};
use crate::repositories::{add_repository, clone_repository, get_merge_lines, list_repositories};
use crate::repository_allow::{get_repository_allowed_commands, remove_repository_allowed_command};
use crate::served::Served;
use crate::servers::{list_servers, observe_server, start_server, stop_server};
use crate::sockets::{events, job_log, observe_check_output, observe_job, observe_land_check};
use crate::studios::{
    add_studio_node, ask_scout, capture_studio_note, create_studio, decide_studio_edge,
    defer_on_studio, delete_studio, dispatch_studio_draft, edit_studio_draft, edit_studio_link,
    edit_studio_sketch, get_studio, get_studio_frame, group_studio_nodes, list_studios,
    move_studio_node, propose_studio_edge, read_in_link, remove_studio_nodes, rename_studio,
    settle_contradiction, start_scout, start_studio_run, start_studio_server, stop_scout,
    write_up_studio_node,
};

/// The inventory this router is compared against, row by row.
mod served;

pub use served::{Route, SERVED};

/// The one listener: the HTTP surface, the Drone's endpoint and the agent's
/// door, on one port.
///
/// **The surface is built twice on purpose.** The agent's door makes every
/// tool call against it rather than against the daemon, and a router that held
/// itself is not a value — so it is handed its own copy of the same table.
///
/// **This is the only way to build a served router, and the refusal is on it.**
/// [`surface`] is private and the door's copy is reachable only through the
/// door, so there is no arrangement of this crate's parts that puts a route on
/// the listener with no [`crate::callers::refuse_a_page`] in front of it. The
/// door's own calls carry no `Origin` — they are built here, not forwarded —
/// so a tool call passes exactly as a Bridge request does.
pub fn router<D: Daemon>(served: Served<D>) -> Router {
    let refusing_a_page =
        from_fn_with_state(served.run_id().clone(), crate::callers::refuse_a_page);
    let waiting = served
        .waiting_on()
        .map(|over| (over, served.run_id().clone()));
    let app =
        surface(served.clone()).merge(crate::door::mounted::<D>(served.clone(), surface(served)));
    // Inside the refusal, so a page is turned away at once and not after the wait.
    let app = match waiting {
        Some(waiting) => app.layer(from_fn_with_state(
            waiting,
            crate::reconciling::hold_commands,
        )),
        None => app,
    };
    app.layer(refusing_a_page)
}

/// The HTTP surface and the Drone's endpoint: every route but the door.
fn surface<D: Daemon>(served: Served<D>) -> Router {
    Router::new()
        .route("/jobs", get(list_jobs::<D>).post(propose_job::<D>))
        .route("/jobs/from_request", post(propose_from_request::<D>))
        .route("/proposals/stop", post(stop_proposal::<D>))
        .route("/workflows", get(list_workflows::<D>))
        .route(
            "/workflows/definition",
            get(crate::repositories::get_workflow::<D>),
        )
        .route(
            "/workflows/left_out",
            get(crate::repositories::list_left_out_workflows::<D>),
        )
        .route(
            "/workflows/save",
            post(crate::repositories::save_workflow::<D>),
        )
        .route(
            "/jobs/:job_id/add_job_step",
            post(crate::added_steps::add_job_step::<D>),
        )
        .route(
            "/jobs/:job_id/remove_job_step",
            post(crate::added_steps::remove_job_step::<D>),
        )
        .route(
            "/jobs/:job_id/edit_job_step",
            post(crate::added_steps::edit_job_step::<D>),
        )
        .route(
            "/jobs/:job_id/repair_diff",
            get(crate::added_steps::get_repair_diff::<D>),
        )
        .route("/triggers", get(crate::repositories::list_triggers::<D>))
        .route(
            "/triggers/definition",
            get(crate::repositories::get_trigger::<D>),
        )
        .route(
            "/triggers/save",
            post(crate::repositories::save_trigger::<D>),
        )
        .route(
            "/triggers/remove",
            post(crate::repositories::remove_trigger::<D>),
        )
        .route(
            "/jobs/:job_id/choose_trigger_fix",
            post(crate::repositories::choose_trigger_fix::<D>),
        )
        .route(
            "/jobs/:job_id/rerun_trigger",
            post(crate::repositories::rerun_trigger::<D>),
        )
        .route(
            "/jobs/:job_id/skip_trigger",
            post(crate::repositories::skip_trigger::<D>),
        )
        .route(
            "/needs",
            get(crate::needs::list_needs::<D>).post(crate::needs::act_on_need::<D>),
        )
        .route(
            "/pull_requests/:repository/:number",
            get(crate::pull_requests::get_pull_request::<D>),
        )
        .route(
            "/pull_requests/:repository/:number/ready",
            post(crate::pull_requests::ready_pull_request::<D>),
        )
        .route(
            "/pull_requests/:repository/:number/merge",
            post(crate::pull_requests::merge_pull_request_by_number::<D>),
        )
        .route(
            "/pull_requests/:repository/:number/auto_merge",
            post(crate::pull_requests::enable_auto_merge::<D>),
        )
        .route(
            "/pull_request_reviews/:repository",
            post(crate::pull_requests::review_pull_request::<D>),
        )
        .route("/mods", get(crate::mods::list_mods::<D>))
        .route("/mods/scaffold", post(crate::mods::scaffold_mod::<D>))
        .route("/mods/enable", post(crate::mods::set_mod_enabled::<D>))
        .route("/mods/validate", get(crate::mods::validate_mod::<D>))
        .route("/mods/promote", post(crate::mods::promote_mod::<D>))
        .route("/sessions", get(crate::sessions::list_sessions::<D>))
        .route("/sessions/owner", get(crate::sessions::who_owns::<D>))
        .route(
            "/sessions/rename",
            post(crate::sessions::rename_session::<D>),
        )
        .route(
            "/sessions/window",
            post(crate::sessions::show_window::<D>),
        )
        .route(
            "/sessions/claim_pull_request",
            post(crate::sessions::claim_pull_request::<D>),
        )
        .route(
            "/sessions/start",
            post(crate::hosted_sessions::start_session::<D>),
        )
        .route(
            "/sessions/message",
            post(crate::hosted_sessions::send_session_message::<D>),
        )
        .route(
            "/sessions/ask/answer",
            post(crate::hosted_sessions::answer_session_ask::<D>),
        )
        .route(
            "/sessions/tune",
            post(crate::hosted_sessions::tune_session::<D>),
        )
        .route(
            "/sessions/close",
            post(crate::hosted_sessions::close_session::<D>),
        )
        .route(
            "/sessions/one",
            get(crate::hosted_sessions::get_session::<D>),
        )
        .route(
            "/sessions/subagent",
            get(crate::hosted_sessions::get_session_subagent::<D>),
        )
        .route(
            "/sessions/file",
            get(crate::hosted_sessions::get_session_file::<D>),
        )
        .route(
            "/sessions/gate",
            post(crate::hosted_sessions::gate_session_call::<D>),
        )
        .route(
            "/sessions/ask/terminal",
            post(crate::hosted_sessions::ask_from_terminal::<D>),
        )
        .route(
            "/sessions/held",
            post(crate::hosted_sessions::take_held_messages::<D>),
        )
        .route(
            "/sessions/report",
            post(crate::sessions::report_session::<D>),
        )
        .route("/manifests", get(list_manifests::<D>))
        .route("/repositories", get(list_repositories::<D>))
        .route("/repositories/add", post(add_repository::<D>))
        .route("/repositories/clone", post(clone_repository::<D>))
        .route("/merge_lines", get(get_merge_lines::<D>))
        .route("/merge_lines/checks/observe", get(observe_land_check::<D>))
        .route("/merge_lines/fix", post(fix_main::<D>))
        .route("/models", get(list_models::<D>))
        .route("/capacity", get(get_capacity::<D>))
        .route("/fleet/build", get(get_fleet_build::<D>))
        .route("/fleet/build/change", post(change_fleet_build::<D>))
        .route("/limits", get(get_limits::<D>))
        .route("/limits/save", post(save_limits::<D>))
        .route("/preferences", get(get_preferences::<D>))
        .route("/preferences/save", post(save_preferences::<D>))
        .route("/health", get(get_health::<D>))
        .route("/usage", get(get_usage::<D>))
        .route("/alerts", get(list_alerts::<D>))
        .route("/drones", get(list_drones::<D>))
        .route("/drones/:drone_id", get(get_drone::<D>))
        .route("/manifests/:manifest_id", get(get_manifest::<D>))
        .route("/events/since", get(get_events_since::<D>))
        .route("/jobs/board", get(list_job_board::<D>))
        .route("/jobs/reviews", get(list_reviews::<D>))
        .route("/jobs/activity", get(get_activity_feed::<D>))
        .route("/manifest/reading", get(get_manifest_reading::<D>))
        .route("/manifest/drift", get(get_manifest_drift::<D>))
        .route("/manifest/spend", get(get_manifest_spend::<D>))
        .route(
            "/repository/scan",
            get(crate::queries::get_repository_scan::<D>),
        )
        .route(
            "/repository/proposals",
            get(crate::manifest_proposals::get_manifest_proposals::<D>),
        )
        .route(
            "/repository/edit_proposal",
            post(crate::manifest_proposals::edit_manifest_proposal::<D>),
        )
        .route(
            "/repository/write_proposal",
            post(crate::manifest_proposals::write_manifest_proposal::<D>),
        )
        .route("/manifest/file", get(get_manifest_file::<D>))
        .route("/manifest/save_file", post(save_manifest_file::<D>))
        .route("/manifest/edit", post(edit_manifest::<D>))
        .route("/manifest/files", get(search_files::<D>))
        .route(
            "/manifest/branches",
            get(crate::queries::list_branches::<D>),
        )
        .route(
            "/manifest/allowed_commands",
            get(get_repository_allowed_commands::<D>),
        )
        .route(
            "/manifest/allowed_commands/remove",
            post(remove_repository_allowed_command::<D>),
        )
        .route("/kit/inventory", get(crate::kit::get_kit_inventory::<D>))
        .route("/kit/servers", get(crate::kit::get_kit_servers::<D>))
        .route("/kit/servers/add", post(crate::kit::add_kit_server::<D>))
        .route(
            "/kit/servers/forget",
            post(crate::kit::forget_kit_server::<D>),
        )
        .route(
            "/kit/servers/reach",
            post(crate::kit::set_kit_server_reach::<D>),
        )
        .route(
            "/kit/servers/manifest_reach",
            post(crate::kit::set_manifest_server_reach::<D>),
        )
        .route(
            "/kit/allowed_commands/remove",
            post(crate::kit::remove_kit_allowed_command::<D>),
        )
        .route("/jobs/:job_id", get(get_job::<D>))
        .route("/jobs/:job_id/events", get(get_job_events::<D>))
        .route(
            "/jobs/:job_id/retro",
            get(crate::retros::get_job_retro::<D>),
        )
        .route(
            "/sessions/:session_id/retro",
            get(crate::retros::get_session_retro::<D>).post(crate::retros::write_session_retro::<D>),
        )
        .route("/lessons", get(crate::retros::list_lessons::<D>))
        .route(
            "/lessons/:lesson_id/agree",
            post(crate::retros::agree_lesson::<D>),
        )
        .route(
            "/lessons/:lesson_id/disagree",
            post(crate::retros::disagree_lesson::<D>),
        )
        .route("/jobs/:job_id/evidence", get(get_evidence::<D>))
        .route("/jobs/:job_id/diff", get(get_diff::<D>))
        .route("/jobs/:job_id/remarks", get(get_remarks::<D>))
        .route("/jobs/:job_id/resources", get(get_job_resources::<D>))
        .route("/jobs/:job_id/drones", get(list_job_drones::<D>))
        .route("/jobs/:job_id/examine", post(examine_job::<D>))
        .route(
            "/jobs/:job_id/ask_person_to_approve",
            post(ask_person_to_approve::<D>),
        )
        .route("/jobs/:job_id/calls/:call_id", get(get_call::<D>))
        .route(
            "/jobs/:job_id/calls/:call_id/explain",
            get(explain_command::<D>),
        )
        .route(
            "/jobs/:job_id/checks/:kept/output",
            get(get_check_output::<D>),
        )
        .route("/jobs/:job_id/frames/:run/:name", get(get_frame::<D>))
        .route("/jobs/:job_id/briefs/:name", get(get_brief::<D>))
        .route("/jobs/:job_id/approve_review", post(approve_review::<D>))
        .route("/jobs/:job_id/merge", post(merge_pull_request::<D>))
        .route(
            "/jobs/:job_id/auto_merge",
            post(crate::commands::enable_job_auto_merge::<D>),
        )
        .route(
            "/jobs/:job_id/rerun_failed_checks",
            post(crate::commands::rerun_failed_checks::<D>),
        )
        .route(
            "/jobs/:job_id/investigate_failed_checks",
            post(crate::commands::investigate_failed_checks::<D>),
        )
        .route("/jobs/:job_id/request_changes", post(request_changes::<D>))
        .route("/jobs/:job_id/reject", post(reject_job::<D>))
        .route("/jobs/:job_id/take_up_remarks", post(take_up_remarks::<D>))
        .route("/jobs/:job_id/dismiss_finding", post(dismiss_finding::<D>))
        .route(
            "/jobs/:job_id/walk_notes",
            post(crate::walking::capture_walk_note::<D>),
        )
        .route(
            "/jobs/:job_id/walk_notes/remove",
            post(crate::walking::remove_walk_note::<D>),
        )
        .route(
            "/jobs/:job_id/queue_after_finding",
            post(queue_after_finding::<D>),
        )
        .route(
            "/jobs/:job_id/file_finding_issue",
            post(file_finding_issue::<D>),
        )
        .route(
            "/jobs/:job_id/override_verdict",
            post(override_verdict::<D>),
        )
        .route("/jobs/:job_id/rerun_gate", post(rerun_gate::<D>))
        .route("/jobs/:job_id/rerun_checks", post(rerun_checks::<D>))
        .route("/jobs/:job_id/show_again", post(show_again::<D>))
        .route("/jobs/:job_id/run_sheet", get(get_run_sheet::<D>))
        .route("/jobs/:job_id/runs", get(list_runs::<D>))
        .route(
            "/jobs/:job_id/runs/:run_id/output",
            get(get_run_output::<D>),
        )
        .route("/jobs/:job_id/runs/:run_id/observe", get(observe_run::<D>))
        .route("/jobs/:job_id/start_run", post(start_run::<D>))
        .route("/jobs/:job_id/stop_run", post(stop_run::<D>))
        .route("/jobs/:job_id/undo_run", post(undo_run::<D>))
        .route("/manifest/run_sheet", get(get_checkout_run_sheet::<D>))
        .route("/manifest/runs", get(list_checkout_runs::<D>))
        .route("/manifest/checks", get(list_manifest_checks::<D>))
        .route(
            "/manifest/runs/:run_id/output",
            get(get_checkout_run_output::<D>),
        )
        .route(
            "/manifest/runs/:run_id/diff",
            get(get_checkout_run_diff::<D>),
        )
        .route(
            "/manifest/runs/:run_id/observe",
            get(observe_checkout_run::<D>),
        )
        .route("/manifest/start_run", post(start_checkout_run::<D>))
        .route("/manifest/stop_run", post(stop_checkout_run::<D>))
        .route("/manifest/undo_run", post(undo_checkout_run::<D>))
        .route("/manifest/start_verify", post(start_checkout_verify::<D>))
        .route("/servers", get(list_servers::<D>))
        .route("/servers/start", post(start_server::<D>))
        .route("/servers/stop", post(stop_server::<D>))
        .route("/servers/:server_id/observe", get(observe_server::<D>))
        .route(
            "/jobs/:job_id/approve_dispatch",
            post(approve_dispatch::<D>),
        )
        .route("/jobs/:job_id/edit", post(crate::commands::edit_job::<D>))
        .route(
            "/jobs/:job_id/approve_wave",
            post(crate::commands::approve_wave::<D>),
        )
        .route("/jobs/:job_id/raise_cost_cap", post(raise_cost_cap::<D>))
        .route("/jobs/:job_id/raise_turn_cap", post(raise_turn_cap::<D>))
        .route("/jobs/:job_id/kill_drone", post(kill_drone::<D>))
        .route(
            "/jobs/:job_id/take_over",
            post(crate::piloting::take_over::<D>),
        )
        .route(
            "/jobs/:job_id/handoff",
            get(crate::piloting::get_handoff::<D>),
        )
        .route(
            "/jobs/:job_id/submit_for_verification",
            post(crate::piloting::submit_for_verification::<D>),
        )
        .route(
            "/jobs/:job_id/attest_complete",
            post(crate::piloting::attest_complete::<D>),
        )
        .route(
            "/jobs/:job_id/close_as_superseded",
            post(crate::piloting::close_as_superseded::<D>),
        )
        .route("/jobs/:job_id/kill_job", post(kill_job::<D>))
        .route("/jobs/:job_id/park_job", post(park_job::<D>))
        .route("/jobs/:job_id/resume_job", post(resume_job::<D>))
        .route("/jobs/:job_id/processes/:pid/kill", post(kill_process::<D>))
        .route("/jobs/:job_id/processes/kill", post(kill_processes::<D>))
        .route(
            "/jobs/:job_id/drones/:drone_id/kill",
            post(crate::one_drone::kill_one_drone::<D>),
        )
        .route(
            "/jobs/:job_id/drones/:drone_id/redirect",
            post(crate::one_drone::redirect_one_drone::<D>),
        )
        .route("/jobs/:job_id/forget_job", post(forget_job::<D>))
        .route(
            "/jobs/:job_id/reclaim_worktree",
            post(reclaim_worktree::<D>),
        )
        .route("/jobs/:job_id/delete_branch", post(delete_branch::<D>))
        .route("/jobs/:job_id/redispatch", post(redispatch_job::<D>))
        .route("/jobs/:job_id/add_task", post(add_task::<D>))
        .route("/jobs/:job_id/drop_task", post(drop_task::<D>))
        .route(
            "/jobs/:job_id/tasks/:task_id/restart",
            post(restart_task::<D>),
        )
        .route("/jobs/:job_id/plan/move", post(move_plan::<D>))
        .route(
            "/jobs/:job_id/tasks/:task_id/edit",
            post(crate::commands::edit_task::<D>),
        )
        .route("/jobs/:job_id/redirect", post(redirect_drone::<D>))
        .route("/jobs/:job_id/restart_step", post(restart_step::<D>))
        .route("/jobs/:job_id/answer_question", post(answer_question::<D>))
        .route("/jobs/:job_id/answer_command", post(answer_command::<D>))
        .route(
            "/jobs/:job_id/set_when_blocked",
            post(set_when_blocked::<D>),
        )
        .route("/jobs/:job_id/answer_judge", post(answer_judge::<D>))
        .route(
            "/jobs/:job_id/set_when_refused",
            post(set_when_refused::<D>),
        )
        .route(
            "/jobs/:job_id/set_model",
            post(crate::commands::set_model::<D>),
        )
        .route(
            "/jobs/:job_id/set_review_model",
            post(crate::commands::set_review_model::<D>),
        )
        .route(
            "/jobs/:job_id/set_tiers",
            post(crate::commands::set_tiers::<D>),
        )
        .route(
            "/jobs/:job_id/to_proposer",
            post(crate::commands::to_proposer::<D>),
        )
        .route(
            "/jobs/:job_id/set_landing_target",
            post(crate::commands::set_landing_target::<D>),
        )
        .route(
            "/jobs/:job_id/remove_allowed_command",
            post(crate::commands::remove_allowed_command::<D>),
        )
        .route("/jobs/:job_id/report", post(file_report::<D>))
        .route("/reports", get(list_reports::<D>))
        .route("/worktrees", get(list_worktrees::<D>))
        .route("/worktrees/slots", post(change_slot_pool::<D>))
        .route("/worktrees/slots/rescue", post(rescue_slot::<D>))
        .route("/jobs/:job_id/observe", get(observe_job::<D>))
        .route("/jobs/:job_id/log", get(job_log::<D>))
        .route("/jobs/:job_id/log/read", get(get_job_log::<D>))
        .route(
            "/jobs/:job_id/checks/:kept/observe",
            get(observe_check_output::<D>),
        )
        .route("/helm/observe", get(observe_helm::<D>))
        .route("/helm/ask", post(ask_helm::<D>))
        .route("/helm/debug", get(get_helm_debug_info::<D>))
        .route("/helm/start_fresh", post(start_helm_fresh::<D>))
        .route("/helm/permission", post(ask_the_person::<D>))
        .route("/helm/calls", get(list_helm_calls::<D>))
        .route("/helm/calls/answer", post(answer_helm_call::<D>))
        .route("/studios", get(list_studios::<D>))
        .route("/studios/create", post(create_studio::<D>))
        .route("/studios/:studio_id", get(get_studio::<D>))
        .route(
            "/studios/:studio_id/frames/:node_id",
            get(get_studio_frame::<D>),
        )
        .route("/studios/:studio_id/rename", post(rename_studio::<D>))
        .route("/studios/:studio_id/delete", post(delete_studio::<D>))
        .route("/studios/:studio_id/add_node", post(add_studio_node::<D>))
        .route(
            "/studios/:studio_id/capture_note",
            post(capture_studio_note::<D>),
        )
        .route("/studios/:studio_id/move_node", post(move_studio_node::<D>))
        .route(
            "/studios/:studio_id/remove_nodes",
            post(remove_studio_nodes::<D>),
        )
        .route(
            "/studios/:studio_id/propose_edge",
            post(propose_studio_edge::<D>),
        )
        .route(
            "/studios/:studio_id/decide_edge",
            post(decide_studio_edge::<D>),
        )
        .route(
            "/studios/:studio_id/group_nodes",
            post(group_studio_nodes::<D>),
        )
        .route("/studios/:studio_id/defer", post(defer_on_studio::<D>))
        .route(
            "/studios/:studio_id/write_up",
            post(write_up_studio_node::<D>),
        )
        .route(
            "/studios/:studio_id/edit_draft",
            post(edit_studio_draft::<D>),
        )
        .route("/studios/:studio_id/edit_link", post(edit_studio_link::<D>))
        .route(
            "/studios/:studio_id/edit_sketch",
            post(edit_studio_sketch::<D>),
        )
        .route(
            "/studios/:studio_id/settle",
            post(settle_contradiction::<D>),
        )
        .route(
            "/studios/:studio_id/dispatch_draft",
            post(dispatch_studio_draft::<D>),
        )
        .route("/studios/:studio_id/ask_scout", post(ask_scout::<D>))
        .route("/studios/:studio_id/start_scout", post(start_scout::<D>))
        .route("/studios/:studio_id/stop_scout", post(stop_scout::<D>))
        .route("/studios/:studio_id/read_in", post(read_in_link::<D>))
        .route("/studios/:studio_id/start_run", post(start_studio_run::<D>))
        .route(
            "/studios/:studio_id/start_server",
            post(start_studio_server::<D>),
        )
        .route("/events", get(events::<D>))
        // The Evidence endpoint, on the same listener and deliberately not in
        // `SERVED`: it is the Fleet/Drone seam rather than the Fleet/Bridge
        // one, and the inventory this table is checked against is Bridge's.
        .merge(crate::mcp::mounted::<D>())
        .with_state(served)
        // Over every route on both copies of the surface, so a call the door
        // makes is named as the door's and not as a request that named nobody.
        .layer(axum::middleware::from_fn(crate::acting::named_the_door))
}
