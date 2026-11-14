//! The claim, written before the code: **Approve all, Ask for changes and Dismiss all reach every item with an
//! Approve, across Jobs, in one call. Each item settles as its own answer settles it, with the same message to its
//! Job and his notes on that item only; a question, or a send back with no note, is refused for that item while the
//! rest settle. Dismiss all tells each Job once, a line per item, and closes each as his Dismiss does.**
//! `crates/ipc/operations/answer_items.toml`, `crates/ipc/operations/dismiss_items.toml`.

use std::sync::Arc;

use api::{HostedSessions, Sessions};
use ipc::{
    AnswerItems, DismissItems, ItemAnswer, ItemHow, ItemNote, ItemRef, ItemSettled, ProposeDraft, SessionId, SessionRecord, ShowSketch,
    WaitingSource,
};

use super::document_answers::{one_page, record_of, sent};
use super::session_host::{eventually, rig_placing, Rig};
use super::session_plan::{first, recorded, running};
use crate::session_host::{drafting, sketching};

const SCENE: &str = r#"{"nodes":[{"id":"a","kind":"box","x":0,"y":0,"title":"Bay"}],"edges":[]}"#;
const DRAFT: &str = "PR body";

fn drawn(id: &SessionId, title: &str) -> ShowSketch {
    ipc::decode("a sketch to show", format!(r#"{{"title":"{title}","session_id":"{}","scene":{SCENE}}}"#, id.as_str()).as_bytes()).unwrap()
}

async fn sketched(rig: &Rig, id: &SessionId, title: &str) {
    rig.fleet.show_sketch(None, drawn(id, title)).await.unwrap();
}

/// The draft's item id.
async fn drafted(rig: &Rig, id: &SessionId) -> String {
    let proposed = ProposeDraft { title: DRAFT.into(), text: "Says why.".into(), session_id: Some(id.clone()), ..Default::default() };
    let record = rig.fleet.propose_draft(None, proposed).await.unwrap();
    record.waiting_for.iter().find(|one| one.source == WaitingSource::Draft).expect("a draft waits").id.clone()
}

fn answering(id: &SessionId, item: &str, how: ItemHow, notes: &[&str]) -> ItemAnswer {
    ItemAnswer { session_id: id.clone(), item: item.into(), how, notes: notes.iter().map(|one| ItemNote { text: one.to_string(), on: None }).collect() }
}

async fn answered(rig: &Rig, items: Vec<ItemAnswer>) -> Vec<ItemSettled> {
    let settled = Arc::clone(&rig.fleet).answer_items(AnswerItems { items }, Vec::new()).await;
    settled.outcomes.into_iter().map(|one| one.outcome).collect()
}

fn ids(record: &SessionRecord) -> Vec<&str> {
    record.waiting_for.iter().map(|one| one.id.as_str()).collect()
}

/// What Fleet sent the Job's process, its own lines too.
fn told(rig: &Rig, id: &SessionId, containing: &str) -> usize {
    rig.stand_in.process_of(id.as_str()).map_or(0, |process| process.sent.lock().unwrap().iter().filter(|line| line.contains(containing)).count())
}

fn refused_code(outcome: &ItemSettled) -> &str {
    match outcome {
        ItemSettled::Refused { code, .. } => code,
        ItemSettled::Settled => "settled",
    }
}

#[tokio::test]
async fn one_call_approves_a_sketch_a_draft_a_page_and_a_plan_across_two_jobs_each_told_as_its_own_answer() {
    let rig = rig_placing(None);
    let (pages, _work, _file, path) = one_page(&rig).await;
    sketched(&rig, &pages, "Bay flow").await;
    let plans = running_second(&rig).await;
    let draft = drafted(&rig, &plans).await;
    recorded(&rig, first(&plans)).await;
    let page = format!("document:{path}");
    assert!(ids(&record_of(&rig, &plans).await).contains(&"plan:1"), "{:?}", ids(&record_of(&rig, &plans).await));

    let outcomes = answered(
        &rig,
        vec![
            answering(&pages, "sketch:Bay flow", ItemHow::Approve, &["Tighter spacing"]),
            answering(&plans, &draft, ItemHow::Approve, &["Say why", " ", "Shorter"]),
            answering(&pages, &page, ItemHow::Approve, &[]),
            answering(&plans, "plan:1", ItemHow::Approve, &[]),
        ],
    )
    .await;
    assert_eq!(outcomes, vec![ItemSettled::Settled; 4]);

    let to_pages = sent(&rig.rows(&pages).await);
    assert!(to_pages.contains(&sketching::approved("Bay flow", Some("1. Tighter spacing"))), "{to_pages:?}");
    assert!(to_pages.contains(&String::from("Approved document: Lease expiry, revision 1\nGo on from it as written.")), "{to_pages:?}");
    let to_plans = sent(&rig.rows(&plans).await);
    assert!(to_plans.contains(&drafting::approved(DRAFT, Some("1. Say why\n2. Shorter"))), "{to_plans:?}");
    assert!(to_plans.iter().all(|one| !one.contains("Tighter")), "a note stays on its own item: {to_plans:?}");
    eventually(|| async { told(&rig, &plans, "approved plan v1") == 1 }).await;
    assert!(ids(&record_of(&rig, &pages).await).is_empty(), "{:?}", ids(&record_of(&rig, &pages).await));
    assert!(ids(&record_of(&rig, &plans).await).is_empty(), "{:?}", ids(&record_of(&rig, &plans).await));
}

/// A second running Job, its process the second started.
async fn running_second(rig: &Rig) -> SessionId {
    let id = rig.start().await;
    rig.send(&id, "plan it").await;
    rig.stand_in.init(1);
    id
}

#[tokio::test]
async fn a_question_in_the_batch_is_refused_while_the_sketch_beside_it_settles() {
    let rig = rig_placing(None);
    let id = running(&rig).await;
    sketched(&rig, &id, "Bay flow").await;
    let fleet = Arc::clone(&rig.fleet);
    let session = id.as_str().to_string();
    let held = tokio::spawn(async move { fleet.session_permission(&session, asking(ONE_QUESTION)).await });
    let ask = eventually_asked(&rig, &id).await;

    let outcomes = answered(&rig, vec![answering(&id, &ask, ItemHow::Approve, &[]), answering(&id, "sketch:Bay flow", ItemHow::Approve, &[])]).await;
    assert_eq!(refused_code(&outcomes[0]), "fleet.items_not_approvable");
    assert_eq!(outcomes[1], ItemSettled::Settled);
    assert_eq!(ids(&record_of(&rig, &id).await), [ask.as_str()], "the question still waits, and only it");
    held.abort();
}

#[tokio::test]
async fn a_send_back_with_no_note_is_refused_for_that_item_only() {
    let rig = rig_placing(None);
    let id = running(&rig).await;
    sketched(&rig, &id, "One").await;
    sketched(&rig, &id, "Two").await;

    let outcomes = answered(
        &rig,
        vec![answering(&id, "sketch:One", ItemHow::SendBack, &["  "]), answering(&id, "sketch:Two", ItemHow::SendBack, &["Bigger"])],
    )
    .await;
    assert_eq!(refused_code(&outcomes[0]), "fleet.items_send_back_empty");
    assert_eq!(outcomes[1], ItemSettled::Settled);
    assert!(sent(&rig.rows(&id).await).contains(&sketching::sent_back("Two", "1. Bigger")));
    // Drawn in the turn still running, the one left waits once that turn ends.
    rig.stand_in.finishes(0, "Drawn.");
    eventually(|| async { !ids(&record_of(&rig, &id).await).is_empty() }).await;
    assert_eq!(ids(&record_of(&rig, &id).await), ["sketch:One"]);
}

#[tokio::test]
async fn dismissing_a_draft_and_a_sketch_of_one_job_tells_it_once_and_both_leave_needs_you() {
    let rig = rig_placing(None);
    let id = running(&rig).await;
    let draft = drafted(&rig, &id).await;
    sketched(&rig, &id, "Bay flow").await;
    let before = sent(&rig.rows(&id).await).len();

    let items = vec![ItemRef { session_id: id.clone(), item: draft }, ItemRef { session_id: id.clone(), item: "sketch:Bay flow".into() }];
    let settled = Arc::clone(&rig.fleet).dismiss_items(DismissItems { items }, Vec::new()).await;
    assert!(settled.outcomes.iter().all(|one| one.outcome == ItemSettled::Settled), "{:?}", settled.outcomes);

    let to_job = sent(&rig.rows(&id).await);
    assert_eq!(
        &to_job[before..],
        [format!("He passed on \"{DRAFT}\" with no verdict.\nHe passed on \"Bay flow\" with no verdict.")],
        "one message, a line each, and no draft's own withdrawal"
    );
    assert!(ids(&record_of(&rig, &id).await).is_empty(), "{:?}", ids(&record_of(&rig, &id).await));
}

#[tokio::test]
async fn dismissing_a_plan_sets_it_aside_as_his_dismiss_does() {
    let rig = rig_placing(None);
    let id = running(&rig).await;
    recorded(&rig, first(&id)).await;

    let items = vec![ItemRef { session_id: id.clone(), item: "plan:1".into() }];
    let settled = Arc::clone(&rig.fleet).dismiss_items(DismissItems { items }, Vec::new()).await;
    assert_eq!(settled.outcomes[0].outcome, ItemSettled::Settled);

    let record = record_of(&rig, &id).await;
    assert!(record.plan.as_ref().and_then(|plan| plan.set_aside_at.as_ref()).is_some(), "{:?}", record.plan);
    assert!(!ids(&record).contains(&"plan:1"));
    // The process reads the message as JSON, so its quotes arrive escaped.
    eventually(|| async { told(&rig, &id, r#"He passed on \"plan v1\" with no verdict."#) == 1 }).await;
    assert_eq!(told(&rig, &id, "Not ready for a plan yet"), 0);
}

#[tokio::test]
async fn a_plans_notes_go_with_its_approval_as_notes_on_the_whole_plan() {
    let rig = rig_placing(None);
    let id = running(&rig).await;
    recorded(&rig, first(&id)).await;

    let outcomes = answered(&rig, vec![answering(&id, "plan:1", ItemHow::Approve, &["Keep the old route"])]).await;
    assert_eq!(outcomes, [ItemSettled::Settled]);
    eventually(|| async { told(&rig, &id, "approved plan v1 with changes") == 1 }).await;
    assert_eq!(told(&rig, &id, "Note on the plan: 1. Keep the old route"), 1);
    assert!(record_of(&rig, &id).await.plan.is_some_and(|plan| plan.approved));
}

async fn eventually_asked(rig: &Rig, id: &SessionId) -> String {
    let call = std::sync::Mutex::new(String::new());
    eventually(|| async {
        let asked = Arc::clone(&rig.fleet).get_session(id.clone()).await.unwrap().session.hosted.unwrap().asked;
        match asked {
            Some(asked) => {
                *call.lock().unwrap() = asked.call;
                true
            }
            None => false,
        }
    })
    .await;
    format!("ask:{}", call.into_inner().unwrap())
}

fn asking(body: &str) -> ipc::AskingToRun {
    ipc::decode("an ask", body.as_bytes()).expect("an ask")
}

const ONE_QUESTION: &str = r#"{"tool_name":"AskUserQuestion","tool_use_id":"t1","input":{"questions":[
  {"question":"Which clock?","header":"Clock","multiSelect":false,
   "options":[{"label":"Fake","description":""},{"label":"Frozen","description":""}]}]}}"#;
