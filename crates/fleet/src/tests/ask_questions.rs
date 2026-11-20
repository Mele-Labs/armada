//! The claim, written before the code: **every ask is a question in plain words, and it titles his item.** A sketch,
//! a part or a Job's own item that asks him to choose or approve is refused without one, saying how to word it; what
//! only informs needs none. A plan, a draft and a page are asks Fleet makes, so Fleet words their question.
//! `.claude/decisions/2026-11-19-one-ask-one-item-one-place.md`; `docs/concepts/session/waiting-on-the-person.md`.

use api::{HostedSessions, Refusal, Sessions};
use ipc::{AddPart, SessionId, SetWaitingFor, ShowSketch, WaitingAct, WaitingActKind, WaitingInput, WaitingOption, WaitingSource};

use super::session_host::rig_placing;
use super::session_plan::{first, recorded, running};

const SCENE: &str = r#"{"nodes":[{"id":"a","kind":"box","x":0,"y":0,"title":"Bay"}],"edges":[]}"#;
const TAKE_A: &str = r#"{"nodes":[{"id":"m","kind":"box","x":0,"y":0,"title":"Left"}],"edges":[]}"#;
const TAKE_B: &str = r#"{"nodes":[{"id":"m","kind":"box","x":0,"y":0,"title":"Right"}],"edges":[]}"#;

