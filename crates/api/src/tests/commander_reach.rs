//! What a Job's Commander may reach through the agent's door. `#2164`.
//!
//! **Two things are held here.** The list: every operation the door could offer
//! is a query, is allowed, or is on the ledger below as reviewed and refused,
//! so a new operation fails [`every_operation_the_door_offers_is_tagged`] until
//! somebody decides it. And the door: a Commander is placed by its connection,
//! is listed and answered by the allowlist alone, and is recorded as itself.

use std::net::SocketAddr;
use std::sync::Arc;

use axum::body::Body;
use axum::http::Request;
use axum::Router;
use http_body_util::BodyExt;
use tower::ServiceExt;

use crate::tests::fake::{running, FakeDaemon};
use crate::tests::shapes::run_id;
use crate::{
    offerable, offerable_to_a_commander, router, Broadcaster, CommanderReach, Served, Steerer, COMMANDER_ALLOWED, DOOR_PATH,
    SERVED,
};

// The operations reviewed and refused, by why. **A reason is a label for a
// reader and decides nothing**: the allowlist is the only thing that grants,
// and anything not on it is refused whether or not it is listed here.

/// Passes a gate, or rules on whether work is good enough.
const RULES_ON_THE_WORK: &[&str] = &[
    "override_verdict",
    "rerun_gate",
    "rerun_failed_checks",
    "rerun_checks",
    "retry_check",
    "skip_check",
    "skip_checks",
    "set_when_blocked",
    "set_when_refused",
    "submit_for_verification",
    "approve_dispatch",
    "deny_dispatch",
    "approve_review",
    "approve_wave",
    "request_changes",
    "take_up_remarks",
    "reject_job",
    "dismiss_finding",
    "answer_judge",
];

/// Attests, merges, pushes or decides where work lands.
const ATTESTS_MERGES_OR_PUSHES: &[&str] = &[
    "attest_complete",
    "merge_pull_request",
    "merge_pull_request_by_number",
    "enable_auto_merge",
    "enable_job_auto_merge",
    "ready_pull_request",
    "review_pull_request",
    "claim_pull_request",
    "delete_branch",
    "set_landing_target",
    "close_as_superseded",
];

/// Edits a Check, the Manifest, a workflow, a limit or what a machine allows.
const EDITS_CHECKS_MANIFEST_OR_SETTINGS: &[&str] = &[
    "edit_manifest",
    "edit_manifest_proposal",
    "write_manifest_proposal",
    "save_manifest_file",
    "save_workflow",
    "save_limits",
    "save_preferences",
    "set_tiers",
    "set_model",
    "set_review_model",
    "set_commander_model",
    "remove_allowed_command",
    "remove_kit_allowed_command",
    "remove_repository_allowed_command",
    "change_slot_pool",
    "add_repository",
    "clone_repository",
];

/// Ends, pauses, hands over or revives a Job, or takes back what it holds.
const THE_JOBS_LIFE: &[&str] = &[
    "kill_job",
    "kill_drone",
    "kill_one_drone",
    "kill_process",
    "kill_processes",
    "pause_job",
    "park_job",
    "resume_job",
    "resume_interrupted_job",
    "redispatch_job",
    "forget_job",
    "take_over",
    "edit_job",
    "to_proposer",
    "stop_proposal",
    "reclaim_worktree",
    "rescue_slot",
    "restart_fleet",
    "examine_job",
    "investigate_failed_checks",
];

/// Drafts a Job, works a Studio, or files what a person reads.
const DRAFTS_OR_FILES: &[&str] = &[
    "propose_job",
    "propose_from_request",
    "ask_person_to_approve",
    "add_studio_node",
    "propose_studio_edge",
    "rename_studio",
    "dispatch_studio_draft",
    "write_up_studio_node",
    "start_studio_run",
    "start_studio_server",
    "start_scout",
    "read_in_link",
    "queue_after_finding",
    "file_finding_issue",
    "file_report",
    "start_lesson_session",
    "ask_lesson",
    "agree_lesson",
    "disagree_lesson",
    "review_lessons",
];

/// Answers for the person, or reaches him by a channel that is not the
/// Commander's own (`#2166` gives it one).
const ANSWERS_FOR_OR_REACHES_THE_PERSON: &[&str] = &[
    "answer_command",
    "answer_question",
    "ask_the_person",
    "show_window",
    "show_again",
    "ask_in_window",
    "waiting_for",
    "all_done",
    "tell_session",
    "start_another_session",
    "fork_this_session",
];

