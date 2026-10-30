//! The follow-ups an agent keeps for itself: held across its replies, on the row, settled by the
//! agent or by the person, and a hold on `all_done` while any is open. The sentence readers that find
//! them read with a model these tests do not load, so these keep sentences the way the reader would
//! hand them over.

use std::sync::Arc;

use api::{HostedSessions, Sessions};
use api::Refusal;
use ipc::{AllDone, DeferLater, LaterHow, SessionId, SessionRecord, SettleLater};

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
    SettleLater { id: which.into(), how, issue: issue.map(str::to_string), reason: reason.map(str::to_string), session_id: Some(id.clone()), misread: false }
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

fn defer(id: &SessionId, sentence: &str) -> DeferLater {
    DeferLater { sentence: sentence.into(), session_id: Some(id.clone()) }
}

#[tokio::test]
async fn the_job_defers_a_followup_with_a_call_and_the_same_sentence_is_one_item() {
    let rig = rig(None);
    let id = rig.an_agent().await;

    let first = rig.fleet.defer_later(caller(), defer(&id, SLOW)).await.unwrap();
    assert_eq!((first.id.as_str(), first.sentence.as_str()), ("later-1", SLOW));
    let second = rig.fleet.defer_later(caller(), defer(&id, DOCS)).await.unwrap();
    assert_eq!(second.id, "later-2");
    assert_eq!(second.open.iter().map(|one| one.id.as_str()).collect::<Vec<_>>(), ["later-1", "later-2"]);

    // Said again, even with stray space round it, it is the item it was.
    let again = rig.fleet.defer_later(caller(), defer(&id, &format!("  {SLOW}\n"))).await.unwrap();
    assert_eq!(again.id, "later-1");
    assert_eq!(open_ids(&read(&rig, &id).await), ["later-1", "later-2"]);
}

#[tokio::test]
async fn a_defer_with_nothing_to_keep_or_no_job_to_keep_it_on_is_refused() {
    let rig = rig(None);
    let id = rig.an_agent().await;

    let empty = rig.fleet.defer_later(caller(), defer(&id, "  \n")).await.expect_err("nothing said");
    let Refusal::Unacceptable(why) = empty else { panic!("{empty:?}") };
    assert_eq!(why.code.to_string(), "fleet.later_not_deferrable");
    assert!(read(&rig, &id).await.later.is_empty());

    // No connection places the call and it names no job.
    let placed_nowhere = rig.fleet.defer_later(None, DeferLater { sentence: SLOW.into(), session_id: None }).await;
    assert!(matches!(placed_nowhere, Err(Refusal::Unacceptable(_))), "{placed_nowhere:?}");

    // The refusal is for the empty sentence, not for the job.
    rig.fleet.defer_later(caller(), defer(&id, SLOW)).await.expect("a sentence is kept");

    Arc::clone(&rig.fleet).close_session(ipc::CloseSession { session_id: id.clone() }).await.expect("ended");
    let ended = rig.fleet.defer_later(caller(), defer(&id, DOCS)).await.expect_err("an ended job keeps nothing");
    let Refusal::IllegalMove(why) = ended else { panic!("{ended:?}") };
    assert_eq!(why.code.to_string(), "fleet.session_closed");
}

