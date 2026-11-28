//! The door a hosted session stands at in a root with no `armada.yml`: Setup's
//! session, whose `--permission-prompt-tool` is on this door. With no tools
//! here the agent CLI ended at the first call that needed a permission answer.

use std::net::SocketAddr;
use std::sync::Arc;

use axum::body::Body;
use axum::http::Request;
use axum::Router;
use http_body_util::BodyExt;
use tower::ServiceExt;

use crate::tests::fake::FakeDaemon;
use crate::tests::shapes::run_id;
use crate::{door_without_a_manifest, router, Broadcaster, Served};

const SESSION: u16 = 52030;
const ANYONE: u16 = 52031;

const HANDSHAKE: &str =
    r#"{"jsonrpc":"2.0","id":0,"method":"initialize","params":{"protocolVersion":"2025-06-18"}}"#;
const LISTING: &str = r#"{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}"#;

fn hosting_a_session() -> (Arc<FakeDaemon>, Router) {
    let daemon = FakeDaemon::new(Broadcaster::new());
    *daemon.session_on.lock().expect("not poisoned") = Some(SESSION);
    let daemon = Arc::new(daemon);
    let app = router(Served::sharing(
        Arc::clone(&daemon),
        run_id(),
        Broadcaster::new(),
    ));
    (daemon, app)
}

async fn posted(app: &Router, port: u16, body: &str) -> String {
    let peer: SocketAddr = format!("127.0.0.1:{port}").parse().expect("an address");
    let request = Request::builder()
        .method("POST")
        .uri(door_without_a_manifest())
        .header("content-type", "application/json")
        .extension(axum::extract::ConnectInfo(peer))
        .body(Body::from(body.to_string()))
        .expect("a well-formed request");
    let response = app.clone().oneshot(request).await.expect("an answer");
    let body = response
        .into_body()
        .collect()
        .await
        .expect("a readable body")
        .to_bytes();
    String::from_utf8_lossy(&body).to_string()
}

fn calling(tool: &str, arguments: &str) -> String {
    format!(
        r#"{{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{{"name":"{tool}","arguments":{arguments}}}}}"#
    )
}

fn named(tool: &str) -> String {
    format!("\"name\":\"{tool}\"")
}

/// **The permission tool and the session's own acts**, and nothing answered
/// inside a Manifest, which would be another repository's.
#[tokio::test]
async fn a_hosted_session_with_no_manifest_is_offered_its_permission_tool_and_its_own_acts() {
    let (_, app) = hosting_a_session();

    let listed = posted(&app, SESSION, LISTING).await;

    for tool in [
        "ask_the_person",
        "waiting_for",
        "all_done",
        "show_window",
        "withdraw_window",
        "show_sketch",
        "withdraw_sketch",
        "withdraw_document",
        "show_file",
        "record_plan",
        "ask_in_window",
        "propose_draft",
        "withdraw_draft",
        "handoff_brief",
        "list_mods",
        "scaffold_mod",
        "validate_mod",
        "try_tracker_mod",
        "defer_later",
        "offer_preview",
        "withdraw_preview",
        "finish_up",
        "start_another_job",
        "list_started_jobs",
        "command_started_jobs",
        "start_watch",
        "list_manifests",
    ] {
        assert!(
            listed.contains(&named(tool)),
            "`{tool}` is offered: {listed}"
        );
    }
    for tool in [
        "list_jobs",
        "get_events_since",
        "list_sessions",
    ] {
        assert!(
            !listed.contains(&named(tool)),
            "`{tool}` needs a Manifest: {listed}"
        );
    }
    let said = posted(&app, SESSION, HANDSHAKE).await;
    assert!(said.contains("No armada.yml is served here"), "{said}");
    assert!(!said.contains("inside Manifest"), "{said}");
}

/// **A Job's terminals stand in its own worktree**, so all four tools are read inside no Manifest.
#[tokio::test]
async fn a_hosted_session_with_no_manifest_is_offered_its_terminals() {
    let (_, app) = hosting_a_session();

    let listed = posted(&app, SESSION, LISTING).await;

    for tool in ["type_in_terminal", "read_terminal", "list_terminals", "show_terminal"] {
        assert!(listed.contains(&named(tool)), "`{tool}` is offered: {listed}");
    }
    assert!(!listed.contains(&named("rename_terminal")), "{listed}");
}

