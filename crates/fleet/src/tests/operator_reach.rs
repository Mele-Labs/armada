//! The claim, written before the code: **Fleet enforces what a project sets for its Operator.** A tool switched off is gone
//! from its reach and refused by the switch's name, at the door and at the gate; a level of acts holds what it names until
//! he has said so; a project that asks its decisions in the talk is refused cards; and a call that names another project is
//! held and put to him on a card Fleet answers itself. `docs/practices/rust.md`, *a narrow type where the wrong call is
//! unspeakable*: the refusals are the door's and the gate's, never the prompt's.

use std::sync::Arc;

use api::{Admitting, Caller, HostedSessions, Refusal};
use ipc::door::Call;
use ipc::operator_page::{OperatorAnswer, OperatorAnswerPress};
use ipc::operator_talk::TalkToOperator;
use ipc::{CardOption, GateAnswer, ManifestId, OperatorActs, OperatorCardKind, OperatorDecisions, OperatorSettingsSet, OperatorToolsSet, RaiseCard, SessionGate};

use super::operator_projects::project_named;
use super::operator_projects_rig::{caller_on, heard_by, operator_of, projects_rig_placed, set_for};
use super::operator_session::switched;
use super::session_host::{eventually, Rig};
use crate::operator::{CHAT_MODE, FLEET_CARD};

/// Two projects with an Operator running each, and a caller placed on each Operator's process.
struct Two {
    rig: Rig,
    second: ManifestId,
    a: String,
    b: String,
    for_a: String,
    for_b: String,
    caller_a: Caller,
    caller_b: Caller,
    _home: super::tmp::TempDir,
}

async fn two() -> Two {
    let (rig, second, home, peers) = projects_rig_placed();
    let (a, b) = (project_named(&rig, "Alpha", &rig.manifest).await, project_named(&rig, "Beta", &second).await);
    switched(&rig, true).await;
    rig.fleet.operator_session_kept().await;
    let (for_a, for_b) = (operator_of(&rig, &a).await.expect("A's"), operator_of(&rig, &b).await.expect("B's"));
    eventually(|| async { rig.stand_in.process_of(&for_a).is_some() && rig.stand_in.process_of(&for_b).is_some() }).await;
    let (caller_a, caller_b) = (caller_on(&rig, &peers, &for_a, 52_100), caller_on(&rig, &peers, &for_b, 52_101));
    Two { rig, second, a, b, for_a, for_b, caller_a, caller_b, _home: home }
}

fn tools(set: impl FnOnce(&mut OperatorToolsSet)) -> OperatorSettingsSet {
    let mut tools = OperatorToolsSet::default();
    set(&mut tools);
    OperatorSettingsSet { tools: Some(tools), ..Default::default() }
}

fn acts(level: OperatorActs) -> OperatorSettingsSet {
    OperatorSettingsSet { acts: Some(level), ..Default::default() }
}

/// What the door holds back from `caller` for `operation`, by its settings.
async fn held_back(two: &Two, caller: Caller, operation: &str) -> Option<String> {
    two.rig.fleet.words_reach_at(caller).await.expect("placed on a session").held_back(operation)
}

async fn says(two: &Two, project: &str, words: &str) {
    let talk = TalkToOperator { words: words.into(), about: None, attachments: Vec::new(), mentions: Vec::new(), project: Some(project.into()), looking_at: None };
    Arc::clone(&two.rig.fleet).operator_talked(talk).await.expect("delivered");
}