fn done(id: &SessionId) -> AllDone {
    AllDone { session_id: Some(id.clone()), summary: None }
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

/// The person's thumbs on a follow-up. `Fake` puts a sentence about the store on one axis, others on the other.
mod thumbs {
    use super::*;
    use adapter_traits::{NotEmbedded, TextModel, TextReading};
    use ipc::ConfirmLater;

    struct Fake;

    impl TextModel for Fake {
        fn model(&self) -> &str {
            "fake@0"
        }

        fn embed(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, NotEmbedded> {
            Ok(texts.iter().map(|text| if text.to_lowercase().contains("store") { vec![1.0, 0.0] } else { vec![0.0, 1.0] }).collect())
        }

        fn read(&self, _: TextReading, _: &[&str]) -> Result<Vec<f32>, NotEmbedded> {
            Err(NotEmbedded(String::from("not read here")))
        }
    }

    const STORE: &str = "I will tidy the store after this lands.";
    const STORE_AGAIN: &str = "I will clean up the store once the merge is in.";

    fn down(id: &SessionId, which: &str) -> SettleLater {
        SettleLater { misread: true, ..settle(id, which, LaterHow::Dropped, None, None) }
    }

    #[tokio::test]
    async fn thumbs_down_settles_the_row_and_keeps_its_sentence_as_no_follow_up() {
        let rig = rig(None);
        let id = rig.an_agent().await;
        rig.fleet.later_kept(id.as_str(), &said(&[SLOW]), FIRST).await;
        rig.fleet.settle_later(None, down(&id, "later-1")).await.unwrap();
        assert!(read(&rig, &id).await.later.is_empty());
        assert_eq!(rig.fleet.store().lock().await.later_misreads().unwrap(), [SLOW]);
    }

    #[tokio::test]
    async fn an_agents_settle_marked_misread_is_refused_and_the_row_stands() {
        let rig = rig(None);
        let id = rig.an_agent().await;
        rig.fleet.later_kept(id.as_str(), &said(&[SLOW]), FIRST).await;
        // With the reason an agent owes, so only the misread can refuse it.
        let with_reason = SettleLater { reason: Some("not needed".into()), ..down(&id, "later-1") };
        let refused = rig.fleet.settle_later(caller(), with_reason).await;
        assert_eq!(code(refused), "fleet.later_not_settleable");
        assert_eq!(open_ids(&read(&rig, &id).await), ["later-1"]);
        assert!(rig.fleet.store().lock().await.later_misreads().unwrap().is_empty());
    }

    #[tokio::test]
    async fn a_follow_up_near_one_marked_not_deferred_is_not_kept() {
        let rig = rig(None);
        rig.fleet.language_model().loaded(Arc::new(Fake));
        let id = rig.an_agent().await;
        rig.fleet.later_kept(id.as_str(), &said(&[STORE]), FIRST).await;
        rig.fleet.settle_later(None, down(&id, "later-1")).await.unwrap();
        // The vectors are embedded on a blocking thread; wait until both are in.
        let model: Arc<dyn TextModel> = Arc::new(Fake);
        super::super::session_host::eventually(|| async {
            rig.fleet.item_vectors().known(Arc::clone(&model), id.as_str(), &[STORE, STORE_AGAIN, DOCS]).len() == 3
        })
        .await;
        assert!(!rig.fleet.later_kept(id.as_str(), &said(&[STORE_AGAIN]), SECOND).await, "near the marked one");
        assert!(rig.fleet.later_kept(id.as_str(), &said(&[DOCS]), SECOND).await, "far from it");
        assert_eq!(open_ids(&read(&rig, &id).await), ["later-2"]);
    }

    #[tokio::test]
    async fn thumbs_up_keeps_the_row_open_confirmed_and_keeps_its_sentence() {
        let rig = rig(None);
        let id = rig.an_agent().await;
        rig.fleet.later_kept(id.as_str(), &said(&[SLOW]), FIRST).await;
        let row = rig.fleet.confirm_later(ConfirmLater { id: "later-1".into(), session_id: Some(id.clone()) }).await.unwrap();
        assert_eq!(row.later.iter().map(|one| (one.id.as_str(), one.confirmed)).collect::<Vec<_>>(), [("later-1", true)]);
        assert_eq!(open_ids(&read(&rig, &id).await), ["later-1"]);
        assert!(read(&rig, &id).await.later[0].confirmed);
        assert_eq!(rig.fleet.store().lock().await.later_confirmed().unwrap(), [SLOW]);
        // Still deferred work: it holds all_done.
        assert!(rig.fleet.all_done(caller(), done(&id)).await.is_err());
    }
}
