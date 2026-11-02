//! The claim, written before the code: **while an Autopilot runs, a kind of decision the person
//! checked is answered for them, a kind they did not waits, and a destructive one always waits**,
//! whatever is checked. Every answer goes on record. The call is the fake and the process is the
//! stand-in: what runs is Fleet's sort, its reading of the answer, the Job's message and the store.

use std::collections::BTreeSet;
use std::sync::Arc;

use api::{Autopilots, HostedSessions, Sessions};
use ipc::{AutopilotDecision, AutopilotSettings, AutopilotWork, SessionRow, SessionVoice};
use store::KeptWaiting;
use testkit::FakeJudge;

use super::planted::Held;
use super::session_host::{eventually, rig_fitted, Rig};

pub(super) const OLD: &str = "2000-01-01T00:00:00.000Z";
pub(super) const EVERY_BOX: [AutopilotDecision; 10] = [
    AutopilotDecision::Recommended,
    AutopilotDecision::Open,
    AutopilotDecision::Plan,
    AutopilotDecision::Draft,
    AutopilotDecision::Walk,
    AutopilotDecision::Land,
    AutopilotDecision::Unsure,
    AutopilotDecision::Red,
    AutopilotDecision::Reads,
    AutopilotDecision::Writes,
];
pub(super) const SURE_OF_THE_FIRST: &str = "choice: 1\nsure: yes\nbecause: it is the one the agent marked";

pub(super) fn item(id: &str, text: &str, options: &[&str]) -> KeptWaiting {
    KeptWaiting {
        item_id: id.into(),
        text: text.into(),
        since: OLD.into(),
        act: None,
        options: options.iter().map(|one| (*one).into()).collect(),
        steps: Vec::new(),
    }
}

/// A rig whose call answers `answer`, and the call itself, so a test can say what it was asked.
pub(super) fn rig_calling(answer: &str) -> (Rig, Arc<FakeJudge>) {
    let (rig, judge, _) = rig_calling_late(answer);
    (rig, judge)
}

/// As [`rig_calling`], with the clock a test can move on, for an item that is stamped when it is
/// raised and so has not waited out the grace.
pub(super) fn rig_calling_late(answer: &str) -> (Rig, Arc<FakeJudge>, Arc<Held>) {
    let judge = Arc::new(FakeJudge::saying(answer));
    let (held, clock) = (Arc::clone(&judge), Arc::new(Held::started()));
    let ticking = Arc::clone(&clock);
    let rig = rig_fitted(move |fittings| {
        fittings.judge = held;
        fittings.clock = ticking;
    });
    (rig, judge, clock)
}

pub(super) async fn started(rig: &Rig, decisions: &[AutopilotDecision]) {
    let settings = AutopilotSettings {
        instruction: "Fix what is open".into(),
        at_once: 1,
        until: None,
        spend_cap_micros: None,
        session_cap: None,
        on_autopilot: AutopilotWork { backlog: false, running: true },
        decisions: decisions.iter().copied().collect::<BTreeSet<_>>(),
    };
    rig.fleet.start_autopilot(rig.manifest.clone(), settings).await.expect("started");
}

pub(super) async fn keep(rig: &Rig, id: &ipc::SessionId, items: &[KeptWaiting]) {
    rig.fleet.store().lock().await.replace_waiting_for(id.as_str(), items).unwrap();
}

pub(super) async fn waiting(rig: &Rig, id: &ipc::SessionId) -> Vec<String> {
    let read = Arc::clone(&rig.fleet).get_session(id.clone()).await.unwrap();
    read.session.waiting_for.into_iter().map(|one| one.id).collect()
}

pub(super) async fn told(rig: &Rig, id: &ipc::SessionId) -> Vec<String> {
    rig.rows(id)
        .await
        .into_iter()
        .filter_map(|row| match row {
            SessionRow::Message { from: SessionVoice::You, text, .. } => Some(text),
            _ => None,
        })
        .collect()
}

#[tokio::test]
async fn a_checked_kind_is_answered_for_the_person() {
    let (rig, judge) = rig_calling(SURE_OF_THE_FIRST);
    let id = rig.start().await;
    started(&rig, &[AutopilotDecision::Recommended]).await;
    keep(&rig, &id, &[item("a", "Which clock?", &["Frozen (Recommended)", "Fake"])]).await;

    let answered = rig.fleet.decision_pass().await.expect("passed");

    assert_eq!(answered, 1);
    assert_eq!(judge.asked().len(), 1, "one call for the one decision");
    assert!(told(&rig, &id).await.last().unwrap().starts_with("Re: Which clock?\nFrozen (Recommended)"));
    assert!(waiting(&rig, &id).await.is_empty(), "the item left the person's list");
}