fn gate(session: &str, tool: &str, input: &str) -> SessionGate {
    ipc::decode("a gate", format!(r#"{{"session_id":"{session}","cwd":"/work","tool_name":"{tool}","tool_input":{input}}}"#).as_bytes()).expect("a gate")
}

/// What a gate answer reads, as text.
fn said(answer: &GateAnswer) -> String {
    format!("{answer:?}")
}

fn refused_code(why: Refusal) -> String {
    why.error().code.to_string()
}

#[tokio::test]
async fn each_tool_switched_off_is_refused_at_the_door_by_its_switch_and_allowed_when_on() {
    let two = two().await;
    let rows: [(&str, &[&str], fn(&mut OperatorToolsSet, bool)); 3] = [
        ("Start Jobs", &["start_another_session", "dispatch_job"], |tools, on| tools.start_jobs = Some(on)),
        ("End Jobs", &["close_session"], |tools, on| tools.end_jobs = Some(on)),
        ("Merge pull requests", &["merge_pull_request_by_number", "enable_auto_merge", "set_session_auto_land"], |tools, on| tools.merge = Some(on)),
    ];
    for (switch, operations, set) in rows {
        for operation in operations {
            assert_eq!(held_back(&two, two.caller_a, operation).await, None, "`{operation}` is on by default");
        }
        set_for(&two.rig, &two.a, tools(|tools| set(tools, false))).await;
        for operation in operations {
            let why = held_back(&two, two.caller_a, operation).await.unwrap_or_else(|| panic!("`{operation}` is refused while {switch} is off"));
            assert!(why.starts_with(&format!("`{operation}` is off for this project's Operator")) && why.ends_with(&format!("Settings, Operator, {switch}")), "{why}");
            assert_eq!(held_back(&two, two.caller_b, operation).await, None, "another project's Operator keeps `{operation}`");
        }
        set_for(&two.rig, &two.a, tools(|tools| set(tools, true))).await;
        for operation in operations {
            assert_eq!(held_back(&two, two.caller_a, operation).await, None, "`{operation}` is allowed again when {switch} is on");
        }
    }
    let reach = two.rig.fleet.words_reach_at(two.caller_a).await.expect("placed");
    assert!(reach.is_the_operator());
}

#[tokio::test]
async fn ask_before_starting_refuses_a_start_until_he_presses_a_card_and_only_for_fifteen_minutes() {
    let two = two().await;
    set_for(&two.rig, &two.a, acts(OperatorActs::AskBeforeStarting)).await;
    let refused = held_back(&two, two.caller_a, "start_another_session").await.expect("held");
    assert!(refused.contains("ask_before_starting") && refused.contains("on a card or in the talk"), "{refused}");
    assert_eq!(held_back(&two, two.caller_a, "close_session").await, None, "only a start is held at this level");
    assert_eq!(held_back(&two, two.caller_b, "start_another_session").await, None, "another project's level is its own");

    // His press on one of this Operator's cards is a say.
    let card = Arc::clone(&two.rig.fleet)
        .card_raised(Some(two.caller_a), RaiseCard { kind: OperatorCardKind::Raise, title: "Start the fix?".into(), read: "A bug.".into(), options: vec![CardOption { label: "Start it".into(), cost: "A Job".into() }, CardOption { label: "Leave it".into(), cost: "A bug stays".into() }], sessions: Vec::new(), repository: None, talk_of: None })
        .await
        .expect("raised")
        .card;
    let press = OperatorAnswerPress { id: card, answer: OperatorAnswer::Yes, project: None, then: None, option: Some("Start it".into()) };
    two.rig.fleet.operator_answered(press).await.expect("pressed");
    assert_eq!(held_back(&two, two.caller_a, "start_another_session").await, None, "open within fifteen minutes of his press");
    assert!(held_back(&two, two.caller_b, "start_another_session").await.is_none());

    let now = two.rig.fleet.now().epoch_millis().expect("a time");
    two.rig.fleet.operator_seat().said_at(&two.a, now - 16 * 60_000);
    assert!(held_back(&two, two.caller_a, "start_another_session").await.is_some(), "closed again after fifteen minutes");
}

#[tokio::test]
async fn only_when_told_refuses_every_tool_act_until_his_words_arrive_and_act_then_tell_refuses_none() {
    let two = two().await;
    set_for(&two.rig, &two.a, acts(OperatorActs::OnlyWhenTold)).await;
    for operation in ["start_another_session", "close_session", "merge_pull_request_by_number", "enable_auto_merge"] {
        let why = held_back(&two, two.caller_a, operation).await.unwrap_or_else(|| panic!("`{operation}` is held"));
        assert!(why.contains("only_when_told") && why.contains("on a card or in the talk"), "{why}");
    }
    let alpha_session = gate(&two.for_a, "Bash", r#"{"command":"gh pr merge 5"}"#);
    assert!(said(&Arc::clone(&two.rig.fleet).gated(alpha_session.clone()).await).contains("only_when_told"), "the gate holds a merge too");

    says(&two, &two.a, "Go ahead and merge it.").await;
    for operation in ["start_another_session", "close_session", "merge_pull_request_by_number"] {
        assert_eq!(held_back(&two, two.caller_a, operation).await, None, "`{operation}` is open after his words");
    }
    assert_eq!(Arc::clone(&two.rig.fleet).operator_gated(&alpha_session, &two.for_a, &two.a).await, None, "and so is a merge at the gate");

    set_for(&two.rig, &two.b, acts(OperatorActs::ActThenTell)).await;
    assert_eq!(held_back(&two, two.caller_b, "close_session").await, None, "act_then_tell holds nothing");
}

#[tokio::test]
async fn the_gate_refuses_a_merge_and_a_new_issue_by_the_switch_and_allows_them_when_on() {
    let two = two().await;
    let fleet = Arc::clone(&two.rig.fleet);
    let merges = ["gh pr merge 41 --squash", "gh api -X PUT repos/o/r/pulls/41/merge"];
    let files = ["gh issue create --title t --body b", "bd create \"a bug\"", "bd new \"a bug\""];
    set_for(&two.rig, &two.a, tools(|tools| (tools.merge, tools.file_issues) = (Some(false), Some(false)))).await;
    for (command, switch) in merges.iter().map(|one| (one, "Merge pull requests")).chain(files.iter().map(|one| (one, "File issues"))) {
        let answer = fleet.gated(gate(&two.for_a, "Bash", &format!("{{\"command\":{command:?}}}"))).await;
        assert!(said(&answer).contains(&format!("Settings, Operator, {switch}")), "`{command}` is refused by {switch}: {}", said(&answer));
        let other = fleet.operator_gated(&gate(&two.for_b, "Bash", &format!("{{\"command\":{command:?}}}")), &two.for_b, &two.b).await;
        assert_eq!(other, None, "`{command}` is B's own to run");
    }
    let reads = fleet.operator_gated(&gate(&two.for_a, "Bash", r#"{"command":"gh pr view 41 && gh issue list && bd list"}"#), &two.for_a, &two.a).await;
    assert_eq!(reads, None, "a read is not refused");

    set_for(&two.rig, &two.a, tools(|tools| (tools.merge, tools.file_issues) = (Some(true), Some(true)))).await;
    for command in merges.iter().chain(files.iter()) {
        let answer = fleet.operator_gated(&gate(&two.for_a, "Bash", &format!("{{\"command\":{command:?}}}")), &two.for_a, &two.a).await;
        assert_eq!(answer, None, "`{command}` is allowed when its switch is on");
    }
}

#[tokio::test]
async fn a_project_that_asks_in_the_talk_is_refused_raise_card_and_one_that_asks_on_cards_is_not() {
    let two = two().await;
    let card = || RaiseCard { kind: OperatorCardKind::Deal, title: "Which fix?".into(), read: "Two ways.".into(), options: vec![CardOption { label: "A".into(), cost: "slow".into() }, CardOption { label: "B".into(), cost: "risky".into() }], sessions: Vec::new(), repository: None, talk_of: None };
    set_for(&two.rig, &two.a, OperatorSettingsSet { decisions: Some(OperatorDecisions::Chat), ..Default::default() }).await;
    let refused = Arc::clone(&two.rig.fleet).card_raised(Some(two.caller_a), card()).await.expect_err("chat mode");
    assert_eq!(refused_code(refused), CHAT_MODE);
    assert!(Arc::clone(&two.rig.fleet).card_raised(Some(two.caller_b), card()).await.is_ok(), "B asks on cards still");

    set_for(&two.rig, &two.a, OperatorSettingsSet { decisions: Some(OperatorDecisions::Cards), ..Default::default() }).await;
    assert!(Arc::clone(&two.rig.fleet).card_raised(Some(two.caller_a), card()).await.is_ok(), "cards again");
}

fn call(operation: &'static str, method: &'static str, path: String, body: Option<String>) -> Call {
    Call { operation, method, path, body, manifest_id: None, drone_id: None }
}

fn issues_of(manifest: &ManifestId) -> Call {
    call("list_issues", "GET", format!("/issues/{}", manifest.as_str()), None)
}

fn merge_in(manifest: &ManifestId, number: u32) -> Call {
    call("merge_pull_request_by_number", "POST", format!("/pull_requests/{}/{number}/merge", manifest.as_str()), Some("{}".into()))
}

/// The cards of Fleet's own that stand, as their rows.
async fn fleet_cards(two: &Two) -> Vec<store::KeptTurn> {
    two.rig.fleet.store().lock().await.operator_turn_open().expect("read").into_iter().filter(|row| row.item.starts_with(FLEET_CARD)).collect()
}

#[tokio::test]
async fn a_call_naming_another_project_is_held_with_a_card_and_one_inside_its_project_is_not() {
    let two = two().await;
    assert_eq!(two.rig.fleet.call_held(two.caller_a, &issues_of(&two.rig.manifest)).await, None, "its own repository");
    let held = two.rig.fleet.call_held(two.caller_a, &issues_of(&two.second)).await.expect("held");
    assert!(held.starts_with("fleet.operator_other_project: held: he is asked"), "{held}");

    let cards = fleet_cards(&two).await;
    assert_eq!(cards.len(), 1, "one card to him");
    assert_eq!(cards[0].project.as_deref(), Some(two.a.as_str()), "it carries the asking Operator's project");
    let item = ipc::operator_page::item_read(&cards[0].body).expect("an item");
    assert_eq!(item.text, "Operator (Alpha) wants to read issues in Beta");
    let labels: Vec<_> = item.options.iter().map(|option| option.label.as_str()).collect();
    assert_eq!(labels, ["Let it through", "Hand it to Beta's Operator", "Refuse"]);

    assert!(two.rig.fleet.call_held(two.caller_a, &issues_of(&two.second)).await.is_some(), "asked again, still held");
    assert_eq!(fleet_cards(&two).await.len(), 1, "and the card he has not answered is not raised twice");
}

#[tokio::test]
async fn a_repository_two_projects_hold_is_inside_both_and_is_not_held() {
    let (rig, second, home, peers) = projects_rig_placed();
    let a = project_named(&rig, "Alpha", &rig.manifest).await;
    // Beta holds both repositories.
    let beta = api::Projects::add_project(
        rig.fleet.as_ref(),
        ipc::NewProject {
            name: "Beta".into(),
            members: Some(vec![ipc::ProjectMember { manifest_id: rig.manifest.clone(), folder: None }, ipc::ProjectMember { manifest_id: second.clone(), folder: None }]),
            place: None,
            values: None,
        },
    )
    .await
    .expect("added")
    .projects
    .into_iter()
    .find(|one| one.name == "Beta")
    .expect("Beta")
    .id;
    switched(&rig, true).await;
    rig.fleet.operator_session_kept().await;
    let (for_a, for_b) = (operator_of(&rig, &a).await.expect("A's"), operator_of(&rig, &beta).await.expect("Beta's"));
    eventually(|| async { rig.stand_in.process_of(&for_a).is_some() && rig.stand_in.process_of(&for_b).is_some() }).await;
    let (caller_a, caller_b) = (caller_on(&rig, &peers, &for_a, 52_110), caller_on(&rig, &peers, &for_b, 52_111));

    assert_eq!(rig.fleet.call_held(caller_a, &issues_of(&rig.manifest)).await, None, "Alpha holds the shared one");
    assert_eq!(rig.fleet.call_held(caller_b, &issues_of(&rig.manifest)).await, None, "so does Beta");
    assert_eq!(rig.fleet.call_held(caller_b, &issues_of(&second)).await, None, "and Beta holds its own");
    assert!(rig.fleet.call_held(caller_a, &issues_of(&second)).await.is_some(), "Alpha does not hold the second");
    drop(home);
}

#[tokio::test]
async fn letting_it_through_grants_that_project_for_an_hour_and_tells_the_operator_to_call_again() {
    let two = two().await;
    let call = merge_in(&two.second, 7);
    assert!(two.rig.fleet.call_held(two.caller_a, &call).await.is_some());
    let card = fleet_cards(&two).await.remove(0);

    let press = OperatorAnswerPress { id: format!("turn:{}", card.n), answer: OperatorAnswer::Yes, project: None, then: None, option: Some("Let it through".into()) };
    two.rig.fleet.operator_answered(press).await.expect("pressed");

    assert_eq!(two.rig.fleet.call_held(two.caller_a, &call).await, None, "the call goes through");
    eventually(|| async { heard_by(&two.rig, &two.for_a, "Call it again") >= 1 }).await;
    assert!(fleet_cards(&two).await.is_empty(), "the card is settled");
    assert!(two.rig.fleet.call_held(two.caller_b, &issues_of(&two.rig.manifest)).await.is_some(), "the grant is Alpha's to Beta and not Beta's to Alpha");

    let now = two.rig.fleet.now().epoch_millis().expect("a time");
    assert!(two.rig.fleet.operator_seat().grant_open(&two.a, &two.b, now + 59 * 60_000), "open for the hour");
    assert!(!two.rig.fleet.operator_seat().grant_open(&two.a, &two.b, now + 61 * 60_000), "and not after it");
}

#[tokio::test]
async fn handing_it_over_tells_the_other_projects_operator_what_was_asked_in_the_words_it_used() {
    let two = two().await;
    assert!(two.rig.fleet.call_held(two.caller_a, &merge_in(&two.second, 7)).await.is_some());
    let card = fleet_cards(&two).await.remove(0);

    let press = OperatorAnswerPress { id: format!("turn:{}", card.n), answer: OperatorAnswer::Yes, project: None, then: None, option: Some("Hand it to Beta's Operator".into()) };
    two.rig.fleet.operator_answered(press).await.expect("pressed");

    eventually(|| async { heard_by(&two.rig, &two.for_b, "merge pull request #7") >= 1 }).await;
    let told_b = two.rig.stand_in.process_of(&two.for_b).expect("running").sent.lock().unwrap().join("\n");
    assert!(told_b.contains("Operator, the Operator of Alpha, asked to merge a pull request in Beta") && told_b.contains("It is yours to decide"), "{told_b}");
    eventually(|| async { heard_by(&two.rig, &two.for_a, "He handed `merge_pull_request_by_number` to Beta's Operator") >= 1 }).await;
    assert!(two.rig.fleet.call_held(two.caller_a, &merge_in(&two.second, 7)).await.is_some(), "no grant: the call is held again");
}

#[tokio::test]
async fn refusing_it_tells_the_asking_operator_and_grants_nothing() {
    let two = two().await;
    assert!(two.rig.fleet.call_held(two.caller_a, &issues_of(&two.second)).await.is_some());
    let card = fleet_cards(&two).await.remove(0);

    let press = OperatorAnswerPress { id: format!("turn:{}", card.n), answer: OperatorAnswer::Yes, project: None, then: None, option: Some("Refuse".into()) };
    two.rig.fleet.operator_answered(press).await.expect("pressed");

    eventually(|| async { heard_by(&two.rig, &two.for_a, "He refused `list_issues` in Beta") >= 1 }).await;
    assert_eq!(heard_by(&two.rig, &two.for_b, "He refused"), 0, "Beta's Operator is told nothing");
    assert!(two.rig.fleet.call_held(two.caller_a, &issues_of(&two.second)).await.is_some(), "still held");
}

#[tokio::test]
async fn a_job_of_another_project_is_held_and_so_is_a_file_under_its_repository_at_the_gate() {
    let two = two().await;
    let start = ipc::StartSession {
        project: Some(two.b.clone()),
        manifest_id: two.second.clone(),
        title: Some("Beta's Job".into()),
        model: None,
        effort: None,
        mode: None,
        root: None,
        fork: None,
        request: None,
        after: None,
        held: false,
        project_rows: Vec::new(),
        on_your_words: None,
        workflow: None,
    };
    let job = two.rig.fleet.start_session(start).await.expect("started").id;
    let close = call("close_session", "POST", "/sessions/close".into(), Some(format!(r#"{{"session_id":"{}"}}"#, job.as_str())));
    assert!(two.rig.fleet.call_held(two.caller_a, &close).await.is_some(), "A may not end B's Job");
    assert_eq!(two.rig.fleet.call_held(two.caller_b, &close).await, None, "B may end its own");
    let tell = call("tell_session", "POST", "/operator/tell_session".into(), Some(format!(r#"{{"session_id":"{}","text":"stop"}}"#, job.as_str())));
    assert!(two.rig.fleet.call_held(two.caller_a, &tell).await.is_some(), "nor message it");

    let second_root = two.rig.fleet.repositories().served().into_iter().find(|one| one.manifest().id().as_str() == two.second.as_str()).expect("served").root().to_string();
    let fleet = Arc::clone(&two.rig.fleet);
    for tool in ["Read", "Grep", "Glob", "Edit", "Write", "NotebookEdit"] {
        let key = if tool == "NotebookEdit" { "notebook_path" } else if matches!(tool, "Grep" | "Glob") { "path" } else { "file_path" };
        let input = format!(r#"{{"{key}":"{second_root}/src/lib.rs"}}"#);
        let answer = fleet.operator_gated(&gate(&two.for_a, tool, &input), &two.for_a, &two.a).await.unwrap_or_else(|| panic!("`{tool}` under Beta's repository is held"));
        assert!(said(&answer).contains("fleet.operator_other_project"), "{}", said(&answer));
        let own = fleet.operator_gated(&gate(&two.for_b, tool, &input), &two.for_b, &two.b).await;
        assert_eq!(own, None, "`{tool}` under its own repository is not");
    }
    let mine = format!(r#"{{"file_path":"{}/src/lib.rs"}}"#, two.rig.root);
    assert_eq!(fleet.operator_gated(&gate(&two.for_a, "Read", &mine), &two.for_a, &two.a).await, None, "Alpha reads its own");
    assert!(!fleet_cards(&two).await.is_empty(), "each held file call put a card to him");
}