/// The person's own checkout, runs, servers and mods.
const THE_PERSONS_OWN_TOOLS: &[&str] = &[
    "start_checkout_run",
    "start_checkout_verify",
    "stop_checkout_run",
    "undo_checkout_run",
    "start_run",
    "stop_run",
    "undo_run",
    "start_server",
    "stop_server",
    "scaffold_mod",
];

fn ledger() -> impl Iterator<Item = &'static str> {
    [
        RULES_ON_THE_WORK,
        ATTESTS_MERGES_OR_PUSHES,
        EDITS_CHECKS_MANIFEST_OR_SETTINGS,
        THE_JOBS_LIFE,
        DRAFTS_OR_FILES,
        ANSWERS_FOR_OR_REACHES_THE_PERSON,
        THE_PERSONS_OWN_TOOLS,
    ]
    .into_iter()
    .flatten()
    .copied()
}

/// **The definition of done.** Adding an operation with no tag fails here, and
/// the failure names it.
#[test]
fn every_operation_the_door_offers_is_tagged() {
    let untagged: Vec<&str> = offerable_to_a_commander()
        .filter(|row| row.kind != "query")
        .map(|row| row.operation)
        .filter(|operation| {
            !COMMANDER_ALLOWED.contains(operation) && !ledger().any(|tagged| tagged == *operation)
        })
        .collect();
    assert!(
        untagged.is_empty(),
        "{untagged:?} are offered by the door and neither on the Commander's allowlist \
         (`daemon/commander.rs`) nor reviewed and refused in this ledger. Decide each: \
         a Commander acts with nobody watching"
    );
}

#[test]
fn the_allowlist_and_the_ledger_name_only_what_the_door_offers_and_never_the_same_twice() {
    let offered: Vec<&str> = offerable_to_a_commander().map(|row| row.operation).collect();
    let mut seen: Vec<&str> = Vec::new();
    for operation in COMMANDER_ALLOWED.iter().copied().chain(ledger()) {
        assert!(
            offered.contains(&operation),
            "`{operation}` is not offered by the door"
        );
        assert!(!seen.contains(&operation), "`{operation}` is tagged twice");
        seen.push(operation);
    }
    for operation in COMMANDER_ALLOWED {
        let row = offerable_to_a_commander()
            .find(|row| row.operation == *operation)
            .expect("offered");
        assert_eq!(
            row.kind, "command",
            "`{operation}` is a query, which needs no entry"
        );
    }
}

#[test]
fn a_commander_may_call_every_query_and_every_allowed_act_and_nothing_else() {
    let reach = CommanderReach::of_the_allowlist(ipc::JobId::carried("01JOB"));
    for row in offerable_to_a_commander() {
        let allowed = row.kind == "query" || COMMANDER_ALLOWED.contains(&row.operation);
        assert_eq!(reach.may(row.operation), allowed, "{}", row.operation);
    }
    for operation in [
        "override_verdict",
        "rerun_gate",
        "rerun_failed_checks",
        "set_when_blocked",
        "attest_complete",
        "submit_for_verification",
        "merge_pull_request",
        "merge_pull_request_by_number",
        "edit_manifest",
        "approve_dispatch",
        "add_job_step",
        "ask_the_person",
        "propose_job",
    ] {
        assert!(
            !reach.may(operation),
            "a Commander may not call `{operation}`"
        );
    }
    assert!(!reach.may("an_operation_nobody_has_written"));
}

/// A query that changed something would be a gate-passer the allowlist waves
/// through, so every query the door offers must be a read on the wire too.
#[test]
fn every_query_the_commander_may_call_is_a_read() {
    for row in offerable().filter(|row| row.kind == "query") {
        let route = SERVED
            .iter()
            .find(|route| route.operation == row.operation)
            .map(|route| route.method);
        assert!(
            matches!(route, Some("GET") | None),
            "`{}` is a query served as {route:?}",
            row.operation
        );
    }
}

pub(super) const COMMANDER: u16 = 52010;
pub(super) const ANYONE: u16 = 52011;

pub(super) fn commander_holding_one() -> Arc<FakeDaemon> {
    let daemon = FakeDaemon::new(Broadcaster::new());
    running(&daemon, "01JOB");
    *daemon.commander_on.lock().expect("not poisoned") = Some((COMMANDER, ipc::JobId::carried("01JOB")));
    Arc::new(daemon)
}

pub(super) fn shared(daemon: &Arc<FakeDaemon>) -> Router {
    router(Served::sharing(
        Arc::clone(daemon),
        run_id(),
        Broadcaster::new(),
    ))
}

