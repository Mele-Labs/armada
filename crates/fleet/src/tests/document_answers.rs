//! The claim, written before the code: **every revision Fleet keeps of a page a Job wrote under
//! `.armada/documents/` is put to the person, read off the kept page and never off the tool call, and
//! waits in Needs you as `document:<path>` until he answers it. Approve or a note sent back reaches the
//! Job as his message and settles that revision; a new revision asks again, and only the newest revision
//! can be answered.** `crates/ipc/operations/answer_document.toml`.

use std::collections::BTreeMap;
use std::sync::Arc;

use adapter_traits::{CallDetail, DroneEvent};
use api::{HostedSessions, Refusal, Sessions};
use ipc::{AnswerDocument, Attachment, DocumentHow, SessionId, SessionRecord, SessionRow, SessionVoice, WaitingSource};

use super::session_host::{eventually, rig_placing, Rig};
use super::tmp::TempDir;
use crate::session_host::document_answers::approved_on_newest;

const TITLE: &str = "Lease expiry";

fn page(body: &str) -> String {
    format!("<html><head><title>{TITLE}</title></head><body><p>{body}</p></body></html>")
}

/// A page written by the Job's Write call, which the tool answered.
async fn written(rig: &Rig, file: &std::path::Path, body: &str, call: &str) {
    std::fs::write(file, page(body)).unwrap();
    rig.stand_in.says(
        0,
        vec![
            DroneEvent::Called { tool: "Write".into(), call: call.into(), detail: CallDetail::of(&format!("{} +3", file.display())) },
            DroneEvent::Answered { call: call.into(), failed: false },
        ],
    );
}

async fn record_of(rig: &Rig, id: &SessionId) -> SessionRecord {
    let listed = rig.fleet.list_sessions(None, None, None).await.unwrap().sessions;
    listed.into_iter().find(|one| &one.id == id).expect("listed")
}

fn the_document<'a>(record: &'a SessionRecord, path: &str) -> Option<&'a Attachment> {
    record.attachments.iter().find(|one| one.kind == "artifact" && one.target == path)
}

async fn revision_kept(rig: &Rig, id: &SessionId, path: &str, revision: &str) {
    eventually(|| async {
        the_document(&record_of(rig, id).await, path).and_then(|one| one.detail.get("revision")).map(String::as_str) == Some(revision)
    })
    .await;
}

fn waits(record: &SessionRecord) -> Vec<(&str, &str, WaitingSource)> {
    record.waiting_for.iter().map(|one| (one.id.as_str(), one.text.as_str(), one.source)).collect()
}

