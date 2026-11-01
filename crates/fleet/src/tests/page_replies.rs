//! His reply in the thread approves a page the Job put to him, read off the thread once a pass has
//! run: "Run the plan and merge it. I'm AFK" settles the page and the plan held behind it, without a
//! press on the card and without the Job being told a second time.
//!
//! **The model is the fake**, as the dispute reader's cases have it: what runs is which replies are
//! read, when, and what Fleet does with the answer.

use std::sync::Arc;

use ipc::{Instant, SessionId, SessionRow, SessionVoice};
use testkit::FakeJudge;

use super::document_answers::{one_page, plan_of, plan_recorded, record_of, sent, the_document, waits, written, revision_kept};
use super::planted::Held;
use super::session_host::{eventually, rig_fitted, Rig};
use super::tmp::TempDir;
use crate::session_host::document_answers::document_items;
use crate::session_host::page_replies::{approves, replies_after, APPROVES};

const REPLY: &str = "Run the plan and merge it. I'm AFK";

fn message(id: &str, at: &str, from: SessionVoice, text: &str, notice: Option<&str>) -> SessionRow {
    SessionRow::Message {
        id: id.into(),
        at: Instant::carried(at),
        from,
        text: text.into(),
        files: Vec::new(),
        tags: Vec::new(),
        notice: notice.map(str::to_string),
        queued: false,
    }
}

#[test]
fn only_his_own_words_after_the_page_are_replies() {
    let you = |id: &str, at: &str, text: &str| message(id, at, SessionVoice::You, text, None);
    let rows = vec![
        you("m1", "2026-08-26T09:00:01.000Z", "before the page was kept"),
        you("m2", "2026-08-26T09:00:10.000Z", "Looks good, go ahead"),
        message("m3", "2026-08-26T09:00:11.000Z", SessionVoice::You, "Subagents burned 5.0 units.", Some("subagent_spend")),
        message("m4", "2026-08-26T09:00:12.000Z", SessionVoice::Agent, "On it.", None),
        you("ask-1:said", "2026-08-26T09:00:13.000Z", "Yes, the second one"),
        you("m5", "2026-08-26T09:00:14.000Z", "Approved: https://localhost:5173/walk"),
        you("m6", "2026-08-26T09:00:15.000Z", "Fleet: the build is red"),
        you("m7", "2026-08-26T09:00:16.000Z", "Approved document: Lease expiry, revision 1\nGo on from it as written."),
        you("m8", "2026-08-26T09:00:17.000Z", "Sent back document: Lease expiry, revision 1\nSay more."),
        you("m9", "2026-08-26T09:00:18.000Z", "Run the plan and merge it"),
    ];

    let kept: Vec<(&str, &str)> = replies_after(&rows, "2026-08-26T09:00:05.000Z");

    assert_eq!(
        kept,
        [("Looks good, go ahead", "2026-08-26T09:00:10.000Z"), ("Run the plan and merge it", "2026-08-26T09:00:18.000Z")]
    );
    assert!(replies_after(&rows, "2026-08-26T09:00:18.000Z").is_empty(), "the newest read is not read again");
}

#[test]
fn only_a_yes_approves() {
    assert!(approves("approves: yes"));
    assert!(approves("Approves: YES"));
    assert!(approves("thinking it over\n  approves: yes  "));
    assert!(!approves("approves: no"));
    assert!(!approves(""));
    assert!(!approves("yes"));
    assert!(!approves("approves:"));
    assert!(!approves("approves: maybe"));
    assert_eq!(APPROVES, "approves:");
}

/// A rig whose clock the case holds and whose model answers `answer`.
fn answering(answer: &str) -> (Rig, Arc<FakeJudge>) {
    let judge = Arc::new(FakeJudge::saying(answer));
    let planted = Arc::clone(&judge);
    let rig = rig_fitted(move |fittings| {
        fittings.clock = Arc::new(Held::started());
        fittings.judge = planted;
    });
    (rig, judge)
}

