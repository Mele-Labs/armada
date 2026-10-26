//! The claim, written before the code: **a Job's document, the HTML page it writes to
//! `.armada/documents/`, is kept as a revision each time a write changes it, is one artifact on the
//! Job's ledger whose `revisions` name them oldest first, and a sketch in it that `show_sketch` would
//! refuse is told to the Job once for that revision, while the write is never refused.**

use adapter_traits::{CallDetail, DroneEvent};
use ipc::{SessionId, SessionRow};

use super::session_host::{eventually, rig_placing, Rig};
use super::tmp::TempDir;
use crate::session_host::document_page::{sketches_in, title_of};
use crate::session_host::documents::{keep, revisions_in, sketches_that_will_not_draw};

const GOOD: &str = r#"{"nodes":[{"id":"a","kind":"box","x":0,"y":0,"title":"Bay"}],"edges":[]}"#;
const BAD: &str = r#"{"nodes":[],"edges":[]}"#;

fn page(title: &str, scenes: &[(&str, &str)]) -> String {
    let sketches: String = scenes
        .iter()
        .map(|(name, scene)| {
            format!(r#"<armada-sketch title="{name}"><script type="application/json">{scene}</script></armada-sketch>"#)
        })
        .collect();
    format!("<html><head><title>{title}</title></head><body><h1>Other</h1>{sketches}</body></html>")
}

#[test]
fn a_page_is_titled_by_its_title_then_its_first_heading_then_its_file() {
    assert_eq!(title_of("<title> Lease  &amp; clock </title><h1>x</h1>", "/r/.armada/documents/a.html"), "Lease & clock");
    assert_eq!(title_of("<body><H1 class=a>Why <em>leases</em> end</H1></body>", "/r/.armada/documents/a.html"), "Why leases end");
    assert_eq!(title_of("<title> </title><p>none</p>", "/r/.armada/documents/slot-lease.html"), "slot-lease");
}

#[test]
fn every_sketch_of_a_page_is_found_in_order_by_its_script_else_its_scene_attribute() {
    let html = format!(
        r#"<armada-sketch title="One"><script type="application/json">{GOOD}</script></armada-sketch>
           <p>between</p>
           <armada-sketch scene='{{"nodes":[],"edges":[]}}'></armada-sketch>"#
    );
    let found = sketches_in(&html);
    assert_eq!(found.len(), 2);
    assert_eq!((found[0].title.as_deref(), found[0].scene.as_str()), (Some("One"), GOOD));
    assert_eq!((found[1].title.as_deref(), found[1].scene.as_str()), (None, BAD));
}

#[test]
fn a_sketch_that_draws_tells_nothing_and_one_that_does_not_is_named_with_the_validators_words() {
    let path = "/r/.armada/documents/a.html";
    assert_eq!(sketches_that_will_not_draw(&page("T", &[("Ok", GOOD)]), path), None);
    assert_eq!(sketches_that_will_not_draw("<p>no sketches</p>", path), None);

    let told = sketches_that_will_not_draw(&page("T", &[("Ok", GOOD), ("Empty", BAD)]), path).expect("one is bad");
    assert!(told.starts_with(&format!("In the page {path}, sketch 2 of 2 (\"Empty\") will not draw: ")), "{told}");
    assert!(told.contains("`nodes` is empty"), "{told}");
    assert!(!told.contains('\n') && !told.contains('\u{2014}'), "{told}");

    let broken = sketches_that_will_not_draw(&page("T", &[("Cut", "{\"nodes\": [")]), path).expect("not JSON");
    assert!(broken.contains("the scene is not JSON"), "{broken}");
}

#[test]
fn two_identical_writes_keep_one_revision_and_a_changed_write_adds_the_next() {
    let home = TempDir::new();
    let folder = home.path().join("kept");
    assert_eq!(keep(&folder, b"one").unwrap().map(|kept| kept.len()), Some(1));
    assert_eq!(keep(&folder, b"one").unwrap(), None);
    let kept = keep(&folder, b"two").unwrap().expect("changed");
    assert_eq!(kept, revisions_in(&folder));
    assert_eq!(kept.len(), 2);
    assert_eq!(std::fs::read(&kept[0]).unwrap(), b"one");
    assert_eq!(std::fs::read(&kept[1]).unwrap(), b"two");
    // Back to what the first held is still a change from the newest.
    assert_eq!(keep(&folder, b"one").unwrap().map(|kept| kept.len()), Some(3));
}

async fn writes(rig: &Rig, at: usize, path: &str, call: &str, tool: &str, detail: &str) {
    rig.stand_in.says(
        at,
        vec![
            DroneEvent::Called { tool: tool.into(), call: call.into(), detail: CallDetail::of(&format!("{path}{detail}")) },
            DroneEvent::Answered { call: call.into(), failed: false },
        ],
    );
}

fn revisions_of(record: &ipc::SessionRecord, path: &str) -> Option<Vec<String>> {
    let one = record.attachments.iter().find(|one| one.kind == "artifact" && one.target == path)?;
    assert_eq!(one.detail.get("form").map(String::as_str), Some("document"));
    Some(ipc::decode("revisions", one.detail.get("revisions")?.as_bytes()).expect("a list of paths"))
}

async fn record_of(rig: &Rig, id: &SessionId) -> ipc::SessionRecord {
    use api::Sessions;
    let listed = rig.fleet.list_sessions(None, None, None).await.unwrap().sessions;
    listed.into_iter().find(|one| &one.id == id).expect("listed")
}

fn sketch_notices(rows: &[SessionRow]) -> usize {
    rows.iter()
        .filter(|row| matches!(row, SessionRow::Message { notice: Some(word), .. } if word == "document_sketch"))
        .count()
}

#[tokio::test]
async fn a_documents_writes_are_revisions_of_one_row_and_a_bad_sketch_is_told_once_per_revision() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    rig.send(&id, "write it up").await;
    rig.stand_in.init(0);
    let work = TempDir::new();
    let file = work.path().join(".armada/documents/lease.html");
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    let path = file.to_string_lossy().into_owned();

    // The first write holds a sketch that draws: a row and a revision, and no word to the Job.
    std::fs::write(&file, page("Lease expiry", &[("Clock", GOOD)])).unwrap();
    writes(&rig, 0, &path, "toolu_1", "Write", " +10").await;
    eventually(|| async { revisions_of(&record_of(&rig, &id).await, &path).is_some_and(|kept| kept.len() == 1) }).await;
    let record = record_of(&rig, &id).await;
    let row = record.attachments.iter().find(|one| one.target == path).unwrap();
    assert_eq!(row.detail["title"], "Lease expiry");
    assert_eq!(sketch_notices(&rig.rows(&id).await), 0);

    // The same bytes again keep nothing; an edit that breaks the sketch is the second revision, told once.
    writes(&rig, 0, &path, "toolu_2", "Edit", " +1 -1").await;
    std::fs::write(&file, page("Lease expiry", &[("Clock", GOOD), ("Empty", BAD)])).unwrap();
    writes(&rig, 0, &path, "toolu_3", "MultiEdit", "").await;
    eventually(|| async { revisions_of(&record_of(&rig, &id).await, &path).is_some_and(|kept| kept.len() == 2) }).await;
    eventually(|| async { sketch_notices(&rig.rows(&id).await) == 1 }).await;

    let record = record_of(&rig, &id).await;
    let kept = revisions_of(&record, &path).unwrap();
    assert!(std::fs::read_to_string(&kept[0]).unwrap().contains("Clock"));
    assert!(std::fs::read_to_string(&kept[1]).unwrap().contains("Empty"));
    assert_eq!(record.attachments.iter().filter(|one| one.target == path).count(), 1);

    // A write that fixes it is a third revision and tells nothing more.
    std::fs::write(&file, page("Lease expiry", &[("Clock", GOOD), ("Empty", GOOD)])).unwrap();
    writes(&rig, 0, &path, "toolu_4", "Write", " +12").await;
    eventually(|| async { revisions_of(&record_of(&rig, &id).await, &path).is_some_and(|kept| kept.len() == 3) }).await;
    assert_eq!(sketch_notices(&rig.rows(&id).await), 1);

    // The phone's file route is not opened to a page or to its revisions.
    let revision = revisions_of(&record_of(&rig, &id).await, &path).unwrap().remove(0);
    for asked in [&path, &revision] {
        let read = rig.fleet.shown_file_read(id.as_str(), asked).await;
        assert!(matches!(read, Err(api::Refusal::NotFound(_))), "{asked}");
    }
}