fn sent(rows: &[SessionRow]) -> Vec<String> {
    rows.iter()
        .filter_map(|row| match row {
            SessionRow::Message { from: SessionVoice::You, text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect()
}

fn answer(id: &SessionId, path: &str, revision: &str, how: DocumentHow, words: Option<&str>) -> AnswerDocument {
    AnswerDocument {
        session_id: id.clone(),
        document: format!("document:{path}"),
        revision: revision.into(),
        how,
        words: words.map(str::to_string),
    }
}

/// A Job that wrote one page, kept as its first revision, and the page's file and path.
async fn one_page(rig: &Rig) -> (SessionId, TempDir, std::path::PathBuf, String) {
    let id = rig.start().await;
    rig.send(&id, "write it up").await;
    rig.stand_in.init(0);
    let work = TempDir::new();
    let file = work.path().join(".armada/documents/lease.html");
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    let path = file.to_string_lossy().into_owned();
    written(rig, &file, "first", "toolu_1").await;
    revision_kept(rig, &id, &path, "1").await;
    (id, work, file, path)
}

#[tokio::test]
async fn a_kept_revision_is_asked_and_waits_as_one_item_that_opens_the_job() {
    let rig = rig_placing(None);
    let (id, _work, _file, path) = one_page(&rig).await;
    let record = record_of(&rig, &id).await;
    let detail = &the_document(&record, &path).unwrap().detail;
    assert_eq!(detail.get("asked").map(String::as_str), Some("true"), "{detail:?}");
    let kept_at = detail.get("kept_at").expect("when the revision was kept");
    assert!(kept_at.as_bytes().get(10) == Some(&b'T') && kept_at.ends_with('Z'), "RFC 3339 as settled_at is: {kept_at}");
    assert!(!detail.contains_key("settled"), "open until he answers it");
    let item = format!("document:{path}");
    assert_eq!(waits(&record), [(item.as_str(), TITLE, WaitingSource::Document)]);
    let act = record.waiting_for[0].act.as_ref().expect("an act");
    assert_eq!((act.kind, act.target.as_str()), (ipc::WaitingActKind::Message, id.as_str()));
}

#[tokio::test]
async fn approving_tells_the_job_settles_the_revision_and_the_same_bytes_again_ask_nothing() {
    let rig = rig_placing(None);
    let (id, work, file, path) = one_page(&rig).await;
    let record = Arc::clone(&rig.fleet).answer_document(answer(&id, &path, "1", DocumentHow::Approved, Some("Ship it."))).await.unwrap();

    assert_eq!(
        sent(&rig.rows(&id).await).pop().unwrap(),
        format!("Approved document: {TITLE}, revision 1\nGo on from it as written.\nShip it.")
    );
    let detail = &the_document(&record, &path).unwrap().detail;
    assert_eq!(detail.get("settled").map(String::as_str), Some("approved"), "{detail:?}");
    assert!(detail.get("settled_at").is_some_and(|at| !at.is_empty()), "{detail:?}");
    assert_eq!(detail.get("words").map(String::as_str), Some("Ship it."));
    assert!(waits(&record).is_empty(), "answered, it is off: {:?}", waits(&record));
    assert!(approved_on_newest(detail));

    // The page written again with the same bytes keeps nothing; a second page shows the first was heard.
    written(&rig, &file, "first", "toolu_2").await;
    let other = work.path().join(".armada/documents/other.html");
    written(&rig, &other, "another", "toolu_3").await;
    revision_kept(&rig, &id, &other.to_string_lossy(), "1").await;
    let record = record_of(&rig, &id).await;
    let detail = &the_document(&record, &path).unwrap().detail;
    assert_eq!((detail["revision"].as_str(), detail.get("settled").map(String::as_str)), ("1", Some("approved")));
    assert!(waits(&record).iter().all(|(item, ..)| *item != format!("document:{path}")), "{:?}", waits(&record));
}

#[tokio::test]
async fn a_new_revision_asks_again_and_lifts_a_dismissal() {
    let rig = rig_placing(None);
    let (id, _work, file, path) = one_page(&rig).await;
    let item = format!("document:{path}");
    Arc::clone(&rig.fleet).answer_document(answer(&id, &path, "1", DocumentHow::Approved, Some("Good."))).await.unwrap();

    written(&rig, &file, "second", "toolu_2").await;
    revision_kept(&rig, &id, &path, "2").await;
    let record = record_of(&rig, &id).await;
    let detail = &the_document(&record, &path).unwrap().detail;
    for gone in ["settled", "settled_at", "words"] {
        assert!(!detail.contains_key(gone), "a new revision drops `{gone}`: {detail:?}");
    }
    assert!(!approved_on_newest(detail), "his approval was of the first");
    assert_eq!(waits(&record), [(item.as_str(), TITLE, WaitingSource::Document)], "it asks again");

    let dismiss = ipc::DismissWaiting { session_id: id.clone(), item_id: item.clone(), reason: None, misread: false };
    let after = Arc::clone(&rig.fleet).dismiss_waiting(dismiss).await.expect("dismissed");
    assert!(waits(&after).iter().all(|(one, ..)| *one != item), "{:?}", waits(&after));
    written(&rig, &file, "third", "toolu_3").await;
    revision_kept(&rig, &id, &path, "3").await;
    assert_eq!(waits(&record_of(&rig, &id).await), [(item.as_str(), TITLE, WaitingSource::Document)], "revised, it waits again");
}

#[tokio::test]
async fn sending_a_document_back_tells_the_job_his_note_and_where_the_page_is() {
    let rig = rig_placing(None);
    let (id, _work, _file, path) = one_page(&rig).await;
    let Err(Refusal::Unacceptable(empty)) =
        Arc::clone(&rig.fleet).answer_document(answer(&id, &path, "1", DocumentHow::SentBack, Some(" \n"))).await
    else {
        panic!("a send-back needs a note");
    };
    assert_eq!(empty.code, "fleet.document_answer_empty");
    assert!(sent(&rig.rows(&id).await).iter().all(|one| !one.starts_with("Sent back")), "nothing sent");

    let note = "Say what the clock is for.";
    let record = Arc::clone(&rig.fleet)
        .answer_document(answer(&id, &path, "1", DocumentHow::SentBack, Some(&format!("  {note}\n"))))
        .await
        .unwrap();
    assert_eq!(
        sent(&rig.rows(&id).await).pop().unwrap(),
        format!("Sent back document: {TITLE}, revision 1\n{note}\nRevise the page at {path} and write it again.")
    );
    let detail = &the_document(&record, &path).unwrap().detail;
    assert_eq!((detail.get("settled").map(String::as_str), detail.get("words").map(String::as_str)), (Some("sent_back"), Some(note)));
    assert!(!approved_on_newest(detail), "a send-back is not an approval");
}

#[tokio::test]
async fn a_revision_kept_since_he_read_it_is_refused_and_a_settled_or_unknown_one_is_not_open() {
    let rig = rig_placing(None);
    let (id, _work, file, path) = one_page(&rig).await;
    written(&rig, &file, "second", "toolu_2").await;
    revision_kept(&rig, &id, &path, "2").await;
    let told = sent(&rig.rows(&id).await).len();

    let Err(Refusal::IllegalMove(stale)) =
        Arc::clone(&rig.fleet).answer_document(answer(&id, &path, "1", DocumentHow::Approved, None)).await
    else {
        panic!("an answer to the first revision is refused");
    };
    assert_eq!(stale.code, "fleet.document_revised");
    assert!(stale.message.contains(TITLE), "{}", stale.message);
    assert_eq!(sent(&rig.rows(&id).await).len(), told, "the Job is told nothing");

    Arc::clone(&rig.fleet).answer_document(answer(&id, &path, "2", DocumentHow::Approved, None)).await.unwrap();
    let Err(Refusal::Unacceptable(twice)) =
        Arc::clone(&rig.fleet).answer_document(answer(&id, &path, "2", DocumentHow::SentBack, Some("No."))).await
    else {
        panic!("a settled revision takes no second answer");
    };
    assert_eq!(twice.code, "fleet.document_not_open");
    let Err(Refusal::Unacceptable(none)) =
        Arc::clone(&rig.fleet).answer_document(answer(&id, "/r/.armada/documents/none.html", "1", DocumentHow::Approved, None)).await
    else {
        panic!("a page the Job does not hold is refused");
    };
    assert_eq!(none.code, "fleet.document_not_open");
    let Err(Refusal::Unacceptable(unnamed)) =
        Arc::clone(&rig.fleet).answer_document(answer(&SessionId::carried("nobody"), &path, "2", DocumentHow::Approved, None)).await
    else {
        panic!("a Job Fleet does not know is refused");
    };
    assert_eq!(unnamed.code, "fleet.document_answer_unnamed");
}

#[tokio::test]
async fn a_document_kept_before_documents_asked_waits_on_nothing_and_takes_no_answer() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let path = "/r/.armada/documents/old.html";
    let detail: BTreeMap<String, String> =
        [("form", "document"), ("title", "Old"), ("revisions", r#"["/kept/r1.html"]"#)].map(|(k, v)| (k.to_string(), v.to_string())).into();
    rig.fleet
        .report_session(ipc::SessionReport {
            harness: String::from(adapters::HOSTED_HARNESS),
            session_id: id.clone(),
            fact: ipc::SessionFact::Attached {
                attachment: ipc::AttachmentReport { kind: "artifact".into(), target: path.into(), detail },
            },
        })
        .await
        .unwrap();
    assert!(waits(&record_of(&rig, &id).await).is_empty());
    let Err(Refusal::Unacceptable(why)) =
        Arc::clone(&rig.fleet).answer_document(answer(&id, path, "1", DocumentHow::Approved, None)).await
    else {
        panic!("not asked, not answered");
    };
    assert_eq!(why.code, "fleet.document_not_open");
}

#[test]
fn the_rail_reads_an_approval_only_on_the_newest_revision_kept() {
    let detail = |pairs: &[(&str, &str)]| -> BTreeMap<String, String> {
        pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
    };
    let two = r#"["/kept/r1.html","/kept/r2.html"]"#;
    let base = [("form", "document"), ("asked", "true"), ("revisions", two)];
    let with = |more: &[(&str, &str)]| detail(&[&base[..], more].concat());
    assert!(approved_on_newest(&with(&[("revision", "2"), ("settled", "approved")])));
    assert!(!approved_on_newest(&with(&[("revision", "1"), ("settled", "approved")])), "an older revision's approval");
    assert!(!approved_on_newest(&with(&[("revision", "2"), ("settled", "sent_back")])));
    assert!(!approved_on_newest(&with(&[("revision", "2")])), "not answered");
    assert!(!approved_on_newest(&detail(&[("form", "sketch"), ("asked", "true"), ("revisions", two), ("revision", "2"), ("settled", "approved")])));
}
