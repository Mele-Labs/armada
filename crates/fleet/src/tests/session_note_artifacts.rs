//! A note sent to a Job is an artifact of form `note` on it, `note:<id>`, and the Job's close settles
//! it: done where the Job had called `all_done`, returned to no Job where it had not. A note sent to
//! another Job is left alone. `crate::session_host::annotating`.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use api::{HostedSessions, Sessions};
use ipc::{AllDone, CloseSession, MarkAnnotationDone, SendSessionMessage, SendSessionNotes, SessionId, SessionUpload};
use store::{Holder, KeptAttachment};

use super::session_host::{rig_fitted, Rig};
use super::session_notes::{a_repository, note, SAID};

const OTHERS: &str = "0b0b0b0b-0000-0000-0000-000000000000";
const WRITTEN: &str = "2026-10-15T09:00:00.000Z";

/// A note in the served root's own annotations, as Bridge writes it.
fn note_file(root: &str, note_id: &str, sent_to: Option<&str>) -> PathBuf {
    let dir = Path::new(root).join(".armada/annotations");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(format!("{note_id}.json"));
    let sent = sent_to
        .map(|to| format!("  \"sent\": {{\n    \"at\": \"{WRITTEN}\",\n    \"sessionId\": \"{to}\"\n  }},\n"))
        .unwrap_or_default();
    let body = format!(
        "{{\n  \"id\": \"{note_id}\",\n  \"status\": \"open\",\n  \"scope\": \"window\",\n{sent}  \"text\": \"Too tight\",\n  \
         \"component\": \"Rail\",\n  \"element\": {{\n    \"tag\": \"nav\"\n  }},\n  \"screen\": \"usage\",\n  \
         \"updatedAt\": \"{WRITTEN}\"\n}}\n"
    );
    std::fs::write(&path, body).unwrap();
    path
}

async fn artifact(rig: &Rig, id: &SessionId, note_id: &str) -> Option<KeptAttachment> {
    let held = rig.fleet.store().lock().await.attachments_of(&Holder::session(id.as_str())).unwrap();
    held.into_iter().find(|one| one.kind == "artifact" && one.target == format!("note:{note_id}"))
}

async fn status_of(rig: &Rig, id: &SessionId, note_id: &str) -> String {
    let kept = artifact(rig, id, note_id).await.unwrap_or_else(|| panic!("an artifact for {note_id}"));
    kept.detail.get("status").cloned().unwrap_or_default()
}

fn on_disk(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap()
}

async fn closed(rig: &Rig, id: &SessionId) {
    Arc::clone(&rig.fleet).close_session(CloseSession { session_id: id.clone() }).await.unwrap();
}

#[tokio::test]
async fn a_note_sent_through_send_session_note_is_kept_as_an_artifact_with_its_detail() {
    let rig = rig_fitted(|_| {});
    a_repository(&rig);
    let id = rig.start().await;
    let file = note_file(&rig.root, "n-1", Some(id.as_str()));

    Arc::clone(&rig.fleet).send_session_note(note(&id, "Other commands", None)).await.expect("taken");

    let kept = artifact(&rig, &id, "n-1").await.expect("the note is an artifact");
    let shown = |key: &str| kept.detail.get(key).map(String::as_str);
    assert_eq!(shown("form"), Some("note"));
    assert_eq!(shown("note"), Some("n-1"));
    assert_eq!(shown("status"), Some("open"));
    assert_eq!(shown("text"), Some(SAID));
    assert_eq!(shown("title"), Some(SAID));
    assert_eq!(shown("tag"), Some("li"));
    assert_eq!(shown("component"), Some("ShareRows"));
    assert_eq!(shown("address"), Some("http://127.0.0.1:1/?walk=burnBySessionModelTool"));
    assert_eq!(shown("file"), Some(file.display().to_string().as_str()));
}

#[tokio::test]
async fn a_note_sent_where_no_file_is_found_is_still_kept_from_what_was_sent() {
    let rig = rig_fitted(|_| {});
    a_repository(&rig);
    let id = rig.start().await;

    Arc::clone(&rig.fleet).send_session_note(note(&id, "Other commands", None)).await.expect("taken");

    let kept = artifact(&rig, &id, "n-1").await.expect("kept without a file");
    assert_eq!(kept.detail.get("file"), None, "{:?}", kept.detail);
    assert_eq!(kept.detail.get("text").map(String::as_str), Some(SAID));
}

#[tokio::test]
async fn each_note_of_a_batch_is_kept_as_its_own_artifact() {
    let rig = rig_fitted(|_| {});
    a_repository(&rig);
    let id = rig.start().await;
    let (mut first, mut second) = (note(&id, "Other commands", None).note, note(&id, "Zebra", None).note);
    first.note_id = Some("n-first".to_string());
    second.note_id = Some("n-second".to_string());
    second.said = "wrong colour".to_string();

    Arc::clone(&rig.fleet)
        .send_session_notes(SendSessionNotes { session_id: id.clone(), notes: vec![first, second] })
        .await
        .expect("taken");

    let one = artifact(&rig, &id, "n-first").await.expect("the first");
    let two = artifact(&rig, &id, "n-second").await.expect("the second");
    assert_eq!(one.detail.get("text").map(String::as_str), Some(SAID));
    assert_eq!(two.detail.get("text").map(String::as_str), Some("wrong colour"));
    assert_eq!((one.detail["form"].as_str(), two.detail["form"].as_str()), ("note", "note"));
}

