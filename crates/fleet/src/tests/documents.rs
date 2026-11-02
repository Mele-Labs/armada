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
    assert_eq!(keep(&folder, b"one", "html").unwrap().map(|kept| kept.len()), Some(1));
    assert_eq!(keep(&folder, b"one", "html").unwrap(), None);
    let kept = keep(&folder, b"two", "html").unwrap().expect("changed");
    assert_eq!(kept, revisions_in(&folder, "html"));
    assert_eq!(kept.len(), 2);
    assert_eq!(std::fs::read(&kept[0]).unwrap(), b"one");
    assert_eq!(std::fs::read(&kept[1]).unwrap(), b"two");
    // Back to what the first held is still a change from the newest.
    assert_eq!(keep(&folder, b"one", "html").unwrap().map(|kept| kept.len()), Some(3));
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
async fn a_page_changed_outside_write_and_edit_is_kept_when_the_turn_ends() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    rig.send(&id, "write it up").await;
    rig.stand_in.init(0);
    // The page is under the session's own directory, which is what the turn's end reads.
    assert_eq!(record_of(&rig, &id).await.cwd, rig.root);
    let file = std::path::Path::new(&rig.root).join(".armada/documents/notes/lease.html");
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    let path = file.to_string_lossy().into_owned();

    std::fs::write(&file, page("Lease expiry", &[])).unwrap();
    writes(&rig, 0, &path, "toolu_1", "Write", " +10").await;
    eventually(|| async { revisions_of(&record_of(&rig, &id).await, &path).is_some_and(|kept| kept.len() == 1) }).await;

    // A script run through Bash changes the page: no Write or Edit names it, and the turn's end keeps it.
    std::fs::write(&file, page("Lease expiry, rewritten", &[])).unwrap();
    rig.stand_in.says(
        0,
        vec![
            DroneEvent::Called { tool: "Bash".into(), call: "toolu_2".into(), detail: CallDetail::of("python3 rewrite.py") },
            DroneEvent::Answered { call: "toolu_2".into(), failed: false },
        ],
    );
    rig.stand_in.finishes(0, "Rewritten.");
    eventually(|| async { revisions_of(&record_of(&rig, &id).await, &path).is_some_and(|kept| kept.len() == 2) }).await;
    let kept = revisions_of(&record_of(&rig, &id).await, &path).unwrap();
    assert!(std::fs::read_to_string(&kept[1]).unwrap().contains("rewritten"));

    // A turn that leaves the page as it was keeps no third: the next change is the third.
    rig.send(&id, "anything else?").await;
    rig.stand_in.finishes(0, "No.");
    eventually(|| async { matches!(rig.turn(&id).await, ipc::SessionTurn::Idle) }).await;
    std::fs::write(&file, page("Lease expiry, third", &[])).unwrap();
    rig.send(&id, "once more").await;
    rig.stand_in.finishes(0, "Done.");
    eventually(|| async { revisions_of(&record_of(&rig, &id).await, &path).is_some_and(|kept| kept.len() == 3) }).await;
    let kept = revisions_of(&record_of(&rig, &id).await, &path).unwrap();
    assert!(std::fs::read_to_string(&kept[1]).unwrap().contains("rewritten"));
    assert!(std::fs::read_to_string(&kept[2]).unwrap().contains("third"));
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

/// A Job at the rig's root with its process up, and the index of that process.
async fn a_job(rig: &Rig) -> (SessionId, usize) {
    let id = rig.start().await;
    rig.send(&id, "write it up").await;
    let at = process_at(rig, &id).await;
    rig.stand_in.init(at);
    (id, at)
}

async fn process_at(rig: &Rig, id: &SessionId) -> usize {
    eventually(|| async { rig.stand_in.starts().iter().any(|one| one.session == id.as_str()) }).await;
    rig.stand_in.starts().iter().position(|one| one.session == id.as_str()).unwrap()
}

