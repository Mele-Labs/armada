//! The follow-ups an agent keeps for itself: held across its replies, on the row, settled by the
//! agent or by the person, and a hold on `all_done` while any is open. The sentence readers that find
//! them read with a model these tests do not load, so these keep sentences the way the reader would
//! hand them over.

use std::sync::Arc;

use api::{HostedSessions, Sessions};
use api::Refusal;
use ipc::{AllDone, LaterHow, SessionId, SessionRecord, SettleLater};

use super::agent_starts::{caller, rig, Rig};

const SLOW: &str = "I will file the slow fixture load once this lands.";
const DOCS: &str = "I will tidy the docs after the merge.";
const FIRST: &str = "2026-10-21T10:00:00.000Z";
const SECOND: &str = "2026-10-21T10:05:00.000Z";

async fn read(rig: &Rig, id: &SessionId) -> SessionRecord {
    rig.record(id).await
}

fn said(sentences: &[&str]) -> Vec<String> {
    sentences.iter().map(|one| one.to_string()).collect()
}

#[tokio::test]
async fn a_followup_survives_the_next_reply_and_the_same_sentence_is_one_item() {
    let rig = rig(None);
    let id = rig.an_agent().await;
    assert!(read(&rig, &id).await.later.is_empty());

    assert!(rig.fleet.later_kept(id.as_str(), &said(&[SLOW]), FIRST).await);
    // The next reply says a different one, and the first again.
    assert!(rig.fleet.later_kept(id.as_str(), &said(&[DOCS, SLOW]), SECOND).await);
    assert!(!rig.fleet.later_kept(id.as_str(), &said(&[SLOW]), SECOND).await, "nothing new");

    let later: Vec<(String, String, String)> =
        read(&rig, &id).await.later.into_iter().map(|one| (one.id, one.sentence, one.said_at.as_str().to_string())).collect();
    assert_eq!(
        later,
        [
            ("later-1".to_string(), SLOW.to_string(), FIRST.to_string()),
            ("later-2".to_string(), DOCS.to_string(), SECOND.to_string()),
        ]
    );
}

fn settle(id: &SessionId, which: &str, how: LaterHow, issue: Option<&str>, reason: Option<&str>) -> SettleLater {
    SettleLater { id: which.into(), how, issue: issue.map(str::to_string), reason: reason.map(str::to_string), session_id: Some(id.clone()) }
}

fn open_ids(record: &SessionRecord) -> Vec<&str> {
    record.later.iter().map(|one| one.id.as_str()).collect()
}

fn code(refused: Result<ipc::LaterSettled, Refusal>) -> String {
    match refused.expect_err("refused") {
        Refusal::Unacceptable(why) | Refusal::IllegalMove(why) => why.code.to_string(),
        other => format!("{other:?}"),
    }
}

#[tokio::test]
async fn the_job_settles_a_followup_as_done_filed_or_dropped_and_it_leaves_the_list() {
    let rig = rig(None);
    let id = rig.an_agent().await;
    rig.fleet.later_kept(id.as_str(), &said(&[SLOW, DOCS, "I will rename the helper."]), FIRST).await;

    let done = rig.fleet.settle_later(caller(), settle(&id, "later-1", LaterHow::Done, None, None)).await.unwrap();
    assert_eq!((done.id.as_str(), done.sentence.as_str()), ("later-1", SLOW));
    assert_eq!(done.open.iter().map(|one| one.id.as_str()).collect::<Vec<_>>(), ["later-2", "later-3"]);
    assert_eq!(open_ids(&read(&rig, &id).await), ["later-2", "later-3"]);

    rig.fleet.settle_later(caller(), settle(&id, "later-2", LaterHow::Filed, Some("#2710"), None)).await.unwrap();
    rig.fleet.settle_later(caller(), settle(&id, "later-3", LaterHow::Dropped, None, Some("the helper is deleted in the next change"))).await.unwrap();
    assert!(read(&rig, &id).await.later.is_empty());

    // A reply read again does not bring a settled one back.
    assert!(!rig.fleet.later_kept(id.as_str(), &said(&[SLOW, DOCS]), SECOND).await);
}

