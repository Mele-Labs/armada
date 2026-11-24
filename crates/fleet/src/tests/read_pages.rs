//! The claim, written before the code: **a page the Job wrote that asks nothing raises no item in his Needs you.** It
//! only informs him, so it is in the thread and his Artifacts, to read there (the owner, 19 Nov 2026: one ask, one
//! item, one place). A page that asks waits for his Approve, and neither a page that buried its ask nor his own Edit
//! raises one. `.claude/decisions/2026-11-19-one-ask-one-item-one-place.md`.

use std::sync::Arc;

use api::HostedSessions;
use ipc::{ArtifactState, SaveArtifactEdit, SessionId};

use super::documents::{a_job, page_at_root, record_of, revisions_of, writes};
use super::session_host::{eventually, rig_placing, Rig};

fn reading(title: &str, body: &str) -> String {
    format!("<html><head><title>{title}</title></head><body><h1>{title}</h1><p>{body}</p></body></html>")
}

/// The Job writes `html` at `path` as call `call`, and Fleet keeps it as revision `revisions`.
async fn written(rig: &Rig, id: &SessionId, at: usize, path: &str, html: &str, call: &str, revisions: usize) {
    std::fs::write(path, html).unwrap();
    writes(rig, at, path, call, "Write", " +1").await;
    eventually(|| async { revisions_of(&record_of(rig, id).await, path).is_some_and(|kept| kept.len() == revisions) }).await;
}

/// Whether any item in his Needs you is about the page at `path`.
fn waits_about(record: &ipc::SessionRecord, path: &str) -> bool {
    record.waiting_for.iter().any(|one| one.artifact.as_deref() == Some(path) || one.id.ends_with(path))
}

#[tokio::test]
async fn a_page_that_asks_nothing_raises_no_item_and_stays_among_his_artifacts_and_a_page_that_asks_waits() {
    let rig = rig_placing(None);
    let (id, at) = a_job(&rig).await;
    let (_, path) = page_at_root(&rig, "status.html");
    written(&rig, &id, at, &path, &reading("Status", "Findings."), "toolu_1", 1).await;

    // The page's id is Fleet's copy of it, wherever the Job wrote it from.
    let home = rig.fleet.page_home(id.as_str(), &path);
    let record = record_of(&rig, &id).await;
    assert!(!waits_about(&record, &home), "it only informs him: {:?}", record.waiting_for);
    let artifacts = rig.fleet.get_job_artifacts(id.clone()).await.unwrap();
    let page = artifacts.iter().find(|one| one.id == home).expect("among his Artifacts");
    assert_eq!(page.state, ArtifactState::Reading);

    written(&rig, &id, at, &path, &reading("Status, later", "Findings."), "toolu_2", 2).await;
    let record = record_of(&rig, &id).await;
    assert!(!waits_about(&record, &home), "a new revision raises none either: {:?}", record.waiting_for);

    // A page that asks waits for his Approve.
    let (_, asks) = page_at_root(&rig, "design.html");
    let marked = r#"<html><head><title>Design</title><meta name="armada-review" content="approve"></head><body><p>Approve the design.</p><h2>Decision</h2></body></html>"#;
    written(&rig, &id, at, &asks, marked, "toolu_3", 1).await;
    let record = record_of(&rig, &id).await;
    assert!(super::documents::waits_on(&record, &asks), "{:?}", record.waiting_for);
}

#[tokio::test]
async fn a_page_that_buried_its_ask_and_his_own_edit_raise_none() {
    let rig = rig_placing(None);
    let (id, at) = a_job(&rig).await;
    let (_, buried) = page_at_root(&rig, "findings.html");
    let words = vec!["word"; 300].join(" ");
    let overloaded = format!(r#"<html><head><title>Findings</title><meta name="armada-review" content="approve"></head><body><p>{words}</p><h2>Decision</h2></body></html>"#);
    written(&rig, &id, at, &buried, &overloaded, "toolu_1", 1).await;
    assert!(!waits_about(&record_of(&rig, &id).await, &rig.fleet.page_home(id.as_str(), &buried)), "a buried ask goes back to the Job");

    let (_, path) = page_at_root(&rig, "status.html");
    let html = reading("Status", "Findings.");
    written(&rig, &id, at, &path, &html, "toolu_2", 1).await;
    let his = html.replace("Findings.", "Findings, as he put them.");
    Arc::clone(&rig.fleet)
        .save_artifact_edit(SaveArtifactEdit { session_id: id.clone(), artifact: path.clone(), revision: 1, text: his })
        .await
        .expect("saved");
    let record = record_of(&rig, &id).await;
    assert_eq!(revisions_of(&record, &path).map(|kept| kept.len()), Some(2));
    assert!(!waits_about(&record, &rig.fleet.page_home(id.as_str(), &path)), "his own revision does not wait on him: {:?}", record.waiting_for);
}