#[tokio::test]
async fn an_unchecked_kind_waits_and_costs_no_call() {
    let (rig, judge) = rig_calling(SURE_OF_THE_FIRST);
    let id = rig.start().await;
    started(&rig, &[AutopilotDecision::Recommended]).await;
    let before = told(&rig, &id).await.len();
    keep(&rig, &id, &[item("a", "Which clock?", &["Fake", "Frozen"])]).await;

    let answered = rig.fleet.decision_pass().await.expect("passed");

    assert_eq!(answered, 0);
    assert!(judge.asked().is_empty(), "an open question is not checked, so nothing is asked");
    assert_eq!(told(&rig, &id).await.len(), before, "the Job was told nothing");
    assert_eq!(waiting(&rig, &id).await, ["a"]);
}

#[tokio::test]
async fn a_destructive_one_waits_with_every_box_checked() {
    let (rig, judge) = rig_calling(SURE_OF_THE_FIRST);
    let id = rig.start().await;
    started(&rig, &EVERY_BOX).await;
    let before = told(&rig, &id).await.len();
    keep(
        &rig,
        &id,
        &[
            item("a", "Delete the old branch?", &["Yes (Recommended)", "No"]),
            item("b", "Which one?", &["Keep", "Force push"]),
            item("c", "Run rm -rf build?", &[]),
        ],
    )
    .await;

    let answered = rig.fleet.decision_pass().await.expect("passed");

    assert_eq!(answered, 0);
    assert!(judge.asked().is_empty(), "destructive is decided before any call");
    assert_eq!(told(&rig, &id).await.len(), before);
    assert_eq!(waiting(&rig, &id).await, ["a", "b", "c"]);
}

// ---- What the helper says, and what it does not ----

fn a_question() -> [KeptWaiting; 1] {
    [item("a", "Which clock?", &["Frozen (Recommended)", "Fake"])]
}

#[tokio::test]
async fn a_helper_that_is_unsure_leaves_the_item_and_is_not_asked_again() {
    let (rig, judge) = rig_calling("choice: 1\nsure: no\nbecause: the instruction says nothing of clocks");
    let id = rig.start().await;
    started(&rig, &[AutopilotDecision::Recommended]).await;
    keep(&rig, &id, &a_question()).await;

    assert_eq!(rig.fleet.decision_pass().await.expect("passed"), 0);
    assert_eq!(rig.fleet.decision_pass().await.expect("passed"), 0);

    assert_eq!(judge.asked().len(), 1, "an item already left is not asked about again");
    assert_eq!(waiting(&rig, &id).await, ["a"]);

    keep(&rig, &id, &[item("a", "Which clock?", &["Frozen (Recommended)", "Fake", "System"])]).await;
    rig.fleet.decision_pass().await.expect("passed");
    assert_eq!(judge.asked().len(), 2, "it is asked again once the item changes");
}

#[tokio::test]
async fn an_answer_naming_a_way_that_was_not_offered_leaves_the_item() {
    let (rig, judge) = rig_calling("choice: 7\nsure: yes\nbecause: whichever");
    let id = rig.start().await;
    started(&rig, &[AutopilotDecision::Recommended]).await;
    keep(&rig, &id, &a_question()).await;

    assert_eq!(rig.fleet.decision_pass().await.expect("passed"), 0);
    rig.fleet.decision_pass().await.expect("passed");

    assert_eq!(judge.asked().len(), 1);
    assert_eq!(waiting(&rig, &id).await, ["a"]);
}

#[tokio::test]
async fn a_failed_call_leaves_the_item_and_rests_before_asking_again() {
    let judge = Arc::new(FakeJudge::that_fails("the network"));
    let held = Arc::clone(&judge);
    let rig = rig_fitted(move |fittings| fittings.judge = held);
    let id = rig.start().await;
    started(&rig, &[AutopilotDecision::Recommended]).await;
    keep(&rig, &id, &a_question()).await;

    assert_eq!(rig.fleet.decision_pass().await.expect("passed"), 0);
    let asked = judge.asked().len();
    rig.fleet.decision_pass().await.expect("passed");

    assert_eq!(asked, 1);
    assert_eq!(judge.asked().len(), asked, "the next pass falls in the rest");
    assert_eq!(waiting(&rig, &id).await, ["a"]);
}

