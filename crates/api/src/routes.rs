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
use crate::commands::{catch_up_checkout, change_slot_pool, checkout_to_pull_request, rescue_slot};
use crate::conversing::{
    answer_helm_call, ask_helm, ask_the_person, get_helm_debug_info, get_helm_session,
    list_helm_calls, read_thought, record_thought_sent, start_helm_fresh, tell_session,
};
use crate::daemon::Daemon;
use crate::editing::{get_manifest_file, save_manifest_file};
use crate::fleetwide::{get_events_since, get_health, get_manifest};
use crate::building::{change_fleet_build, get_fleet_build};
use crate::limiting::{get_limits, save_limits};
use crate::preferring::{get_preferences, save_preferences};
use crate::queries::{
    get_manifest_drift, get_manifest_reading, list_manifests, list_models, list_workflows,
    list_worktrees, search_files,
};
use crate::rehearsing::{
    get_checkout_run_diff, get_checkout_run_output, get_checkout_run_sheet, list_checkout_runs,
    observe_checkout_run, start_checkout_run, start_checkout_verify, stop_checkout_run,
    undo_checkout_run,
};
use crate::repositories::{
    add_repository, checkout_diff, clone_repository, create_project, get_merge_lines, list_repositories, read_request_repository,
};
use crate::repository_allow::{get_repository_allowed_commands, remove_repository_allowed_command};
use crate::served::Served;
use crate::bay_history::get_slot_history;
use crate::timeline::get_fleet_timeline;
use crate::servers::{list_servers, observe_server, start_server, stop_server};
use crate::sockets::{events, observe_land_check};
use crate::studios::{
    add_studio_node, ask_scout, capture_studio_note, create_studio, decide_studio_edge,
    defer_on_studio, delete_studio, edit_studio_draft, edit_studio_link,
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
            "/workflows/steps",
            get(crate::repositories::get_workflow_steps::<D>),
        )
        .route(
            "/workflows/save",
            post(crate::repositories::save_workflow::<D>),
        )
        .route(
            "/workflows/delete",
            post(crate::repositories::delete_workflow_copy::<D>),
        )
        .route(
            "/workflows/reset",
            post(crate::repositories::reset_workflow_copy::<D>),
        )
        .route(
            "/workflows/not_a_drift",
            post(crate::repositories::mark_not_a_drift::<D>),
        )
        .route(
            "/workflows/changes",
            get(crate::repositories::list_workflow_changes::<D>),
        )
        .route(
            "/workflows/changes/ask",
            post(crate::repositories::ask_workflow_change::<D>),
        )
        .route(
            "/workflows/changes/propose_new",
            post(crate::repositories::propose_new_workflow::<D>),
        )
        .route(
            "/workflows/changes/suggest",
            post(crate::repositories::suggest_workflow_change::<D>),
        )
        .route(
            "/workflows/changes/approve",
            post(crate::repositories::approve_workflow_change::<D>),
        )
        .route(
            "/workflows/changes/dismiss",
            post(crate::repositories::dismiss_workflow_change::<D>),
        )
        .route(
            "/workflows/changes/send_back",
            post(crate::repositories::send_back_workflow_change::<D>),
        )
        .route(
            "/workflows/propose",
            post(crate::repositories::propose_workflow_to_armada::<D>),
        )
        .route("/clashes", get(crate::clashing::list_write_clashes::<D>))
        .route("/clashes/file", get(crate::clashing::read_clash_file::<D>))
        .route("/sessions/file_change", get(crate::clashing::get_session_file_change::<D>))
        .route("/repository_areas", get(crate::clashing::list_repository_areas::<D>))
        .route("/slot_touches", get(crate::clashing::list_slot_touches::<D>))
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
            "/pull_requests/:repository/:number/refresh",
            post(crate::pull_requests::refresh_pull_request::<D>),
        )
        .merge(crate::issues::mounted::<D>())
        .route(
            "/pull_request_details/:repository/:number",
            get(crate::pull_request_panel::get_pull_request_detail::<D>),
        )
        .route(
            "/pull_request_files/:repository/:number",
            get(crate::pull_request_panel::get_pull_request_file::<D>),
        )
        .route(
            "/pull_request_patches/:repository/:number",
            get(crate::pull_request_panel::get_pull_request_patches::<D>),
        )
        .route(
            "/pull_request_writes/:repository",
            post(crate::pull_request_panel::write_pull_request::<D>),
        )
        .route("/review_lists", get(crate::pull_request_panel::get_review_lists::<D>))
        .route(
            "/review_threads/:repository",
            post(crate::pull_request_panel::resolve_review_thread::<D>),
        )
        .route(
            "/review_tools/:repository",
            get(crate::pull_request_panel::list_review_tools::<D>)
                .post(crate::pull_request_panel::save_review_tool::<D>),
        )
        .route(
            "/review_runs/:repository",
            post(crate::pull_request_panel::start_review_run::<D>),
        )
        .route(
            "/review_runs/:repository/:number",
            get(crate::pull_request_panel::get_review_runs::<D>),
        )
        .merge(crate::review_walks::mounted::<D>())
        .route(
            "/pull_request_notes/:repository/:number",
            get(crate::pull_request_notes::list_pull_request_notes::<D>)
                .post(crate::pull_request_notes::save_pull_request_note::<D>),
        )
        .route(
            "/pull_request_notes/:repository/:number/:id/remove",
            post(crate::pull_request_notes::remove_pull_request_note::<D>),
        )
        .route(
            "/pull_request_notes/:repository/:number/sent",
            post(crate::pull_request_notes::send_pull_request_notes::<D>),
        )
        .route("/mods", get(crate::mods::list_mods::<D>))
        .route("/mods/scaffold", post(crate::mods::scaffold_mod::<D>))
        .route("/mods/enable", post(crate::mods::set_mod_enabled::<D>))
        .route("/mods/validate", get(crate::mods::validate_mod::<D>))
        .route("/mods/promote", post(crate::mods::promote_mod::<D>))
        .route("/mods/try", post(crate::mods::try_tracker_mod::<D>))
        .route("/mods/session", post(crate::mods::start_mod_session::<D>))
        .route("/sessions", get(crate::sessions::list_sessions::<D>))
        .route("/sessions/owner", get(crate::sessions::who_owns::<D>))
        .route("/logbook/recall", get(crate::sessions::recall::<D>))
        .route("/logbook/subagent", post(crate::sessions::get_logbook_for_subagent::<D>))
        .route("/sessions/loadout", get(crate::sessions::get_session_loadout::<D>))
        .route("/sessions/attachment_history", get(crate::sessions::get_attachment_history::<D>))
        .route("/sessions/usage", get(crate::sessions::get_session_usage::<D>))
        .route("/usage/summary", get(crate::sessions::get_usage_summary::<D>))
        .route("/usage/rules", get(crate::sessions::get_usage_rules::<D>))
        .route("/sessions/loadout/item", get(crate::sessions::read_loadout_item::<D>))
        .route(
            "/sessions/rename",
            post(crate::sessions::rename_session::<D>),
        )
        .route("/sessions/name", post(crate::sessions::name_session::<D>))
        .route(
            "/sessions/run/start",
            post(crate::sessions::start_session_run::<D>),
        )
        .route("/sessions/shell", post(crate::sessions::run_session_shell::<D>))
        .route(
            "/sessions/run/stop",
            post(crate::sessions::stop_session_run::<D>),
        )
        .route(
            "/sessions/run/write",
            post(crate::sessions::write_session_run::<D>),
        )
        .route(
            "/sessions/run/resize",
            post(crate::sessions::resize_session_run::<D>),
        )
        .route(
            "/sessions/run/:run_id/observe",
            get(crate::sessions::observe_session_run::<D>),
        )
        .route("/sessions/terminal/open", post(crate::terminals::open_terminal::<D>))
        .route("/sessions/terminal/run", post(crate::terminals::start_terminal_run::<D>))
        .route("/sessions/terminal/type", post(crate::terminals::type_in_terminal::<D>))
        .route("/sessions/terminal/read", get(crate::terminals::read_terminal::<D>))
        .route(
            "/sessions/window",
            post(crate::sessions::show_window::<D>),
        )
        .route(
            "/sessions/withdraw_window",
            post(crate::sessions::withdraw_window::<D>),
        )
        .route("/sessions/sketch", post(crate::sessions::show_sketch::<D>))
        .route("/sessions/show_file", post(crate::sessions::show_file::<D>))
        .route("/sessions/shown", get(crate::shown::get_shown_file::<D>))
        .route("/sessions/plan", post(crate::sessions::record_plan::<D>))
        .route("/sessions/plan/approve", post(crate::sessions::approve_plan::<D>))
        .route("/sessions/plan/changes", post(crate::sessions::send_plan_changes::<D>))
        .route("/sessions/judge", post(crate::sessions::judge_session::<D>))
        .route("/sessions/judge/send", post(crate::sessions::send_concern::<D>))
        .route("/sessions/judge/dismiss", post(crate::sessions::dismiss_concern::<D>))
        .route("/adversarial_review", post(crate::adversarial_review::start_adversarial_review::<D>))
        .route("/adversarial_review/send", post(crate::adversarial_review::send_finding::<D>))
        .route("/adversarial_review/dismiss", post(crate::adversarial_review::dismiss_finding::<D>))
        .route(
            "/sessions/window/ask",
            post(crate::sessions::ask_in_window::<D>),
        )
        .route("/sessions/draft", post(crate::sessions::propose_draft::<D>))
        .route("/sessions/withdraw_draft", post(crate::sessions::withdraw_draft::<D>))
        .route(
            "/sessions/window/closed",
            post(crate::sessions::window_closed::<D>),
        )
        .route(
            "/sessions/window/sign_in",
            post(crate::sessions::take_window_sign_in::<D>),
        )
        .route(
            "/sessions/window/failed",
            post(crate::sessions::window_failed::<D>),
        )
        .route("/plan_usage", get(crate::hosted_sessions::get_plan_usage::<D>))
        .route("/model_downshift", get(crate::hosted_sessions::get_model_downshift::<D>))
        .route("/model_downshift/accept", post(crate::hosted_sessions::accept_downshift::<D>))
        .route("/model_downshift/restore", post(crate::hosted_sessions::restore_downshift::<D>))
        .route("/model_downshift/keep", post(crate::hosted_sessions::keep_downshift::<D>))
        .route("/model_downshift/settle", post(crate::hosted_sessions::settle_downshift::<D>))
        .route(
            "/agent/sign_in",
            get(crate::hosted_sessions::get_agent_sign_in::<D>).post(crate::hosted_sessions::start_agent_sign_in::<D>),
        )
        .route("/sleep", get(crate::hosted_sessions::get_sleep::<D>).post(crate::hosted_sessions::set_sleep::<D>))
        .route("/sleep/override", post(crate::hosted_sessions::override_sleep::<D>))
        .route("/sleep/review", post(crate::hosted_sessions::review_sleep::<D>))
        .route("/sessions/waiting", post(crate::sessions::waiting_for::<D>))
        .route("/sessions/done", post(crate::sessions::all_done::<D>))
        .route("/sessions/defer_later", post(crate::sessions::defer_later::<D>))
        .route("/sessions/settle_later", post(crate::sessions::settle_later::<D>))
        .route("/sessions/dispute_fleet", post(crate::sessions::dispute_fleet::<D>))
        .route("/sessions/finish_up", post(crate::sessions::finish_up::<D>))
        .route("/sessions/annotation_done", post(crate::sessions::mark_annotation_done::<D>))
        .route(
            "/sessions/pull_request_note_resolved",
            post(crate::sessions::resolve_pull_request_note::<D>),
        )
        .route(
            "/sessions/waiting/answer",
            post(crate::hosted_sessions::answer_waiting::<D>),
        )
        .route(
            "/sessions/waiting/dismiss",
            post(crate::hosted_sessions::dismiss_waiting::<D>),
        )
        .route(
            "/sessions/waiting/confirm",
            post(crate::hosted_sessions::confirm_waiting::<D>),
        )
        .route(
            "/sessions/later/confirm",
            post(crate::hosted_sessions::confirm_later::<D>),
        )
        .route(
            "/sessions/checks",
            post(crate::sessions::report_session_check::<D>),
        )
        .route(
            "/sessions/slot_wanted",
            post(crate::sessions::ask_for_slot::<D>),
        )
        .route(
            "/sessions/checks/:run/output",
            get(crate::sessions::get_session_check_output::<D>),
        )
        .route(
            "/sessions/checks/:run/observe",
            get(crate::sessions::observe_session_check::<D>),
        )
        .route(
            "/sessions/checks/:run/stop",
            post(crate::sessions::stop_session_check::<D>),
        )
        .route(
            "/sessions/checks/:run/force_stop",
            post(crate::sessions::force_stop_session_check::<D>),
        )
        .route(
            "/sessions/claim_pull_request",
            post(crate::sessions::claim_pull_request::<D>),
        )
        .route(
            "/sessions/claim_issue",
            post(crate::sessions::claim_issue::<D>),
        )
        .route(
            "/sessions/take_main_red",
            post(crate::sessions::take_main_red::<D>),
        )
        .route(
            "/sessions/start_another",
            post(crate::sessions::start_another_session::<D>),
        )
        .route(
            "/sessions/change_waiting",
            post(crate::sessions::change_waiting::<D>),
        )
        .route(
            "/sessions/fork_this",
            post(crate::sessions::fork_this_session::<D>),
        )
        .route(
            "/sessions/start",
            post(crate::hosted_sessions::start_session::<D>),
        )
        .route(
            "/sessions/hand_to_session",
            post(crate::hosted_sessions::hand_to_session::<D>),
        )
        .route(
            "/sessions/read_wait",
            post(crate::hosted_sessions::read_session_wait::<D>),
        )
        .route(
            "/sessions/start_waiting",
            post(crate::hosted_sessions::start_waiting_now::<D>),
        )
        .route(
            "/sessions/message",
            post(crate::hosted_sessions::send_session_message::<D>).layer(
                axum::extract::DefaultBodyLimit::max(
                    crate::hosted_sessions::MOST_A_MESSAGE_MAY_BE,
                ),
            ),
        )
        .route(
            "/sessions/note",
            post(crate::hosted_sessions::send_session_note::<D>).layer(
                axum::extract::DefaultBodyLimit::max(
                    crate::hosted_sessions::MOST_A_MESSAGE_MAY_BE,
                ),
            ),
        )
        .route(
            "/sessions/notes",
            post(crate::hosted_sessions::send_session_notes::<D>).layer(
                axum::extract::DefaultBodyLimit::max(
                    crate::hosted_sessions::MOST_A_MESSAGE_MAY_BE,
                ),
            ),
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
            "/sessions/command",
            post(crate::hosted_sessions::run_session_command::<D>),
        )
        .route(
            "/sessions/close",
            post(crate::hosted_sessions::close_session::<D>),
        )
        .route(
            "/sessions/move",
            post(crate::hosted_sessions::move_session::<D>),
        )
        .route(
            "/sessions/stop",
            post(crate::hosted_sessions::stop_session::<D>),
        )
        .route(
            "/sessions/handoff/retry",
            post(crate::hosted_sessions::retry_handoff::<D>),
        )
        .route(
            "/sessions/pause",
            post(crate::hosted_sessions::pause_session::<D>),
        )
        .route(
            "/sessions/auto_land",
            post(crate::hosted_sessions::set_session_auto_land::<D>),
        )
        .route(
            "/sessions/mcp",
            get(crate::hosted_sessions::get_session_mcp::<D>),
        )
        .route(
            "/sessions/mcp/act",
            post(crate::hosted_sessions::act_on_session_mcp::<D>),
        )
        .route(
            "/sessions/resume",
            post(crate::hosted_sessions::resume_session::<D>),
        )
        .route(
            "/sessions/workflow",
            post(crate::hosted_sessions::set_session_workflow::<D>),
        )
        .route(
            "/sessions/begin",
            post(crate::hosted_sessions::begin_session::<D>),
        )
        .route(
            "/sessions/release_holds",
            post(crate::hosted_sessions::release_session_holds::<D>),
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
        .route("/repositories/of_request", get(read_request_repository::<D>))
        .route("/repositories/add", post(add_repository::<D>))
        .route("/repositories/clone", post(clone_repository::<D>))
        .route("/import/folders", get(crate::importing::list_import_folders::<D>))
        .route("/import/runs", get(crate::importing::get_import_runs::<D>))
        .route("/import/install_mod", post(crate::importing::install_agent_mod::<D>))
        .route("/repositories/runs", get(crate::run_memory::get_run_memory::<D>))
        .route(
            "/repositories/runs/place",
            get(crate::run_memory::get_run_memory_place::<D>).post(crate::run_memory::set_run_memory_place::<D>),
        )
        .route("/notices/mute", post(crate::notice_mutes::set_notice_mute::<D>))
        .route("/repositories/runs/drop", post(crate::run_memory::drop_run_fact::<D>))
        .route(
            "/repositories/flight_controls",
            get(crate::kit::get_repository_flight_controls::<D>).post(crate::kit::save_repository_flight_controls::<D>),
        )
        .route("/repositories/flight_controls/approve", post(crate::kit::approve_repository_tile::<D>))
        .route("/repositories/kit_manifest", post(crate::kit::save_kit_manifest::<D>))
        .route("/share/package", post(crate::sharing::share_package::<D>))
        .route("/share/read", post(crate::sharing::read_package::<D>))
        .route("/share/add", post(crate::sharing::add_package::<D>))
        .route("/share/unbound", get(crate::sharing::list_unbound_pieces::<D>))
        .route("/share/bind", post(crate::sharing::bind_unbound_piece::<D>))
        .route("/repositories/commit", post(crate::kit::commit_to_repository::<D>))
        .route("/repositories/create", post(create_project::<D>))
        .route("/merge_lines", get(get_merge_lines::<D>))
        .route("/merge_lines/checkout_diff", get(checkout_diff::<D>))
        .route("/timeline", get(get_fleet_timeline::<D>))
        .route("/slot_history", get(get_slot_history::<D>))
        .route("/merge_lines/checks/observe", get(observe_land_check::<D>))
        .route("/merge_lines/catch_up", post(catch_up_checkout::<D>))
        .route("/merge_lines/checkout_to_pull_request", post(checkout_to_pull_request::<D>))
        .route("/models", get(list_models::<D>))
        .route("/fleet/build", get(get_fleet_build::<D>))
        .route("/fleet/build/change", post(change_fleet_build::<D>))
        .route("/limits", get(get_limits::<D>))
        .route("/limits/save", post(save_limits::<D>))
        .route("/preferences", get(get_preferences::<D>))
        .route("/preferences/save", post(save_preferences::<D>))
        .route("/settings", get(crate::settling::get_settings::<D>))
        .route("/settings/save", post(crate::settling::save_settings::<D>))
        .route("/phone", get(crate::settling::get_phone::<D>).post(crate::settling::set_phone::<D>))
        .route("/accounts", get(crate::accounting::get_accounts::<D>))
        .route("/accounts/start_sign_in", post(crate::accounting::start_sign_in::<D>))
        .route("/accounts/sign_out", post(crate::accounting::sign_out::<D>))
        .route("/accounts/set_repository_profile", post(crate::accounting::set_repository_profile::<D>))
        .route("/shared_home", get(crate::homing::get_shared_home::<D>))
        .route("/shared_home/sync", post(crate::homing::sync_shared_home::<D>))
        .route("/shared_home/add", post(crate::homing::add_to_shared_home::<D>))
        .route("/shared_home/bring", post(crate::homing::bring_from_shared_home::<D>))
        .route("/shared_home/choose", post(crate::homing::choose_shared_home_side::<D>))
        .route("/shared_home/kinds", get(crate::homing::get_home_kinds::<D>))
        .route("/shared_home/kinds/set", post(crate::homing::set_home_kind::<D>))
        .route("/shared_home/items/set", post(crate::homing::set_home_item::<D>))
        .route("/shared_home/repository", post(crate::homing::set_home_repository::<D>))
        .route("/projects", get(crate::projects::list_projects::<D>).post(crate::projects::add_project::<D>))
        .route("/projects/project/:project/rename", post(crate::projects::rename_project::<D>))
        .route("/projects/project/:project/delete", post(crate::projects::delete_project::<D>))
        .route("/projects/tasks", post(crate::projects::add_project_task::<D>))
        .route("/projects/issues", post(crate::projects::link_project_issue::<D>))
        .route("/projects/rows/:row/subtasks", post(crate::projects::add_project_subtask::<D>))
        .route("/projects/rows/:row/issues", post(crate::projects::link_task_issue::<D>))
        .route("/projects/rows/:row/issues/unlink", post(crate::projects::unlink_task_issue::<D>))
        .route("/projects/rows/:row/job", post(crate::projects::link_task_job::<D>))
        .route("/projects/rows/:row/job/unlink", post(crate::projects::unlink_task_job::<D>))
        .route("/projects/rows/:row/pulls", post(crate::projects::link_task_pull::<D>))
        .route("/projects/rows/:row/pulls/unlink", post(crate::projects::unlink_task_pull::<D>))
        .route("/projects/rows/:row/edit", post(crate::projects::edit_project_task::<D>))
        .route("/projects/rows/:row/remove", post(crate::projects::remove_project_row::<D>))
        .route("/projects/rows/:row/move", post(crate::projects::move_project_row::<D>))
        .route("/projects/rows/:row/tick", post(crate::projects::tick_project_task::<D>))
        .route("/health", get(get_health::<D>))
        .route("/manifests/:manifest_id", get(get_manifest::<D>))
        .route("/events/since", get(get_events_since::<D>))
        .route("/manifest/reading", get(get_manifest_reading::<D>))
        .route("/manifest/drift", get(get_manifest_drift::<D>))
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
        .route("/test_map/reaching", get(crate::test_map::get_tests_reaching::<D>))
        .route(
            "/manifest/allowed_commands",
            get(get_repository_allowed_commands::<D>),
        )
        .route(
            "/manifest/allowed_commands/remove",
            post(remove_repository_allowed_command::<D>),
        )
        .route("/requests/send", post(crate::requests::send_request::<D>))
        .route("/requests", get(crate::requests::get_requests::<D>))
        .route("/requests/collections", post(crate::requests::save_request_collection::<D>))
        .route(
            "/requests/collections/request",
            post(crate::requests::save_request_to_collection::<D>),
        )
        .route("/requests/environments", post(crate::requests::save_request_environment::<D>))
        .route(
            "/requests/environments/delete",
            post(crate::requests::delete_request_environment::<D>),
        )
        .route("/watches", get(crate::watches::list_watches::<D>))
        .route("/watches/start", post(crate::watches::start_watch::<D>))
        .route("/watches/weigh", post(crate::watches::weigh_watch::<D>))
        .route("/watches/stop", post(crate::watches::stop_watch::<D>))
        .route("/watches/delete", post(crate::watches::delete_watch::<D>))
        .route("/watches/log", get(crate::watches::read_watch_log::<D>))
        .route("/watches/try", post(crate::watches::try_watch::<D>))
        .route("/watches/recipes", get(crate::watches::list_watch_recipes::<D>))
        .route("/watches/recipes/save", post(crate::watches::save_watch_recipe::<D>))
        .route("/app_runs", get(crate::app_runs::list_app_runs::<D>))
        .route("/app_runs/draft", get(crate::app_runs::get_app_run_draft::<D>))
        .route("/app_runs/start", post(crate::app_runs::start_app_run::<D>))
        .route("/app_runs/stop", post(crate::app_runs::stop_app_run::<D>))
        .route("/app_runs/restart", post(crate::app_runs::restart_app_run::<D>))
        .route("/app_runs/save", post(crate::app_runs::save_app_run::<D>))
        .route("/app_runs/pin", post(crate::app_runs::pin_app_run_port::<D>))
        .route("/app_runs/unpin", post(crate::app_runs::unpin_app_run_port::<D>))
        .route("/kit/inventory", get(crate::kit::get_kit_inventory::<D>))
        .route(
            "/kit/flight_controls",
            get(crate::kit::get_flight_controls::<D>).post(crate::kit::save_flight_controls::<D>),
        )
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
        .route("/kit/pieces/look", post(crate::kit::look_at_kit_source::<D>))
        .route("/kit/pieces/install", post(crate::kit::install_kit_piece::<D>))
        .route("/kit/pieces/remove", post(crate::kit::remove_kit_piece::<D>))
        .route("/kit/pieces/restore", post(crate::kit::restore_kit_piece::<D>))
        .route(
            "/sessions/:session_id/retro",
            get(crate::retros::get_session_retro::<D>).post(crate::retros::write_session_retro::<D>),
        )
        .route("/logbook", get(crate::logbook::get_logbook::<D>))
        .route("/logbook/entries", post(crate::logbook::teach_logbook_entry::<D>))
        .route("/logbook/entries/:entry_id/edit", post(crate::logbook::edit_logbook_entry::<D>))
        .route("/logbook/entries/:entry_id/lock", post(crate::logbook::lock_logbook_entry::<D>))
        .route("/logbook/entries/:entry_id/forget", post(crate::logbook::forget_logbook_entry::<D>))
        .route("/logbook/tidy", post(crate::logbook::tidy_logbook::<D>))
        .route("/teachings", post(crate::teaching::save_teaching::<D>))
        .route("/teachings/read", post(crate::teaching::read_teaching::<D>))
        .route("/teachings/words", get(crate::teaching::list_word_rules::<D>))
        .route("/lessons", get(crate::retros::list_lessons::<D>))
        .route("/lessons/review", post(crate::retros::review_lessons::<D>))
        .route(
            "/lessons/:lesson_id/ask",
            post(crate::retros::ask_lesson::<D>),
        )
        .route(
            "/lessons/:lesson_id/session",
            post(crate::retros::start_lesson_session::<D>),
        )
        .route(
            "/lessons/:lesson_id/disagree",
            post(crate::retros::disagree_lesson::<D>),
        )
        .route("/manifest/run_sheet", get(get_checkout_run_sheet::<D>))
        .route("/manifest/runs", get(list_checkout_runs::<D>))
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
        .route("/worktrees", get(list_worktrees::<D>))
        .route("/worktrees/slots", post(change_slot_pool::<D>))
        .route("/worktrees/slots/rescue", post(rescue_slot::<D>))
        .route("/helm/session", get(get_helm_session::<D>))
        .route("/helm/ask", post(ask_helm::<D>))
        .route("/helm/debug", get(get_helm_debug_info::<D>))
        .route("/helm/start_fresh", post(start_helm_fresh::<D>))
        .route("/helm/read_thought", post(read_thought::<D>))
        .route("/helm/thought_sent", post(record_thought_sent::<D>))
        .route("/helm/tell_session", post(tell_session::<D>))
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
        .with_state(served)
        // Over every route on both copies of the surface, so a call the door
        // makes is named as the door's and not as a request that named nobody.
        .layer(axum::middleware::from_fn(crate::acting::named_the_door))
}
