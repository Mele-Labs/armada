//! The claim, written before the code: **a page an earlier build kept at the slot path it was written at moves to
//! Fleet's copy on its next revision, as one page, with what he read of it.** `docs/concepts/document.md`, *Where it
//! is written*.

use store::Holder;

use super::documents::{a_job, page_at_root, record_of, revisions_of, writes};
use super::session_host::{eventually, rig_placing};
use super::tmp::TempDir;

const PAGE: &str = r#"<html><head><title>Right column</title><meta name="armada-review" content="approve"></head><body><p>What he decides.</p></body></html>"#;

#[tokio::test]
async fn a_page_kept_at_its_slot_path_moves_to_fleets_copy_on_its_next_revision_with_what_he_read() {
    let rig = rig_placing(None);
    let (id, at) = a_job(&rig).await;
    let (file, path) = page_at_root(&rig, "right-column.html");
    std::fs::write(&file, PAGE).unwrap();
    writes(&rig, at, &path, "toolu_1", "Write", " +10").await;
    eventually(|| async { revisions_of(&record_of(&rig, &id).await, &path).is_some() }).await;

    // The row as an earlier build kept it: at the slot path, which he had opened.
    let home = rig.fleet.page_home(id.as_str(), &path);
    {
        let mut store = rig.fleet.store().lock().await;
        store.move_page(id.as_str(), &home, &path).unwrap();
        store.keep_artifact_seen(id.as_str(), &path, 1).unwrap();
    }

    // The slot is given back; the Job writes the page again from another.
    std::fs::remove_file(&file).unwrap();
    let other = TempDir::new();
    let moved = other.path().join(".armada/documents/right-column.html");
    std::fs::create_dir_all(moved.parent().unwrap()).unwrap();
    std::fs::write(&moved, PAGE.replace("decides", "decides now")).unwrap();
    let moved_path = moved.to_string_lossy().into_owned();
    writes(&rig, at, &moved_path, "toolu_2", "Write", " +10").await;
    eventually(|| async { revisions_of(&record_of(&rig, &id).await, &moved_path).is_some_and(|kept| kept.len() == 2) }).await;

    let store = rig.fleet.store().lock().await;
    let held = store.attachments_of(&Holder::session(id.as_str())).unwrap();
    let pages: Vec<&str> = held.iter().filter(|one| one.kind == "artifact" && one.target.ends_with("right-column.html")).map(|one| one.target.as_str()).collect();
    assert_eq!(pages, [home.as_str()], "one page, at Fleet's copy");
    let seen = store.artifact_seen(id.as_str()).unwrap();
    assert!(seen.iter().any(|one| one.artifact == home && one.revision == 1), "{seen:?}");
    assert!(!seen.iter().any(|one| one.artifact == path), "{seen:?}");
}