#[tokio::test]
async fn an_item_that_has_not_waited_out_the_grace_is_left_for_the_agent() {
    let (rig, judge) = rig_calling(SURE_OF_THE_FIRST);
    let id = rig.start().await;
    started(&rig, &[AutopilotDecision::Recommended]).await;
    let mut fresh = a_question();
    fresh[0].since = rig.fleet.now().as_str().to_string();
    keep(&rig, &id, &fresh).await;

    assert_eq!(rig.fleet.decision_pass().await.expect("passed"), 0);

    assert!(judge.asked().is_empty());
}

#[tokio::test]
async fn a_paused_autopilot_answers_nothing() {
    let (rig, judge) = rig_calling(SURE_OF_THE_FIRST);
    let id = rig.start().await;
    started(&rig, &[AutopilotDecision::Recommended]).await;
    keep(&rig, &id, &a_question()).await;
    rig.fleet.pause_autopilot(rig.manifest.clone()).await.expect("paused");

    assert_eq!(rig.fleet.decision_pass().await.expect("passed"), 0);
    assert!(judge.asked().is_empty());

    rig.fleet.stop_autopilot(rig.manifest.clone()).await.expect("stopped");
    assert_eq!(rig.fleet.decision_pass().await.expect("passed"), 0);
    assert_eq!(waiting(&rig, &id).await, ["a"]);
}

#[tokio::test]
async fn a_job_in_a_terminal_always_waits() {
    let (rig, judge) = rig_calling(SURE_OF_THE_FIRST);
    started(&rig, &EVERY_BOX).await;
    let id = ipc::SessionId::carried("terminal-1");
    rig.fleet
        .report_session(ipc::SessionReport {
            harness: adapters::HOSTED_HARNESS.into(),
            session_id: id.clone(),
            fact: ipc::SessionFact::Started {
                cwd: rig.root.clone(),
                title: None,
                origin: ipc::SessionOrigin::Terminal,
                mod_version: None,
            },
        })
        .await
        .expect("reported");
    keep(&rig, &id, &a_question()).await;

    assert_eq!(rig.fleet.decision_pass().await.expect("passed"), 0);

    assert!(judge.asked().is_empty(), "only a Job Fleet hosts can be answered");
    assert_eq!(waiting(&rig, &id).await, ["a"], "and the item is the person's still");
}

// ---- Reading what the helper says ----

#[test]
fn an_answer_is_read_by_field_name_and_by_the_ways_it_was_offered() {
    use crate::autopilot::deciding::{read_decision, NotDecided, Reading};
    let read = |text: &str| read_decision(text, 2);
    assert_eq!(read("choice: 2\nsure: yes\nbecause: cheaper"), Ok(Reading::Answer { way: 1, because: "cheaper".into() }));
    assert_eq!(read("choice: none\nsure: yes\nbecause: unclear"), Ok(Reading::Leave { because: "unclear".into() }));
    assert_eq!(read("choice: 1\nsure: no\nbecause: unclear"), Ok(Reading::Leave { because: "unclear".into() }));
    assert_eq!(read("choice: 1\nsure: yes"), Ok(Reading::Answer { way: 0, because: "it gave no reason".into() }));
    for bad in ["sure: yes", "choice: 0\nsure: yes", "choice: 3\nsure: yes", "choice: one\nsure: yes", "choice: 1"] {
        assert!(matches!(read(bad), Err(NotDecided::Unreadable { .. })), "{bad}");
    }
}

#[test]
fn a_permission_is_a_read_or_a_write_and_only_where_fleet_can_read_all_of_it() {
    use crate::autopilot::sorting::sorted;
    let card = |tool: &str, detail: &str| ipc::HelmCallInFlight {
        call: "c1".into(),
        manifest_id: ipc::ManifestId::carried("m"),
        asked_at: ipc::Instant::carried(OLD),
        tool: tool.into(),
        detail: detail.into(),
        truncated: false,
        length: None,
        whole: None,
        rule: String::new(),
        offers: vec![ipc::HelmCallAnswer::AllowOnce],
        holding_for_seconds: 60,
        questions: Vec::new(),
        reaches: None,
        reaches_to: None,
        kind: None,
    };
    let item = |card: &ipc::HelmCallInFlight| ipc::WaitingItem {
        id: "perm:c1".into(),
        text: format!("{} {}", card.tool, card.detail),
        since: ipc::Instant::carried(OLD),
        source: ipc::WaitingSource::Permission,
        act: None,
        options: Vec::new(),
        steps: Vec::new(),
        place: None,
        confirmed: false,
    };
    let kind = |card: ipc::HelmCallInFlight| sorted(&item(&card), Some(&card));
    assert_eq!(kind(card("Read", "src/lib.rs")), Some(AutopilotDecision::Reads));
    assert_eq!(kind(card("Edit", "src/lib.rs")), Some(AutopilotDecision::Writes));
    assert_eq!(kind(card("Bash", "cargo build")), Some(AutopilotDecision::Writes), "a run command is a write");
    assert_eq!(kind(card("Bash", "")), None, "a command Fleet cannot read waits");
    assert_eq!(kind(ipc::HelmCallInFlight { truncated: true, ..card("Bash", "cargo build -p") }), None);
    assert_eq!(kind(card("Bash", "rm -rf target")), None, "destructive waits");
    assert_eq!(kind(card("mcp__elsewhere__post", "x")), None, "a tool nobody named waits");
    assert_eq!(kind(ipc::HelmCallInFlight { reaches: Some("/etc".into()), ..card("Read", "/etc/hosts") }), None);
}