/// The questions put about a page's replies.
fn page_asks(judge: &FakeJudge) -> Vec<String> {
    judge.asked().into_iter().filter(|asked| asked.contains(APPROVES)).collect()
}

fn page_waits(record: &ipc::SessionRecord, path: &str) -> bool {
    waits(record).iter().any(|(item, ..)| *item == format!("document:{path}"))
}

#[tokio::test]
async fn a_reply_that_goes_ahead_settles_the_page_and_approves_the_plan_without_telling_the_job_again() {
    let (rig, judge) = answering("approves: yes");
    let (id, _work, _file, path) = one_page(&rig).await;
    plan_recorded(&rig, &id).await;
    rig.send(&id, REPLY).await;
    let heard = || rig.stand_in.process(0).sent.lock().unwrap().clone();
    eventually(|| async { heard().iter().any(|line| line.contains(REPLY)) }).await;
    let told = heard().len();
    assert!(page_waits(&record_of(&rig, &id).await, &path), "asking before the pass");

    rig.fleet.pages_read().await;

    let asks = page_asks(&judge);
    assert_eq!(asks.len(), 1, "{asks:?}");
    assert!(asks[0].contains(REPLY) && asks[0].contains("Lease expiry"), "{}", asks[0]);
    let record = record_of(&rig, &id).await;
    let detail = &the_document(&record, &path).unwrap().detail;
    assert_eq!(detail.get("settled").map(String::as_str), Some("approved"), "{detail:?}");
    assert!(!page_waits(&record, &path), "{:?}", waits(&record));
    assert_eq!(document_items(&record.attachments, id.as_str()).count(), 0);
    let plan = plan_of(&rig, &id).await;
    assert!(plan.approved && plan.approved_at.is_some(), "approved with the page: {plan:?}");
    assert_eq!(sent(&rig.rows(&id).await).pop().as_deref(), Some(REPLY), "the card sent nothing");
    // The plan's approval tells the Job to build; the page's own Approved message would be a second telling of his reply.
    let after = heard();
    assert!(after.iter().all(|line| !line.contains("Approved document")), "{after:?}");
    assert_eq!(after.len(), told + 1, "only the plan's go-ahead: {after:?}");
    assert!(after[told].contains("approved plan v1"), "{after:?}");
}

#[tokio::test]
async fn a_reply_that_does_not_approve_leaves_the_page_asking_and_is_read_once() {
    let (rig, judge) = answering("approves: no");
    let (id, _work, _file, path) = one_page(&rig).await;
    plan_recorded(&rig, &id).await;
    rig.send(&id, "Why a lease and not a lock?").await;

    rig.fleet.pages_read().await;
    rig.fleet.pages_read().await;

    assert_eq!(page_asks(&judge).len(), 1, "no new reply, no second call: {:?}", page_asks(&judge));
    let record = record_of(&rig, &id).await;
    assert!(!the_document(&record, &path).unwrap().detail.contains_key("settled"));
    assert!(page_waits(&record, &path), "still waits on him");
    assert!(!plan_of(&rig, &id).await.approved);

    rig.send(&id, "Fine, go ahead").await;
    rig.fleet.pages_read().await;
    assert_eq!(page_asks(&judge).len(), 2, "a new reply is read");
}

#[tokio::test]
async fn a_reply_sent_before_the_revision_was_kept_is_not_read() {
    let (rig, judge) = answering("approves: yes");
    let id: SessionId = rig.start().await;
    rig.send(&id, "write it up").await;
    rig.stand_in.init(0);
    rig.send(&id, "Looks good, run the plan and merge it").await;
    let work = TempDir::new();
    let file = work.path().join(".armada/documents/lease.html");
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    let path = file.to_string_lossy().into_owned();
    written(&rig, &file, "first", "toolu_1").await;
    revision_kept(&rig, &id, &path, "1").await;

    rig.fleet.pages_read().await;

    assert!(page_asks(&judge).is_empty(), "{:?}", page_asks(&judge));
    assert!(page_waits(&record_of(&rig, &id).await, &path), "his earlier words did not approve a page they never saw");
}