/// A page under the rig's root, its folder made, and its path.
fn page_at_root(rig: &Rig, name: &str) -> (std::path::PathBuf, String) {
    let file = std::path::Path::new(&rig.root).join(".armada/documents").join(name);
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    let path = file.to_string_lossy().into_owned();
    (file, path)
}

/// Ends the Job's turn and waits out the sweep it starts: pages are read in name order, so a new page
/// named last is kept once every page before it was read.
async fn turn_swept(rig: &Rig, id: &SessionId, at: usize, last: &str) {
    let (file, path) = page_at_root(rig, last);
    std::fs::write(&file, page(last, &[])).unwrap();
    rig.stand_in.finishes(at, "Read it.");
    eventually(|| async { revisions_of(&record_of(rig, id).await, &path).is_some() }).await;
}

/// Job `old` handed on, once it has ended, and the new Job with its process up.
async fn handed_on(rig: &Rig, old: &SessionId) -> (SessionId, usize) {
    use api::Sessions;
    rig.fleet.handed_on(old.as_str(), None).await;
    eventually(|| async { record_of(rig, old).await.state == ipc::SessionState::Ended }).await;
    let listed = rig.fleet.list_sessions(None, None, None).await.unwrap().sessions;
    let new = listed.into_iter().find(|one| one.id != *old).expect("the new job").id;
    let at = process_at(rig, &new).await;
    rig.stand_in.init(at);
    (new, at)
}

fn waits_on(record: &ipc::SessionRecord, path: &str) -> bool {
    record.waiting_for.iter().any(|one| one.id == format!("document:{path}"))
}

#[tokio::test]
async fn a_page_handed_on_keeps_its_revisions_and_answer_until_a_real_edit_asks_again() {
    use api::Sessions;
    let rig = rig_placing(None);
    let (old, at) = a_job(&rig).await;
    let (file, path) = page_at_root(&rig, "lease.html");
    std::fs::write(&file, page("Lease", &[("Clock", GOOD)])).unwrap();
    writes(&rig, at, &path, "toolu_1", "Write", " +10").await;
    eventually(|| async { revisions_of(&record_of(&rig, &old).await, &path).is_some_and(|kept| kept.len() == 1) }).await;
    std::fs::write(&file, page("Lease, second", &[("Clock", GOOD)])).unwrap();
    writes(&rig, at, &path, "toolu_2", "Write", " +10").await;
    eventually(|| async { revisions_of(&record_of(&rig, &old).await, &path).is_some_and(|kept| kept.len() == 2) }).await;
    let approve = super::document_answers::answer(&old, &path, "2", ipc::DocumentHow::Approved, None);
    std::sync::Arc::clone(&rig.fleet).answer_document(approve).await.unwrap();

    let (new, at) = handed_on(&rig, &old).await;

    // Its first turn only read the page: it holds the second revision, answered, and nothing asks.
    turn_swept(&rig, &new, at, "zz.html").await;
    let record = record_of(&rig, &new).await;
    let row = record.attachments.iter().find(|one| one.target == path).expect("the page came with the job");
    assert_eq!(row.detail.get("revision").map(String::as_str), Some("2"), "{:?}", row.detail);
    assert_eq!(row.detail.get("settled").map(String::as_str), Some("approved"), "{:?}", row.detail);
    assert!(!waits_on(&record, &path), "{:?}", record.waiting_for);

    // A real edit is the third revision of the same history, and asks again.
    std::fs::write(&file, page("Lease, third", &[("Clock", GOOD)])).unwrap();
    writes(&rig, at, &path, "toolu_3", "Write", " +10").await;
    eventually(|| async { revisions_of(&record_of(&rig, &new).await, &path).is_some_and(|kept| kept.len() == 3) }).await;
    let record = record_of(&rig, &new).await;
    let kept = revisions_of(&record, &path).unwrap();
    assert!(std::fs::read_to_string(&kept[1]).unwrap().contains("second"));
    assert!(std::fs::read_to_string(&kept[2]).unwrap().contains("third"));
    assert!(!record.attachments.iter().find(|one| one.target == path).unwrap().detail.contains_key("settled"));
    assert!(waits_on(&record, &path), "{:?}", record.waiting_for);
}

