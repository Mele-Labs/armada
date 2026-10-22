//! A reply that claims what the agent's call does not back: a done said without `all_done`, a walk
//! asked for without `show_window`. Fleet reminds once, where the turn ends, and never reads the reply
//! to its reminder. `crate::session_host::reply_claims`.
//!
//! The replies are the two the owner reported on 21 Oct 2026, one Job saying it was done and another
//! listing walks no window was open for.

use std::time::Duration;

use api::Sessions;
use ipc::{AllDone, SessionId, SessionTurn, ShowWindow};

use super::session_host::{eventually, rig_placing, Rig};
use crate::session_host::reply_claims::{
    asks_to_walk, claims_done, unbacked, Backed, Claim, DONE_REMINDER, REMINDED_DONE, WALK_REMINDER,
};

const SAID_DONE: &str = "PR 2681 is merged and the job is done.";
const ASKED_TO_WALK: &str = "Finish requests-repository-picker and requests-in-a-package, approve or leave notes.";

/// How many lines the agent was sent that carry `words`.
fn sent_with(rig: &Rig, words: &str) -> usize {
    rig.stand_in.process(0).sent.lock().unwrap().iter().filter(|line| line.contains(words)).count()
}

/// A Session with its first turn running.
async fn working(rig: &Rig) -> SessionId {
    let id = rig.start().await;
    rig.send(&id, "carry on").await;
    rig.stand_in.init(0);
    // Begun, so a done said in it is not the one a turn's start clears.
    eventually(|| async { rig.fleet.hosts().of(id.as_str()).state().turn_began.is_some() }).await;
    id
}

/// The turn ended and whatever follows its end has had time to happen.
async fn settled(rig: &Rig, id: &SessionId) {
    eventually(|| async { rig.turn(id).await == SessionTurn::Idle }).await;
    tokio::time::sleep(Duration::from_millis(300)).await;
}

#[tokio::test]
async fn a_reply_saying_the_job_is_done_without_all_done_is_reminded_once() {
    let rig = rig_placing(None);
    let _id = working(&rig).await;
    rig.stand_in.finishes(0, SAID_DONE);
    eventually(|| async { sent_with(&rig, DONE_REMINDER) == 1 }).await;
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert_eq!(sent_with(&rig, DONE_REMINDER), 1, "one reminder for one reply");
    assert_eq!(sent_with(&rig, WALK_REMINDER), 0);
}

#[tokio::test]
async fn a_done_deferred_to_later_is_not_reminded() {
    let rig = rig_placing(None);
    let id = working(&rig).await;
    rig.stand_in.finishes(0, "The job is done once ci passes.");
    settled(&rig, &id).await;
    assert_eq!(sent_with(&rig, DONE_REMINDER), 0);
}

#[tokio::test]
async fn a_done_with_all_done_standing_is_not_reminded() {
    let rig = rig_placing(None);
    let id = working(&rig).await;
    rig.fleet.all_done(None, AllDone { session_id: Some(id.clone()) }).await.unwrap();
    rig.stand_in.finishes(0, SAID_DONE);
    settled(&rig, &id).await;
    assert_eq!(sent_with(&rig, DONE_REMINDER), 0);
}

#[tokio::test]
async fn a_reply_asking_for_walks_with_no_window_open_is_reminded_to_show_them() {
    let rig = rig_placing(None);
    let _id = working(&rig).await;
    rig.stand_in.finishes(0, ASKED_TO_WALK);
    eventually(|| async { sent_with(&rig, WALK_REMINDER) == 1 }).await;
    assert_eq!(sent_with(&rig, DONE_REMINDER), 0);
}

#[tokio::test]
async fn a_reply_asking_for_walks_with_a_window_open_is_not_reminded() {
    let rig = rig_placing(None);
    let id = working(&rig).await;
    let shown = ShowWindow {
        url: "http://localhost:5173/".into(),
        title: Some("Repository picker".into()),
        session_id: Some(id.clone()),
        variations: Vec::new(),
        ..Default::default()
    };
    rig.fleet.show_window(None, shown).await.unwrap();
    rig.stand_in.finishes(0, ASKED_TO_WALK);
    settled(&rig, &id).await;
    assert_eq!(sent_with(&rig, WALK_REMINDER), 0);
}

