//! A hosted session marking done a note Bridge sent it, wherever under the served root Bridge
//! wrote it, and nobody else's. `docs/concepts/session.md`.

use std::path::{Path, PathBuf};

use api::Sessions;
use ipc::{MarkAnnotationDone, SessionId};

use super::session_host::rig_placing;

const OTHERS: &str = "0b0b0b0b-0000-0000-0000-000000000000";
const WRITTEN: &str = "2026-10-15T09:00:00.000Z";

fn mark(id: &SessionId, note_id: &str) -> MarkAnnotationDone {
    MarkAnnotationDone { note_id: note_id.to_string(), session_id: Some(id.clone()) }
}

/// A note in the preview's checkout, as Bridge run from the preview writes it.
fn note_in_preview(root: &str, note_id: &str, sent_to: &str) -> PathBuf {
    let dir = Path::new(root).join(".armada/preview/.armada/annotations");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(format!("{note_id}.json"));
    let note = format!(
        "{{\n  \"id\": \"{note_id}\",\n  \"status\": \"open\",\n  \"sent\": {{\n    \"at\": \"{WRITTEN}\",\n    \
         \"sessionId\": \"{sent_to}\",\n    \"title\": \"Fix the rail\"\n  }},\n  \"text\": \"Too tight\",\n  \
         \"component\": null,\n  \"owners\": [\n    \"Rail\"\n  ],\n  \"createdAt\": \"{WRITTEN}\",\n  \
         \"updatedAt\": \"{WRITTEN}\"\n}}\n"
    );
    std::fs::write(&path, note).unwrap();
    path
}

#[tokio::test]
async fn a_note_sent_to_the_session_in_the_preview_is_marked_done_and_keeps_the_rest() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let path = note_in_preview(&rig.root, "mfz1-abc", id.as_str());
    let before = std::fs::read_to_string(&path).unwrap();

    let marked = rig.fleet.mark_annotation_done(None, mark(&id, "mfz1-abc")).await.unwrap();

    assert_eq!(Path::new(&marked.path), path);
    let after = std::fs::read_to_string(&path).unwrap();
    let stamped = after
        .lines()
        .find(|line| line.trim_start().starts_with("\"updatedAt\""))
        .expect("updatedAt")
        .to_string();
    assert_ne!(stamped, format!("  \"updatedAt\": \"{WRITTEN}\""), "{after}");
    let expected = before
        .replace("\"status\": \"open\"", "\"status\": \"done\"")
        .replace(&format!("  \"updatedAt\": \"{WRITTEN}\""), &stamped);
    assert_eq!(after, expected);
}

#[tokio::test]
async fn a_note_sent_to_another_session_is_refused_and_left_as_it_was() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let path = note_in_preview(&rig.root, "mfz1-abc", OTHERS);
    let before = std::fs::read(&path).unwrap();

    let refused = rig.fleet.mark_annotation_done(None, mark(&id, "mfz1-abc")).await.unwrap_err();

    assert!(refused.error().message.contains("not sent to this job"), "{refused:?}");
    assert_eq!(std::fs::read(&path).unwrap(), before);
}

#[tokio::test]
async fn an_id_that_leaves_the_annotations_directory_is_refused() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    // Where `../x` from the preview's annotations would land, sent to this session.
    let dir = Path::new(&rig.root).join(".armada/preview/.armada");
    std::fs::create_dir_all(&dir).unwrap();
    let escaped = dir.join("x.json");
    let note = format!("{{\"id\": \"x\", \"status\": \"open\", \"sent\": {{\"sessionId\": \"{}\"}}}}", id.as_str());
    std::fs::write(&escaped, &note).unwrap();

    let refused = rig.fleet.mark_annotation_done(None, mark(&id, "../x")).await.unwrap_err();

    assert!(refused.error().message.contains("not a note id"), "{refused:?}");
    assert_eq!(std::fs::read_to_string(&escaped).unwrap(), note);
}

#[tokio::test]
async fn a_note_nowhere_is_refused_naming_where_it_looked() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let refused = rig.fleet.mark_annotation_done(None, mark(&id, "mfz1-gone")).await.unwrap_err();
    let said = &refused.error().message;
    assert!(said.contains(".armada/preview/.armada/annotations"), "{said}");
}
