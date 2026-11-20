//! The claim, written before the code: **a page the Job wrote that asks nothing waits on him as one Read item,
//! `read:<path>`, carrying the page as its artifact, until he opens its newest revision.** A new revision raises it
//! again, the Job archiving the page withdraws it, and neither a page that buried its ask nor his own Edit raises one.

use std::sync::Arc;

use api::HostedSessions;
use ipc::{ArtifactBy, ArtifactRef, MarkArtifactSeen, SaveArtifactEdit, SessionId, WaitingItem, WaitingSource};

use super::documents::{a_job, page_at_root, record_of, revisions_of, writes};
use super::page_actions::sent;
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

fn read_items(record: &ipc::SessionRecord) -> Vec<WaitingItem> {
    record.waiting_for.iter().filter(|one| one.id.starts_with("read:")).cloned().collect()
}

async fn opened(rig: &Rig, id: &SessionId, path: &str, revision: u32) {
    rig.fleet.mark_artifact_seen(MarkArtifactSeen { session_id: id.clone(), artifact: path.into(), revision }).await.expect("marked");
}

#[tokio::test]
async fn a_reading_page_waits_as_one_read_item_naming_its_artifact_and_a_page_that_asks_does_not() {
    let rig = rig_placing(None);
    let (id, at) = a_job(&rig).await;
    let (_, path) = page_at_root(&rig, "status.html");
    written(&rig, &id, at, &path, &reading("Status", "Findings."), "toolu_1", 1).await;

    let items = read_items(&record_of(&rig, &id).await);
    assert_eq!(items.len(), 1, "{items:?}");
    let item = &items[0];
    let home = rig.fleet.page_home(id.as_str(), &path);
    assert_eq!(item.id, format!("read:{home}"));
    assert_eq!(item.text, "Read: Status");
    assert_eq!(item.source, WaitingSource::Document);
    assert_eq!(item.artifact.as_deref(), Some(home.as_str()));
    assert!(item.options.is_empty() && !item.approves, "{item:?}");

    // A second revision while it is open refreshes the one item.
    written(&rig, &id, at, &path, &reading("Status, later", "Findings."), "toolu_2", 2).await;
    let items = read_items(&record_of(&rig, &id).await);
    assert_eq!(items.iter().map(|one| one.text.as_str()).collect::<Vec<_>>(), ["Read: Status, later"]);

    // A page that asks waits for his Approve, and is no Read item.
    let (_, asks) = page_at_root(&rig, "design.html");
    let marked = r#"<html><head><title>Design</title><meta name="armada-review" content="approve"></head><body><p>Approve the design.</p><h2>Decision</h2></body></html>"#;
    written(&rig, &id, at, &asks, marked, "toolu_3", 1).await;
    let record = record_of(&rig, &id).await;
    assert!(super::documents::waits_on(&record, &asks), "{:?}", record.waiting_for);
    assert!(read_items(&record).iter().all(|one| one.artifact.as_deref() != Some(rig.fleet.page_home(id.as_str(), &asks).as_str())), "{:?}", record.waiting_for);
}

#[tokio::test]
async fn opening_the_newest_revision_closes_it_telling_the_job_nothing_and_a_new_revision_raises_it_again() {
    let rig = rig_placing(None);
    let (id, at) = a_job(&rig).await;
    let (_, path) = page_at_root(&rig, "status.html");
    written(&rig, &id, at, &path, &reading("Status", "Findings."), "toolu_1", 1).await;
    written(&rig, &id, at, &path, &reading("Status", "More findings."), "toolu_2", 2).await;
    let before = sent(&rig.rows(&id).await).len();

    opened(&rig, &id, &path, 1).await;
    assert_eq!(read_items(&record_of(&rig, &id).await).len(), 1, "an older revision opened leaves it");
    opened(&rig, &id, &path, 2).await;
    assert!(read_items(&record_of(&rig, &id).await).is_empty(), "the newest opened closes it");
    assert_eq!(sent(&rig.rows(&id).await).len(), before, "nothing is sent to the Job");

    written(&rig, &id, at, &path, &reading("Status", "Newer findings."), "toolu_3", 3).await;
    assert_eq!(read_items(&record_of(&rig, &id).await).len(), 1, "a revision after he read it waits again");
}

#[tokio::test]
async fn a_page_that_buried_its_ask_and_his_own_edit_raise_none() {
    let rig = rig_placing(None);
    let (id, at) = a_job(&rig).await;
    let (_, buried) = page_at_root(&rig, "findings.html");
    let words = vec!["word"; 300].join(" ");
    let overloaded = format!(r#"<html><head><title>Findings</title><meta name="armada-review" content="approve"></head><body><p>{words}</p><h2>Decision</h2></body></html>"#);
    written(&rig, &id, at, &buried, &overloaded, "toolu_1", 1).await;
    assert!(read_items(&record_of(&rig, &id).await).is_empty(), "a buried ask goes back to the Job");

    let (_, path) = page_at_root(&rig, "status.html");
    let html = reading("Status", "Findings.");
    written(&rig, &id, at, &path, &html, "toolu_2", 1).await;
    opened(&rig, &id, &path, 1).await;
    let his = html.replace("Findings.", "Findings, as he put them.");
    Arc::clone(&rig.fleet)
        .save_artifact_edit(SaveArtifactEdit { session_id: id.clone(), artifact: path.clone(), revision: 1, text: his })
        .await
        .expect("saved");
    let record = record_of(&rig, &id).await;
    assert_eq!(revisions_of(&record, &path).map(|kept| kept.len()), Some(2));
    assert!(read_items(&record).is_empty(), "his own revision does not wait on him: {:?}", record.waiting_for);
}

#[tokio::test]
async fn the_job_archiving_the_page_withdraws_its_read_item() {
    let rig = rig_placing(None);
    let (id, at) = a_job(&rig).await;
    let (_, path) = page_at_root(&rig, "status.html");
    written(&rig, &id, at, &path, &reading("Status", "Findings."), "toolu_1", 1).await;
    assert_eq!(read_items(&record_of(&rig, &id).await).len(), 1);

    let named = ArtifactRef { artifact: path.clone(), session_id: Some(id.clone()) };
    Arc::clone(&rig.fleet).archive_artifact(None, ArtifactBy::Agent, named).await.expect("archived");
    assert!(read_items(&record_of(&rig, &id).await).is_empty(), "archived, it waits no more");
}