pub(super) async fn from(app: &Router, port: u16, headers: &[(&str, &str)], body: &str) -> String {
    let peer: SocketAddr = format!("127.0.0.1:{port}").parse().expect("an address");
    let mut request = Request::builder()
        .method("POST")
        .uri(DOOR_PATH)
        .header("content-type", "application/json")
        .extension(axum::extract::ConnectInfo(peer));
    for (name, value) in headers {
        request = request.header(*name, *value);
    }
    let response = app
        .clone()
        .oneshot(
            request
                .body(Body::from(body.to_string()))
                .expect("a request"),
        )
        .await
        .expect("an answer");
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("a body")
        .to_bytes();
    String::from_utf8_lossy(&bytes).to_string()
}

pub(super) fn calling(tool: &str, arguments: &str) -> String {
    format!(
        r#"{{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{{"name":"{tool}","arguments":{arguments}}}}}"#
    )
}

const LISTING: &str = r#"{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}"#;
pub(super) const REDIRECT: &str = r#"{"job_id":"1","body":{"instruction":"read the failing test first"}}"#;

/// The names in a `tools/list` answer, read as the text it is.
fn listed(body: &str) -> Vec<String> {
    body.split("\"name\":\"")
        .skip(1)
        .filter_map(|rest| rest.split('"').next())
        .map(str::to_string)
        .collect()
}

#[tokio::test]
async fn a_commander_is_listed_its_allowlist_and_every_query_and_anyone_else_is_not() {
    let app = shared(&commander_holding_one());
    let mut commander = listed(&from(&app, COMMANDER, &[], LISTING).await);
    let mut expected: Vec<String> = offerable_to_a_commander()
        .filter(|row| row.kind == "query" || COMMANDER_ALLOWED.contains(&row.operation))
        .map(|row| row.operation.to_string())
        .collect();
    commander.sort();
    expected.sort();
    assert_eq!(commander, expected);

    let anyone = listed(&from(&app, ANYONE, &[], LISTING).await);
    assert!(anyone.iter().any(|name| name == "undo_run"), "{anyone:?}");
    assert!(
        !anyone.iter().any(|name| name == "restart_task"),
        "any agent is not offered Helm's rows"
    );
}

#[tokio::test]
async fn a_commander_asking_to_pass_a_gate_is_refused_by_name_and_reaches_no_route() {
    let daemon = commander_holding_one();
    let app = shared(&daemon);
    for tool in [
        "override_verdict",
        "rerun_gate",
        "attest_complete",
        "submit_for_verification",
        "merge_pull_request",
        "edit_manifest",
        "approve_dispatch",
        "undo_run",
    ] {
        let body = from(
            &app,
            COMMANDER,
            &[],
            &calling(tool, r#"{"job_id":"1","body":{}}"#),
        )
        .await;
        assert!(body.contains("\"isError\":true"), "{tool}: {body}");
        assert!(
            body.contains(&format!("`{tool}` is not the Commander's to call")),
            "{tool}: {body}"
        );
    }
    assert!(daemon.redirected_by.lock().expect("not poisoned").is_empty());
}

/// **Who acted is the connection's word.** The same bytes from a connection the
/// daemon did not place are a person's redirect, whatever they claim.
#[tokio::test]
async fn a_redirect_from_the_commander_is_the_commanders_and_is_never_claimed() {
    let daemon = commander_holding_one();
    let app = shared(&daemon);
    let redirect = calling("redirect_drone", REDIRECT);

    let body = from(&app, COMMANDER, &[], &redirect).await;
    assert!(!body.contains("not the Commander's to call"), "{body}");
    from(
        &app,
        ANYONE,
        &[("x-armada-caller", "commander")],
        &redirect,
    )
    .await;
    let claimed = calling(
        "redirect_drone",
        r#"{"job_id":"1","body":{"instruction":"go on","by":"commander","actor":"commander"}}"#,
    );
    from(&app, ANYONE, &[], &claimed).await;

    assert_eq!(
        *daemon.redirected_by.lock().expect("not poisoned"),
        vec![Steerer::Commander, Steerer::Person, Steerer::Person]
    );
    let doors = daemon.redirected_via.lock().expect("not poisoned").clone();
    let spelled: Vec<_> = doors
        .iter()
        .map(|via| via.map(|via| via.as_wire()))
        .collect();
    assert_eq!(spelled, vec![Some("commander"), Some("door"), Some("door")]);
}