#[tokio::test]
async fn a_page_another_job_left_in_the_checkout_is_not_raised_until_this_job_changes_it() {
    let rig = rig_placing(None);
    let (first, at) = a_job(&rig).await;
    let (file, path) = page_at_root(&rig, "lease.html");
    std::fs::write(&file, page("Lease", &[])).unwrap();
    writes(&rig, at, &path, "toolu_1", "Write", " +10").await;
    eventually(|| async { revisions_of(&record_of(&rig, &first).await, &path).is_some() }).await;

    let (other, at) = a_job(&rig).await;
    turn_swept(&rig, &other, at, "zz.html").await;
    assert_eq!(revisions_of(&record_of(&rig, &other).await, &path), None, "the first job's page, as it left it");

    // A script of this Job's changes it: no Write names it, and the turn's end keeps it as this Job's.
    std::fs::write(&file, page("Lease, changed", &[])).unwrap();
    rig.send(&other, "change it").await;
    rig.stand_in.finishes(at, "Changed.");
    eventually(|| async { revisions_of(&record_of(&rig, &other).await, &path).is_some_and(|kept| kept.len() == 1) }).await;
    assert!(waits_on(&record_of(&rig, &other).await, &path));
}

#[tokio::test]
async fn a_page_handed_on_unanswered_still_asks_and_one_dismissed_stays_dismissed() {
    use api::HostedSessions;
    let rig = rig_placing(None);
    let (old, at) = a_job(&rig).await;
    let (asks, asks_path) = page_at_root(&rig, "asks.html");
    std::fs::write(&asks, page("Asks", &[])).unwrap();
    writes(&rig, at, &asks_path, "toolu_1", "Write", " +10").await;
    let (dismissed, dismissed_path) = page_at_root(&rig, "dismissed.html");
    std::fs::write(&dismissed, page("Dismissed", &[])).unwrap();
    writes(&rig, at, &dismissed_path, "toolu_2", "Write", " +10").await;
    eventually(|| async { revisions_of(&record_of(&rig, &old).await, &dismissed_path).is_some() }).await;
    let dismiss = ipc::DismissWaiting { session_id: old.clone(), item_id: format!("document:{dismissed_path}"), reason: None, misread: false };
    std::sync::Arc::clone(&rig.fleet).dismiss_waiting(dismiss).await.expect("dismissed");

    let (new, at) = handed_on(&rig, &old).await;
    turn_swept(&rig, &new, at, "zz.html").await;

    let record = record_of(&rig, &new).await;
    assert!(waits_on(&record, &asks_path), "{:?}", record.waiting_for);
    assert!(!waits_on(&record, &dismissed_path), "{:?}", record.waiting_for);
    // The old Job keeps its own files: the new Job reads copies under its own folder.
    let (theirs, ours) = (revisions_of(&record_of(&rig, &old).await, &asks_path).unwrap(), revisions_of(&record, &asks_path).unwrap());
    assert_ne!(theirs, ours);
    assert_eq!(std::fs::read(&theirs[0]).unwrap(), std::fs::read(&ours[0]).unwrap());
}

#[tokio::test]
async fn a_write_is_kept_even_with_the_bytes_another_job_left() {
    let rig = rig_placing(None);
    let (first, at) = a_job(&rig).await;
    let (file, path) = page_at_root(&rig, "lease.html");
    std::fs::write(&file, page("Lease", &[])).unwrap();
    writes(&rig, at, &path, "toolu_1", "Write", " +10").await;
    eventually(|| async { revisions_of(&record_of(&rig, &first).await, &path).is_some() }).await;

    let (other, at) = a_job(&rig).await;
    writes(&rig, at, &path, "toolu_2", "Write", " +10").await;
    eventually(|| async { revisions_of(&record_of(&rig, &other).await, &path).is_some_and(|kept| kept.len() == 1) }).await;
}