#[tokio::test]
async fn a_window_scope_note_named_in_a_plain_message_is_kept_from_its_file() {
    let rig = rig_fitted(|_| {});
    let id = rig.start().await;
    let file = note_file(&rig.root, "w-1", Some(id.as_str()));
    let text = "Take these.\n- Note file: .armada/annotations/w-1.json\n- Note file: .armada/annotations/w-gone.json";

    Arc::clone(&rig.fleet)
        .send_session_message(SendSessionMessage {
            session_id: id.clone(),
            text: text.to_string(),
            attachments: Vec::new(),
            mentions: Vec::new(),
        })
        .await
        .expect("sent");

    let kept = artifact(&rig, &id, "w-1").await.expect("the named note is an artifact");
    assert_eq!(kept.detail.get("form").map(String::as_str), Some("note"));
    assert_eq!(kept.detail.get("text").map(String::as_str), Some("Too tight"));
    assert_eq!(kept.detail.get("tag").map(String::as_str), Some("nav"));
    assert_eq!(kept.detail.get("status").map(String::as_str), Some("open"));
    assert_eq!(kept.detail.get("file"), Some(&file.display().to_string()));
    assert!(artifact(&rig, &id, "w-gone").await.is_none(), "a note with no file is not invented");
}

#[tokio::test]
async fn a_window_scope_note_named_by_its_picture_is_kept_too() {
    let rig = rig_fitted(|_| {});
    let id = rig.start().await;
    note_file(&rig.root, "w-2", Some(id.as_str()));

    Arc::clone(&rig.fleet)
        .send_session_message(SendSessionMessage {
            session_id: id.clone(),
            text: "Look at this.".to_string(),
            attachments: vec![SessionUpload {
                name: "annotation-w-2.png".to_string(),
                media_type: "image/png".to_string(),
                data: "aGVsbG8=".to_string(),
            }],
            mentions: Vec::new(),
        })
        .await
        .expect("sent");

    assert!(artifact(&rig, &id, "w-2").await.is_some());
}

#[tokio::test]
async fn closing_a_job_that_called_all_done_marks_its_notes_done_and_the_artifact_says_so() {
    let rig = rig_fitted(|_| {});
    a_repository(&rig);
    let id = rig.start().await;
    let file = note_file(&rig.root, "n-1", Some(id.as_str()));
    Arc::clone(&rig.fleet).send_session_note(note(&id, "Other commands", None)).await.expect("taken");
    assert_eq!(status_of(&rig, &id, "n-1").await, "open");
    rig.fleet.all_done(None, AllDone { session_id: Some(id.clone()), summary: None }).await.expect("done");

    closed(&rig, &id).await;

    let after = on_disk(&file);
    assert!(after.contains("\"status\": \"done\""), "{after}");
    assert!(after.contains(id.as_str()), "a done note keeps its sent: {after}");
    assert_eq!(status_of(&rig, &id, "n-1").await, "done");
}

#[tokio::test]
async fn closing_a_job_without_all_done_lets_its_notes_go_and_the_artifact_says_returned() {
    let rig = rig_fitted(|_| {});
    a_repository(&rig);
    let id = rig.start().await;
    let file = note_file(&rig.root, "n-1", Some(id.as_str()));
    Arc::clone(&rig.fleet).send_session_note(note(&id, "Other commands", None)).await.expect("taken");

    closed(&rig, &id).await;

    let after = on_disk(&file);
    assert!(after.contains("\"status\": \"open\""), "{after}");
    assert!(!after.contains("\"sent\""), "the note is sent to no Job: {after}");
    assert!(!after.contains(id.as_str()), "{after}");
    assert_eq!(status_of(&rig, &id, "n-1").await, "returned");
}

#[tokio::test]
async fn a_note_found_by_closing_with_no_artifact_yet_gets_one_that_says_how_it_ended() {
    let rig = rig_fitted(|_| {});
    let id = rig.start().await;
    note_file(&rig.root, "old-1", Some(id.as_str()));

    closed(&rig, &id).await;

    assert_eq!(status_of(&rig, &id, "old-1").await, "returned");
}

#[tokio::test]
async fn closing_a_job_leaves_a_note_sent_to_another_job_alone() {
    let rig = rig_fitted(|_| {});
    let id = rig.start().await;
    let theirs = note_file(&rig.root, "t-1", Some(OTHERS));
    let nobodys = note_file(&rig.root, "u-1", None);
    let (theirs_before, nobodys_before) = (on_disk(&theirs), on_disk(&nobodys));
    rig.fleet.all_done(None, AllDone { session_id: Some(id.clone()), summary: None }).await.expect("done");

    closed(&rig, &id).await;

    assert_eq!(on_disk(&theirs), theirs_before);
    assert_eq!(on_disk(&nobodys), nobodys_before);
    assert!(artifact(&rig, &id, "t-1").await.is_none());
    assert!(artifact(&rig, &id, "u-1").await.is_none());
}

#[tokio::test]
async fn marking_a_note_done_sets_its_artifact_to_done() {
    let rig = rig_fitted(|_| {});
    a_repository(&rig);
    let id = rig.start().await;
    note_file(&rig.root, "n-1", Some(id.as_str()));
    Arc::clone(&rig.fleet).send_session_note(note(&id, "Other commands", None)).await.expect("taken");
    assert_eq!(status_of(&rig, &id, "n-1").await, "open");

    rig.fleet
        .mark_annotation_done(None, MarkAnnotationDone { note_id: "n-1".to_string(), session_id: Some(id.clone()) })
        .await
        .expect("marked");

    assert_eq!(status_of(&rig, &id, "n-1").await, "done");
}
