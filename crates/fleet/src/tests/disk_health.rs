//! What Fleet says of a short disk: `get_health` carries the floors and the free bytes, and a look
//! that ends under the hold floor puts one card in Your turn, which the next look over the floor
//! takes away. The card is the one place Fleet speaks up of its own accord, once reclaiming has
//! not kept up.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use ipc::operator_page::{OperatorAnswer, OperatorAnswerPress, OperatorTurnItem, OperatorUrgency};

use super::operator_sources::on;
use super::session_host::{rig_fitted, rig_placing, Rig};
use crate::headroom::{Bytes, InUse, Machine, Reading};
use crate::operator::keep_the_raises;
use crate::sweeping::{Looked, Seen};

const GIB: u64 = 1024 * 1024 * 1024;

/// A machine whose free space a test moves.
struct Planted(Arc<AtomicU64>);

impl Machine for Planted {
    fn read(&self) -> Option<Reading> {
        Some(Reading::of(InUse::percent(10), InUse::percent(20), Bytes::gibibytes(self.0.load(Ordering::SeqCst))))
    }

    fn disk_free_at(&self, _path: &std::path::Path) -> Option<Bytes> {
        Some(Bytes::gibibytes(self.0.load(Ordering::SeqCst)))
    }
}

async fn page_turn(rig: &Rig) -> Vec<OperatorTurnItem> {
    rig.fleet.operator_page_wire().await.expect("read").turn
}

async fn holds(rig: &Rig) -> Vec<OperatorTurnItem> {
    page_turn(rig).await.into_iter().filter(|one| one.item.id.starts_with("disk-hold:")).collect()
}

/// A look that ended with `free_gib` free on the rig's repository, a pass having left `next`.
fn looked(rig: &Rig, free_gib: u64, next: &[&str]) -> Looked {
    let root = PathBuf::from(&rig.root);
    Looked {
        lines: Vec::new(),
        seen: vec![Seen { next: next.iter().map(|one| root.join(one)).collect(), root, free: Some(free_gib * GIB), freed: None }],
    }
}