#[tokio::test]
async fn a_settle_that_says_too_little_or_names_nothing_open_is_refused_and_leaves_the_item() {
    let rig = rig(None);
    let id = rig.an_agent().await;
    rig.fleet.later_kept(id.as_str(), &said(&[SLOW]), FIRST).await;

    let unfiled = rig.fleet.settle_later(caller(), settle(&id, "later-1", LaterHow::Filed, None, None)).await;
    assert_eq!(code(unfiled), "fleet.later_not_settleable");
    let garbled = rig.fleet.settle_later(caller(), settle(&id, "later-1", LaterHow::Filed, Some("the slow one"), None)).await;
    assert_eq!(code(garbled), "fleet.later_not_settleable");
    let unreasoned = rig.fleet.settle_later(caller(), settle(&id, "later-1", LaterHow::Dropped, None, Some("  "))).await;
    assert_eq!(code(unreasoned), "fleet.later_not_settleable");
    let unknown = rig.fleet.settle_later(caller(), settle(&id, "later-9", LaterHow::Done, None, None)).await;
    assert_eq!(code(unknown), "fleet.later_not_open");
    assert_eq!(open_ids(&read(&rig, &id).await), ["later-1"]);

    rig.fleet.settle_later(caller(), settle(&id, "later-1", LaterHow::Done, None, None)).await.unwrap();
    let again = rig.fleet.settle_later(caller(), settle(&id, "later-1", LaterHow::Done, None, None)).await;
    assert_eq!(code(again), "fleet.later_not_open", "settled already");
}

#[tokio::test]
async fn the_persons_drop_needs_no_reason() {
    let rig = rig(None);
    let id = rig.an_agent().await;
    rig.fleet.later_kept(id.as_str(), &said(&[SLOW]), FIRST).await;
    // No connection places the call: it is Bridge's, on the person's press.
    rig.fleet.settle_later(None, settle(&id, "later-1", LaterHow::Dropped, None, None)).await.unwrap();
    assert!(read(&rig, &id).await.later.is_empty());
}

fn done(id: &SessionId) -> AllDone {
    AllDone { session_id: Some(id.clone()) }
}

#[tokio::test]
async fn all_done_is_refused_while_a_followup_is_open_and_names_each() {
    let rig = rig(None);
    let id = rig.an_agent().await;
    rig.fleet.later_kept(id.as_str(), &said(&[SLOW, DOCS]), FIRST).await;

    let refused = rig.fleet.all_done(caller(), done(&id)).await.expect_err("held by what is open");
    let Refusal::IllegalMove(why) = refused else { panic!("{refused:?}") };
    for named in [SLOW, DOCS, "later-1", "later-2", "settle_later", "filed", "dropped", "reason"] {
        assert!(why.message.contains(named), "{named} in {}", why.message);
    }
    assert_eq!(read(&rig, &id).await.done_since, None, "nothing was stamped");

    // Settling the first leaves the refusal naming only the second.
    rig.fleet.settle_later(caller(), settle(&id, "later-1", LaterHow::Done, None, None)).await.unwrap();
    let refused = rig.fleet.all_done(caller(), done(&id)).await.expect_err("one is still open");
    let Refusal::IllegalMove(why) = refused else { panic!("{refused:?}") };
    assert!(why.message.contains(DOCS) && !why.message.contains(SLOW), "{}", why.message);

    rig.fleet.settle_later(caller(), settle(&id, "later-2", LaterHow::Dropped, None, Some("not needed"))).await.unwrap();
    assert!(rig.fleet.all_done(caller(), done(&id)).await.unwrap().done_since.is_some());
}

#[tokio::test]
async fn the_persons_end_job_is_never_held_by_an_open_followup() {
    let rig = rig(None);
    let id = rig.an_agent().await;
    rig.fleet.later_kept(id.as_str(), &said(&[SLOW]), FIRST).await;

    let ended = Arc::clone(&rig.fleet).close_session(ipc::CloseSession { session_id: id.clone() }).await.expect("ended");
    assert_eq!(ended.state, ipc::SessionState::Ended);
}