#[test]
fn the_question_carries_the_decision_its_ways_and_how_this_repository_was_answered_before() {
    use crate::autopilot::deciding::{question, Deciding};
    let deciding = Deciding {
        kind: AutopilotDecision::Open,
        subject: "Which clock?".into(),
        detail: None,
        ways: vec!["Frozen".into(), "Fake".into()],
    };
    let earlier = store::KeptDecision {
        id: "s:a".into(),
        manifest_id: "m".into(),
        session_id: None,
        who: String::new(),
        kind: "recommended".into(),
        asked: "Which port?".into(),
        chose: "8080".into(),
        because: "the default".into(),
        at: OLD.into(),
        reviewed: true,
        corrected: Some("Use 9090".into()),
    };
    let asked = question(&crate::prompts::Prompts::shipped(), " Fix what is open ", &deciding, &[earlier]);
    for fact in ["Fix what is open", "Which clock?", "1. Frozen", "2. Fake", "Which port?", "chose: 8080", "corrected: Use 9090"] {
        assert!(asked.contains(fact), "{fact}\n{asked}");
    }
    assert!(!asked.contains('{'), "every placeholder was filled\n{asked}");
    let alone = question(&crate::prompts::Prompts::shipped(), "x", &deciding, &[]);
    assert!(alone.contains("none yet"));
}

// ---- The kinds that are not a question ----

const APPROVE: &str = "choice: 1\nsure: yes\nbecause: it matches what was asked";

async fn the_plan_waits(rig: &Rig, id: &ipc::SessionId) {
    rig.fleet.record_plan(None, super::session_rail::plan(id), false).await.expect("recorded");
    assert_eq!(waiting(rig, id).await, ["plan:1"]);
}

#[tokio::test]
async fn a_plan_is_approved_when_plan_is_checked_and_waits_when_it_is_not() {
    let (rig, judge, clock) = rig_calling_late(APPROVE);
    let id = super::session_plan::running(&rig).await;
    started(&rig, &[AutopilotDecision::Draft]).await;
    the_plan_waits(&rig, &id).await;
    clock.on(300);
    let before = judge.asked().len();

    assert_eq!(rig.fleet.decision_pass().await.expect("passed"), 0);
    assert_eq!(judge.asked().len(), before, "plan is not checked, so nothing is asked");

    rig.fleet.stop_autopilot(rig.manifest.clone()).await.expect("stopped");
    started(&rig, &[AutopilotDecision::Plan]).await;
    assert_eq!(rig.fleet.decision_pass().await.expect("passed"), 1);

    assert!(judge.asked().last().unwrap().contains("Reader"), "the helper reads the plan's groups");
    assert!(waiting(&rig, &id).await.is_empty(), "the plan is approved");
    eventually(|| async { super::session_plan::told(&rig, "approved plan v1") == 1 }).await;
}

#[tokio::test]
async fn a_draft_is_approved_as_written() {
    let (rig, judge, clock) = rig_calling_late(APPROVE);
    let id = rig.start().await;
    started(&rig, &[AutopilotDecision::Draft]).await;
    rig.fleet
        .propose_draft(
            None,
            ipc::ProposeDraft {
                title: "Description for #41".into(),
                text: "Reads the draft row back.".into(),
                before: None,
                session_id: Some(id.clone()),
            },
        )
        .await
        .expect("proposed");
    clock.on(300);

    assert_eq!(rig.fleet.decision_pass().await.expect("passed"), 1);

    assert!(judge.asked()[0].contains("Reads the draft row back."), "the helper reads the draft's text");
    assert!(told(&rig, &id).await.last().unwrap().starts_with("Approved: Description for #41"));
    assert!(waiting(&rig, &id).await.is_empty());
}