/// A sketch titled `title`, with `extra` fields such as its question.
fn shown(id: &SessionId, title: &str, extra: &str) -> ShowSketch {
    ipc::decode("a sketch", format!(r#"{{"title":"{title}","session_id":"{}","scene":{SCENE}{extra}}}"#, id.as_str()).as_bytes()).unwrap()
}

fn part(id: &SessionId, body: &str) -> AddPart {
    ipc::decode("a part", format!(r#"{{"session_id":"{}",{body}}}"#, id.as_str()).as_bytes()).unwrap()
}

fn refused<T: std::fmt::Debug>(said: Result<T, Refusal>) -> ipc::WireError {
    match said {
        Err(Refusal::Unacceptable(why)) => why,
        other => panic!("refused, not {other:?}"),
    }
}

/// An agent item `text` with two options, answered by a pick.
fn choice(id: &str, text: &str) -> WaitingInput {
    let option = |label: &str| WaitingOption { label: label.into(), description: None, picture: None };
    WaitingInput {
        id: id.into(),
        text: text.into(),
        act: Some(WaitingAct { kind: WaitingActKind::Answer, target: id.into(), on: None }),
        options: vec![option("Rows"), option("Cards")],
        approves: false,
        steps: Vec::new(),
        artifact: None,
    }
}

#[tokio::test]
async fn a_sketch_that_asks_with_no_question_or_one_not_worded_as_one_is_refused_and_one_that_explains_needs_none() {
    let rig = rig_placing(None);
    let id = rig.start().await;

    let none = refused(rig.fleet.show_sketch(None, shown(&id, "Bay flow", "")).await);
    assert_eq!(none.code, "fleet.sketch_refused");
    assert!(none.message.contains("`question` is missing") && none.message.contains("?`"), "it says how to word one: {}", none.message);
    let statement = refused(rig.fleet.show_sketch(None, shown(&id, "Bay flow", r#","question":"Rows or cards""#)).await);
    assert!(statement.message.contains("is not a question"), "{}", statement.message);
    let marked = refused(rig.fleet.show_sketch(None, shown(&id, "Bay flow", r#","question":"Use `rows`?""#)).await);
    assert!(marked.message.contains("backtick"), "{}", marked.message);

    rig.fleet.show_sketch(None, shown(&id, "Request flow", r#","asks":false"#)).await.expect("a sketch that explains asks nothing");
    rig.fleet.show_sketch(None, shown(&id, "Bay flow", r#","question":"Rows or cards for the bay?""#)).await.unwrap();
    let record = rig.fleet.published_hosted(id.as_str()).await.unwrap();
    let items: Vec<(&str, &str, Option<&str>)> = record.waiting_for.iter().map(|one| (one.id.as_str(), one.text.as_str(), one.artifact.as_deref())).collect();
    assert_eq!(items, [("sketch:Bay flow", "Rows or cards for the bay?", Some("sketch:Bay flow"))], "titled with its question, naming its artifact");
}

#[tokio::test]
async fn a_part_that_asks_needs_its_question_and_the_question_titles_its_item() {
    let rig = rig_placing(None);
    let id = rig.start().await;

    let words = refused(rig.fleet.add_part(None, part(&id, r#""title":"Cache notes","part":{"id":"notes","kind":"words","text":"Evict by age."}"#)).await);
    assert_eq!(words.code, "fleet.part_refused");
    assert!(words.message.contains("`question` is missing"), "{}", words.message);
    rig.fleet
        .add_part(None, part(&id, r#""title":"Cache notes","asks":false,"part":{"id":"notes","kind":"words","text":"Evict by age."}"#))
        .await
        .expect("an aside asks nothing");
    let takes = format!(
        r#""title":"Catalog","part":{{"id":"layout","kind":"takes","question":"Which layout reads better?","takes":[{{"id":"a","scene":{TAKE_A}}},{{"id":"b","scene":{TAKE_B}}}]}}"#
    );
    rig.fleet.add_part(None, part(&id, &takes)).await.unwrap();
    let record = rig.fleet.published_hosted(id.as_str()).await.unwrap();
    let item = record.waiting_for.iter().find(|one| one.id == "sketch:Catalog").expect("the takes wait");
    assert_eq!(item.text, "Which layout reads better?");
    let held = rig.fleet.get_job_artifacts(id.clone()).await.unwrap();
    let layout = held.iter().flat_map(|one| one.parts.iter()).find(|one| one.id == "layout").expect("the takes part");
    assert_eq!(layout.question.as_deref(), Some("Which layout reads better?"), "the part carries its question");
}

#[tokio::test]
async fn an_agent_item_that_asks_him_to_choose_or_approve_is_refused_unless_worded_as_a_question() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let list = |items: Vec<WaitingInput>| SetWaitingFor { items, session_id: Some(id.clone()) };

    let pick = rig.fleet.set_waiting(None, list(vec![choice("layout", "Pick the layout")])).await.unwrap_err();
    assert_eq!((pick.status(), pick.error().code.as_str()), (422, "fleet.session_waiting_question"));
    assert!(pick.error().message.contains("`layout`") && pick.error().message.contains("is not a question"), "{}", pick.error().message);
    let approve = WaitingInput { approves: true, act: None, options: Vec::new(), ..choice("ship", "Approve the release notes") };
    assert_eq!(rig.fleet.set_waiting(None, list(vec![approve])).await.unwrap_err().error().code, "fleet.session_waiting_question");

    // A step he carries out is no question, and one worded as a question is kept.
    let run = WaitingInput {
        act: Some(WaitingAct { kind: WaitingActKind::Run, target: "scripts/restart".into(), on: None }),
        options: Vec::new(),
        ..choice("restart", "Restart onto the new build")
    };
    rig.fleet.set_waiting(None, list(vec![run, choice("layout", "Rows or cards?")])).await.expect("kept");
}

#[tokio::test]
async fn a_plan_and_a_draft_are_titled_with_the_question_fleet_words() {
    let rig = rig_placing(None);
    let id = running(&rig).await;
    let record = recorded(&rig, first(&id)).await;
    let plan = record.waiting_for.iter().find(|one| one.source == WaitingSource::Plan).expect("the plan waits");
    assert_eq!(plan.text, "Approve this plan?");

    let draft = ipc::ProposeDraft { title: "Description for #41".into(), text: "Says why.".into(), session_id: Some(id.clone()), ..Default::default() };
    let record = rig.fleet.propose_draft(None, draft).await.unwrap();
    let item = record.waiting_for.iter().find(|one| one.source == WaitingSource::Draft).expect("the draft waits");
    assert_eq!((item.text.as_str(), item.artifact.as_deref()), ("Approve \"Description for #41\"?", Some("draft:Description for #41")));
}