#[tokio::test]
async fn the_reply_to_a_reminder_is_not_reminded_again() {
    let rig = rig_placing(None);
    let id = working(&rig).await;
    rig.stand_in.finishes(0, SAID_DONE);
    eventually(|| async { sent_with(&rig, DONE_REMINDER) == 1 }).await;
    eventually(|| async {
        let tell = rig.fleet.store().lock().await.tell_of(id.as_str()).unwrap();
        tell.is_some_and(|tell| tell.kind == REMINDED_DONE)
    })
    .await;
    // A reply is later than the tell it follows, as a turn of the agent's always is.
    tokio::time::sleep(Duration::from_millis(20)).await;
    rig.stand_in.init(0);
    rig.stand_in.finishes(0, "The job is done.");
    settled(&rig, &id).await;
    assert_eq!(sent_with(&rig, DONE_REMINDER), 1, "the reply to the reminder is not read");
}

fn nothing_standing() -> Backed {
    Backed { done: false, waiting: false, walk: false, subagents: false }
}

#[test]
fn a_close_that_says_the_work_is_over_claims_done() {
    for said in [
        SAID_DONE,
        "Everything is pushed.\n\nThe work is complete.",
        "The job's done.",
        "Nothing left to do here.",
        "All done.",
    ] {
        assert!(claims_done(said), "{said}");
    }
}

#[test]
fn a_close_that_defers_negates_or_asks_does_not_claim_done() {
    for said in [
        "The job is done once ci passes.",
        "The job is not done.",
        "The work isn't finished.",
        "It is done when the review lands.",
        "I will call all_done after the merge.",
        "Is the job done?",
        "I have done the work on the parser, and the tests are next.",
        "Task 3 is done, and task 4 is next.",
        "done",
        // Quoted, it is about a reply, not one.
        "The reader now reminds a reply that says `the job is done` with no all_done.",
        "A close like \"the work is complete\" gets one reminder.",
    ] {
        assert!(!claims_done(said), "{said}");
    }
}

#[test]
fn a_reply_asking_him_to_walk_or_leave_notes_asks_to_walk() {
    for said in [
        ASKED_TO_WALK,
        "| Who | What | When |\n|---|---|---|\n| Nick | Finish requests-repository-picker and requests-in-a-package, approve or leave notes | Now |",
        "Open the walk and approve it.",
        "The walks are ready: approve each or leave notes.",
        "Open `http://localhost:5441/?walk=picker` and approve it.",
    ] {
        assert!(asks_to_walk(said), "{said}");
    }
    for said in [
        "I walked the code path again.",
        "The walk window is not needed for this change.",
        "Approve the PR.",
        // The two replies of this change's own job the reminder misfired on: one describes walks, one
        // quotes the phrases it reads.
        "A reply that says done or asks you to review walks without making the call gets one reminder.",
        "Your screenshot's case, \"approve or leave notes\", still gets the reminder.",
    ] {
        assert!(!asks_to_walk(said), "{said}");
    }
}

#[test]
fn a_claim_something_standing_backs_is_not_reminded() {
    assert_eq!(unbacked(SAID_DONE, &nothing_standing()), Some(Claim::Done));
    assert_eq!(unbacked(SAID_DONE, &Backed { done: true, ..nothing_standing() }), None);
    assert_eq!(unbacked(SAID_DONE, &Backed { waiting: true, ..nothing_standing() }), None, "something is owed");
    assert_eq!(unbacked(SAID_DONE, &Backed { subagents: true, ..nothing_standing() }), None, "a subagent is at work");
    assert_eq!(unbacked(ASKED_TO_WALK, &Backed { subagents: true, ..nothing_standing() }), None);
    assert_eq!(unbacked(ASKED_TO_WALK, &nothing_standing()), Some(Claim::Walk));
    assert_eq!(unbacked(ASKED_TO_WALK, &Backed { walk: true, waiting: true, ..nothing_standing() }), None);
}

#[test]
fn a_reply_claiming_both_is_reminded_of_the_walk() {
    let both = format!("{ASKED_TO_WALK}\n\nThe job is done.");
    assert_eq!(unbacked(&both, &nothing_standing()), Some(Claim::Walk));
}