/// **The Operator reads a repository's issues and writes none**, for the backlog report it writes
/// on a page. A session that is not the Operator's is offered none of them.
#[tokio::test]
async fn the_operator_is_offered_the_issue_reads_and_another_session_is_not() {
    let (daemon, app) = hosting_a_session();
    let another = posted(&app, SESSION, LISTING).await;
    *daemon.session_is_the_operator.lock().expect("not poisoned") = true;

    let operator = posted(&app, SESSION, LISTING).await;

    for tool in ["list_issues", "get_issue", "search_issues"] {
        assert!(operator.contains(&named(tool)), "`{tool}` is offered to the Operator: {operator}");
        assert!(!another.contains(&named(tool)), "`{tool}` is the Operator's alone: {another}");
    }
    for tool in ["claim_issue", "link_task_issue", "save_issue_query"] {
        assert!(!operator.contains(&named(tool)), "`{tool}` writes, and is not offered: {operator}");
    }
    let listed = posted(&app, SESSION, &calling("list_issues", r#"{"repository":"armada"}"#)).await;
    assert!(!listed.contains("\"isError\":true"), "{listed}");
}

/// **The Operator is not a Job**, so it is offered none of a Job's own acts and a call to one is
/// refused. It once said "I'm not calling all_done" in its thread, because the door listed it.
#[tokio::test]
async fn the_operator_is_offered_no_job_only_act_and_a_call_to_one_is_refused() {
    let (daemon, app) = hosting_a_session();
    let job = posted(&app, SESSION, LISTING).await;
    *daemon.session_is_the_operator.lock().expect("not poisoned") = true;

    let operator = posted(&app, SESSION, LISTING).await;

    for tool in ["waiting_for", "list_closed_waiting", "all_done", "finish_up", "record_plan", "offer_preview", "withdraw_preview", "defer_later", "settle_later", "raise_conflict", "ask_held", "withdraw_held", "settle_clash", "write_feature_sections"] {
        assert!(job.contains(&named(tool)), "`{tool}` is a Job's: {job}");
        assert!(!operator.contains(&named(tool)), "`{tool}` is a Job's alone, and offered to the Operator: {operator}");
    }
    for tool in ["show_sketch", "report_bug", "start_another_job", "handoff_brief", "list_issues"] {
        assert!(operator.contains(&named(tool)), "`{tool}` is still the Operator's: {operator}");
    }
    let refused = posted(&app, SESSION, &calling("all_done", r#"{"summary":"done"}"#)).await;
    assert!(refused.contains("\"isError\":true"), "{refused}");
    assert!(refused.contains("not a tool this Fleet offers"), "{refused}");
}

/// At a door inside a Manifest the reach decides, and it refuses the same acts by name.
#[test]
fn the_operators_reach_refuses_a_jobs_own_acts_by_name() {
    let job = crate::WordsReach::of_a_hosted_session();
    let operator = job.clone().the_operator();
    assert!(job.may("all_done"));
    assert!(!operator.may("all_done"));
    assert!(!operator.may("waiting_for"));
    assert!(operator.may("show_sketch"));
    assert!(operator.refusing("all_done").starts_with("`all_done` is not the Operator's to call"), "{}", operator.refusing("all_done"));
}

/// What `--permission-prompt-tool` sends, answered by the daemon rather than
/// refused as a tool that is not there.
#[tokio::test]
async fn its_permission_prompt_is_answered() {
    let (daemon, app) = hosting_a_session();

    let answered = posted(
        &app,
        SESSION,
        &calling(
            "ask_the_person",
            r#"{"tool_name":"Bash","input":{"command":"ls"}}"#,
        ),
    )
    .await;

    assert!(answered.contains("allow"), "{answered}");
    assert!(!answered.contains("not a tool"), "{answered}");
    assert_eq!(daemon.asked_to_run.lock().expect("not poisoned").len(), 1);
}

/// **The machine's mods are read inside no Manifest**, so a Mod Job in the mods folder makes
/// and checks one here.
#[tokio::test]
async fn a_mod_is_made_and_checked_through_it() {
    let (_, app) = hosting_a_session();

    let made = posted(&app, SESSION, &calling("scaffold_mod", r#"{"body":{"name":"calmer"}}"#)).await;
    let checked = posted(&app, SESSION, &calling("validate_mod", r#"{"name":"calmer"}"#)).await;

    for answered in [&made, &checked] {
        assert!(!answered.contains("\"isError\":true"), "{answered}");
        assert!(answered.contains("calmer"), "{answered}");
    }
}

#[tokio::test]
async fn a_tool_that_needs_a_manifest_is_refused_saying_why() {
    let (_, app) = hosting_a_session();

    let refused = posted(&app, SESSION, &calling("list_jobs", "{}")).await;

    assert!(refused.contains("\"isError\":true"), "{refused}");
    assert!(refused.contains("no armada.yml yet"), "{refused}");
}

/// **A connection that is no hosted session is offered nothing**: the acts
/// offered here are a session's own, and it has none.
#[tokio::test]
async fn a_caller_that_is_no_hosted_session_is_offered_nothing() {
    let (daemon, app) = hosting_a_session();

    let listed = posted(&app, ANYONE, LISTING).await;
    assert!(listed.contains("\"tools\":[]"), "{listed}");
    let said = posted(&app, ANYONE, HANDSHAKE).await;
    assert!(said.contains("not a job Armada is hosting"), "{said}");
    let refused = posted(
        &app,
        ANYONE,
        &calling(
            "ask_the_person",
            r#"{"tool_name":"Bash","input":{"command":"ls"}}"#,
        ),
    )
    .await;
    assert!(refused.contains("\"isError\":true"), "{refused}");
    assert!(daemon.asked_to_run.lock().expect("not poisoned").is_empty());
}

/// **Every act the rules name is offered here.** A hosted Job is told these rules and a repository
/// with no `armada.yml` has no other door, so a name the rules gain that this door does not list is
/// a Job told to call what is not there. It was 13 acts before this held.
#[test]
fn every_act_the_rules_name_is_offered_with_no_manifest() {
    let rules = include_str!("../../../adapters/src/session_rules.md").replace("mcp__armada-fleet__", "");
    let operations: Vec<&str> = crate::SERVED.iter().map(|route| route.operation).collect();
    let offered = crate::offered_without_a_manifest();
    let mut named = Vec::new();
    for span in rules.split('`').skip(1).step_by(2) {
        for word in span.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_')) {
            let operation = ipc::door::RENAMED.iter().find(|(tool, _)| *tool == word).map_or(word, |(_, operation)| *operation);
            if operations.contains(&operation) {
                named.push((word, operation));
            }
        }
    }
    assert!(named.len() >= 10, "the rules name operations, or this read none: {named:?}");
    for (word, operation) in named {
        assert!(offered.contains(&operation), "`{word}` is named in the rules and is not offered with no armada.yml");
    }
}

/// **A tool the Operator's project switched off is left out of what it is offered, and a call to it is refused by the
/// switch's name**, not as a tool that does not exist. The same call goes through where the switch is on.
#[tokio::test]
async fn an_act_its_project_switched_off_is_unlisted_and_refused_by_the_switchs_name() {
    let (daemon, app) = hosting_a_session();
    *daemon.session_is_the_operator.lock().expect("not poisoned") = true;
    let on = posted(&app, SESSION, LISTING).await;
    for tool in ["close_session", "start_another_job", "merge_pull_request_by_number", "show_sketch"] {
        assert!(on.contains(&named(tool)), "`{tool}` is offered while its switch is on: {on}");
    }

    *daemon.operator_switched_off.lock().expect("not poisoned") = vec!["close_session", "start_another_session", "merge_pull_request_by_number"];
    let off = posted(&app, SESSION, LISTING).await;
    for tool in ["close_session", "start_another_job", "merge_pull_request_by_number"] {
        assert!(!off.contains(&named(tool)), "`{tool}` is left out while its switch is off: {off}");
    }
    assert!(off.contains(&named("show_sketch")), "{off}");
    let refused = posted(&app, SESSION, &calling("close_session", r#"{"body":{"session_id":"s1"}}"#)).await;
    assert!(refused.contains("\"isError\":true"), "{refused}");
    assert!(refused.contains("`close_session` is off for this project's Operator: Settings, Operator, Tools"), "{refused}");
    assert!(!refused.contains("not a tool this Fleet offers"), "{refused}");

    daemon.operator_switched_off.lock().expect("not poisoned").clear();
    let allowed = posted(&app, SESSION, &calling("close_session", r#"{"body":{"session_id":"s1"}}"#)).await;
    assert!(!allowed.contains("is off for this project's Operator"), "{allowed}");
}

/// **An act held until he has said so stays listed**, since a tool list an agent read at its start cannot follow a say that opens
/// later, and a call to it is refused at the call.
#[tokio::test]
async fn an_act_held_until_he_has_said_so_stays_listed_and_its_call_is_refused() {
    let (daemon, app) = hosting_a_session();
    *daemon.session_is_the_operator.lock().expect("not poisoned") = true;
    *daemon.operator_unsaid.lock().expect("not poisoned") = vec!["close_session"];

    let listed = posted(&app, SESSION, LISTING).await;
    assert!(listed.contains(&named("close_session")), "{listed}");
    let refused = posted(&app, SESSION, &calling("close_session", r#"{"body":{"session_id":"s1"}}"#)).await;
    assert!(refused.contains("waits for him: on a card or in the talk"), "{refused}");

    daemon.operator_unsaid.lock().expect("not poisoned").clear();
    let allowed = posted(&app, SESSION, &calling("close_session", r#"{"body":{"session_id":"s1"}}"#)).await;
    assert!(!allowed.contains("waits for him"), "{allowed}");
}

/// The reach's own words: a withheld act names its switch whether or not the reach ever listed it.
#[test]
fn a_withheld_act_is_refused_by_name_and_the_rest_keep_the_generic_refusal() {
    let reach = crate::WordsReach::of_a_hosted_session().the_operator().without(&["close_session"], "is off: Settings").until_said(&["start_another_session"], "waits for him");
    assert!(!reach.may("close_session") && !reach.listed("close_session"));
    assert_eq!(reach.refusing("close_session"), "`close_session` is off: Settings");
    assert!(reach.listed("start_another_session") && !reach.may("start_another_session"));
    assert_eq!(reach.refusing("start_another_session"), "`start_another_session` waits for him");
    assert!(reach.refusing("all_done").starts_with("`all_done` is not the Operator's to call"));
}