async fn the_card(rig: &Rig) -> OperatorTurnItem {
    for _ in 0..500 {
        if let Some(one) = holds(rig).await.into_iter().next() {
            return one;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("no disk card was queued: {:#?}", page_turn(rig).await);
}

fn hold_gib(rig: &Rig) -> u64 {
    rig.fleet.headroom().disk_floor().whole_gibibytes()
}

#[tokio::test]
async fn health_carries_the_disk_with_both_floors_and_the_free_bytes_the_machine_reports() {
    let free = Arc::new(AtomicU64::new(25));
    let planted = Arc::clone(&free);
    let rig = rig_fitted(move |fittings| fittings.machine = Arc::new(Planted(planted)));

    let disk = rig.fleet.health().await.expect("health never refuses").disk.expect("a disk was read");

    assert_eq!(disk.free_bytes, 25 * GIB);
    assert_eq!(disk.hold_floor_bytes, rig.fleet.headroom().disk_floor().count());
    assert_eq!(disk.reclaim_floor_bytes, 100 * GIB, "limits.diskReclaimFloorGib as shipped");
    assert_eq!((disk.last_pass, disk.next_to_drop), (None, Vec::new()), "no look has run");

    free.store(7, Ordering::SeqCst);
    assert_eq!(rig.fleet.health().await.expect("health").disk.expect("a disk").free_bytes, 7 * GIB);
}

#[tokio::test]
async fn health_names_the_last_pass_and_what_the_next_would_drop_as_the_last_look_left_them() {
    let rig = rig_placing(None);
    let mut found = looked(&rig, 40, &["a/target", "b/target"]);
    found.seen[0].freed = Some(6 * GIB);

    rig.fleet.disk_looked(&found).await;

    let disk = rig.fleet.health().await.expect("health").disk.expect("a disk");
    let pass = disk.last_pass.expect("a pass");
    assert_eq!(pass.freed_bytes, 6 * GIB);
    assert!(pass.at.ends_with('Z') && pass.at.starts_with("20"), "RFC 3339: {}", pass.at);
    assert_eq!(disk.next_to_drop, vec![format!("{}/a/target", rig.root), format!("{}/b/target", rig.root)]);

    rig.fleet.disk_looked(&looked(&rig, 150, &[])).await;
    let after = rig.fleet.health().await.expect("health").disk.expect("a disk");
    assert!(after.next_to_drop.is_empty(), "over the floor nothing would be dropped");
    assert_eq!(after.last_pass.map(|pass| pass.freed_bytes), Some(6 * GIB), "the last pass is kept");
}

#[tokio::test]
async fn a_look_under_the_hold_floor_raises_one_disk_card_and_a_second_look_raises_no_other() {
    let rig = rig_placing(None);
    on(&rig).await;
    let next = [".armada/slots/slot-2/crates/x/target", ".armada/slots/slot-5/target", ".armada/slots/slot-1/crates/y/target", ".armada/slots/slot-9/target"];

    rig.fleet.disk_looked(&looked(&rig, 1, &next)).await;
    let one = the_card(&rig).await;
    rig.fleet.disk_looked(&looked(&rig, 1, &next)).await;
    rig.fleet.disk_looked(&looked(&rig, 2, &next)).await;
    tokio::time::sleep(Duration::from_millis(100)).await;

    assert_eq!(holds(&rig).await.len(), 1, "raised once while short");
    assert_eq!(one.item.id, format!("disk-hold:{}", rig.manifest.as_str()));
    assert_eq!(
        one.item.text,
        format!("Disk has 1 GB free, under the {} GB hold. No new Jobs start. Next to drop: slot-2/crates/x/target, slot-5/target, slot-1/crates/y/target.", hold_gib(&rig))
    );
    assert_eq!(one.item.options.iter().map(|option| option.label.as_str()).collect::<Vec<_>>(), ["Drop them", "Leave them"]);
}

#[tokio::test]
async fn a_look_back_over_the_hold_floor_takes_the_card_away_and_a_later_shortage_raises_another() {
    let rig = rig_placing(None);
    on(&rig).await;
    rig.fleet.disk_looked(&looked(&rig, 1, &["a/target"])).await;
    the_card(&rig).await;

    rig.fleet.disk_looked(&looked(&rig, hold_gib(&rig) + 1, &[])).await;
    assert!(holds(&rig).await.is_empty(), "free is back over the floor");

    rig.fleet.disk_looked(&looked(&rig, 1, &["a/target"])).await;
    the_card(&rig).await;
    assert_eq!(holds(&rig).await.len(), 1);
}

#[tokio::test]
async fn the_card_stands_through_the_reconcile_that_drops_an_item_its_list_does_not_hold() {
    let rig = rig_placing(None);
    on(&rig).await;
    rig.fleet.disk_looked(&looked(&rig, 1, &["a/target"])).await;
    the_card(&rig).await;

    keep_the_raises(Arc::clone(&rig.fleet)).await;

    assert_eq!(holds(&rig).await.len(), 1);
}

#[tokio::test]
async fn a_short_look_with_the_operator_off_raises_no_card() {
    let rig = rig_placing(None);

    rig.fleet.disk_looked(&looked(&rig, 1, &["a/target"])).await;
    tokio::time::sleep(Duration::from_millis(100)).await;

    assert!(holds(&rig).await.is_empty());
    let store = rig.fleet.store().lock().await;
    assert!(store.operator_turn_present().expect("read").is_empty());
}

#[tokio::test]
async fn a_short_look_with_nothing_to_drop_says_so_by_leaving_the_sentence_and_the_options_out() {
    let rig = rig_placing(None);
    on(&rig).await;

    rig.fleet.disk_looked(&looked(&rig, 1, &[])).await;

    let one = the_card(&rig).await;
    assert_eq!(one.item.text, format!("Disk has 1 GB free, under the {} GB hold. No new Jobs start.", hold_gib(&rig)));
    assert!(one.item.options.is_empty());
}

/// A build folder of an idle worktree under the rig's repository, with caches to drop.
fn a_build_folder(rig: &Rig, name: &str) -> PathBuf {
    let target = PathBuf::from(&rig.root).join(".armada/worktrees").join(name).join("target");
    for file in ["debug/deps/a.rlib", "debug/.fingerprint/a/lib-a", "debug/incremental/a/s-1/dep-graph.bin"] {
        let file = target.join(file);
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(file, [0u8; 64]).unwrap();
    }
    std::fs::write(target.join("CACHEDIR.TAG"), "Signature: 8a477f597d28d172789f06886806bc55\n").unwrap();
    let long_ago = std::time::SystemTime::now() - Duration::from_secs(10 * 3600);
    touch_all(&target.join("debug"), long_ago);
    target
}

fn touch_all(at: &std::path::Path, then: std::time::SystemTime) {
    if at.is_dir() {
        for entry in std::fs::read_dir(at).unwrap().flatten() {
            touch_all(&entry.path(), then);
        }
    }
    std::fs::File::open(at).unwrap().set_modified(then).unwrap();
}

fn press(item: &OperatorTurnItem, option: &str) -> OperatorAnswerPress {
    OperatorAnswerPress { id: item.id.clone(), answer: OperatorAnswer::Yes, then: None, option: Some(option.to_string()), project: None }
}

#[tokio::test]
async fn drop_them_removes_build_output_the_way_a_pass_would_and_leave_them_removes_nothing() {
    let rig = rig_placing(None);
    on(&rig).await;
    let target = a_build_folder(&rig, "idle");
    rig.fleet.disk_looked(&looked(&rig, 1, &[".armada/worktrees/idle/target"])).await;
    let one = the_card(&rig).await;

    rig.fleet.operator_answered(press(&one, "Leave them")).await.expect("answered");
    assert!(target.join("debug/incremental").exists(), "Leave them takes nothing");

    let again = {
        rig.fleet.disk_looked(&looked(&rig, hold_gib(&rig) + 1, &[])).await;
        rig.fleet.disk_looked(&looked(&rig, 1, &[".armada/worktrees/idle/target"])).await;
        the_card(&rig).await
    };
    rig.fleet.operator_answered(press(&again, "Drop them")).await.expect("answered");

    assert!(!target.join("debug/incremental").exists(), "Drop them takes the caches a pass would");
    assert!(target.join("debug/deps").exists(), "and nothing a pass would not");
}

#[tokio::test]
async fn a_pick_the_card_does_not_offer_is_refused() {
    let rig = rig_placing(None);
    on(&rig).await;
    rig.fleet.disk_looked(&looked(&rig, 1, &["a/target"])).await;
    let one = the_card(&rig).await;

    let refused = rig.fleet.operator_answered(press(&one, "Start a fix")).await.expect_err("not offered");

    assert_eq!(refused.error().code.as_str(), "fleet.operator_answer_not_offered");
}

#[tokio::test]
async fn the_disk_card_is_urgent_as_a_red_main_is() {
    let rig = rig_placing(None);
    on(&rig).await;
    rig.fleet.disk_looked(&looked(&rig, 1, &["a/target"])).await;

    assert_eq!(the_card(&rig).await.urgency, OperatorUrgency::Urgent);
}