#[tokio::test]
async fn a_walk_is_approved_the_way_his_press_approves_it() {
    let (rig, _, clock) = rig_calling_late(APPROVE);
    let id = rig.start().await;
    started(&rig, &[AutopilotDecision::Walk]).await;
    let page = "http://localhost:5173/?walk=findings";
    rig.fleet
        .show_window(None, ipc::ShowWindow { url: page.into(), title: Some("Findings".into()), session_id: Some(id.clone()), ..Default::default() })
        .await
        .expect("shown");
    clock.on(300);

    assert_eq!(rig.fleet.decision_pass().await.expect("passed"), 1);

    assert_eq!(told(&rig, &id).await.last().unwrap(), &format!("Approved: {page}"));
    assert!(waiting(&rig, &id).await.is_empty());
}

#[test]
fn everything_but_the_kinds_waits_whatever_is_checked() {
    use crate::autopilot::sorting::sorted;
    for source in [
        ipc::WaitingSource::WindowAsk,
        ipc::WaitingSource::Replied,
        ipc::WaitingSource::Done,
        ipc::WaitingSource::FirstMessage,
        ipc::WaitingSource::Delivery,
        ipc::WaitingSource::Sketch,
        ipc::WaitingSource::RuleAct,
    ] {
        let one = ipc::WaitingItem {
            id: "a".into(),
            text: "Which colour?".into(),
            since: ipc::Instant::carried(OLD),
            source,
            act: None,
            options: vec![ipc::WaitingOption { label: "Blue (Recommended)".into(), description: None }],
            steps: Vec::new(),
            place: None,
            confirmed: false,
        };
        assert_eq!(sorted(&one, None), None, "{source:?}");
    }
    let errand = ipc::WaitingItem {
        id: "a".into(),
        text: "Merge once ci is green".into(),
        since: ipc::Instant::carried(OLD),
        source: ipc::WaitingSource::Agent,
        act: Some(ipc::WaitingAct { kind: ipc::WaitingActKind::Run, target: "gh pr merge 1".into() }),
        options: vec![ipc::WaitingOption { label: "Go (Recommended)".into(), description: None }],
        steps: Vec::new(),
        place: None,
        confirmed: false,
    };
    assert_eq!(sorted(&errand, None), None, "an errand always waits");
}

/// A permission card held open for `tool`, by a Job on Ask mode.
async fn permission_asked(rig: &Rig, id: &ipc::SessionId, tool: &str, input: &str) -> tokio::task::JoinHandle<()> {
    rig.fleet
        .tune_session(ipc::TuneSession { session_id: id.clone(), model: None, effort: None, mode: ipc::SessionMode::Ask })
        .await
        .unwrap();
    let (fleet, session) = (Arc::clone(&rig.fleet), id.as_str().to_string());
    let ask = format!(r#"{{"tool_name":"{tool}","input":{input}}}"#);
    let held = tokio::spawn(async move {
        let answer = fleet.session_permission(&session, ipc::decode("an ask", ask.as_bytes()).unwrap()).await;
        assert!(matches!(answer, Ok(ipc::RunOrNot::Allow { .. })), "{answer:?}");
    });
    eventually(|| async {
        Arc::clone(&rig.fleet).get_session(id.clone()).await.unwrap().session.hosted.unwrap().asked.is_some()
    })
    .await;
    held
}

#[tokio::test]
async fn a_run_command_is_allowed_by_writes_and_a_read_by_reads_and_neither_by_the_other() {
    let (rig, judge, clock) = rig_calling_late("choice: 1\nsure: yes\nbecause: the build is what was asked");
    let id = rig.start().await;
    started(&rig, &[AutopilotDecision::Reads]).await;
    let held = permission_asked(&rig, &id, "Bash", r#"{"command":"cargo build"}"#).await;
    clock.on(300);

    assert_eq!(rig.fleet.decision_pass().await.expect("passed"), 0, "a run command is a write, and writes is not checked");
    assert!(judge.asked().is_empty() && !held.is_finished());

    rig.fleet.stop_autopilot(rig.manifest.clone()).await.expect("stopped");
    started(&rig, &[AutopilotDecision::Writes]).await;
    assert_eq!(rig.fleet.decision_pass().await.expect("passed"), 1);
    tokio::time::timeout(std::time::Duration::from_secs(5), held).await.expect("allowed").expect("joined");
}