#[tokio::test]
async fn a_write_the_tool_failed_keeps_nothing() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    rig.send(&id, "write it up").await;
    rig.stand_in.init(0);
    let work = TempDir::new();
    let folder = work.path().join(".armada/documents");
    std::fs::create_dir_all(&folder).unwrap();
    let (failed, answered) = (folder.join("failed.html"), folder.join("answered.html"));
    std::fs::write(&failed, page("A", &[])).unwrap();
    std::fs::write(&answered, page("B", &[])).unwrap();
    let (failed, answered) = (failed.to_string_lossy().into_owned(), answered.to_string_lossy().into_owned());
    let called = |path: &str, call: &str| DroneEvent::Called {
        tool: "Write".into(),
        call: call.into(),
        detail: CallDetail::of(&format!("{path} +3")),
    };
    rig.stand_in.says(
        0,
        vec![
            called(&failed, "toolu_1"),
            DroneEvent::Answered { call: "toolu_1".into(), failed: true },
            called(&answered, "toolu_2"),
            DroneEvent::Answered { call: "toolu_2".into(), failed: false },
        ],
    );
    eventually(|| async { revisions_of(&record_of(&rig, &id).await, &answered).is_some() }).await;
    assert_eq!(revisions_of(&record_of(&rig, &id).await, &failed), None);
}
