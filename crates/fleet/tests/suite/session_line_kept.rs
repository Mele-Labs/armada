//! The line for a slot is kept in the store as Jobs join and leave it, and a Fleet that restarts
//! reads it back in the same order, the person asked again.

use std::sync::Arc;

use api::HostedSessions;
use ipc::WaitingSource;

use super::session_queue::{close, queued_and_idle, with_slot_one_held};

#[tokio::test]
async fn a_line_kept_across_a_restart_is_served_in_the_order_it_was_in() {
    let (rig, _holder) = with_slot_one_held().await;
    let (first, _) = queued_and_idle(&rig, "first").await;
    let (second, _) = queued_and_idle(&rig, "second").await;
    let (gone, _) = queued_and_idle(&rig, "gone").await;
    close(&rig, &gone).await;

    let kept: Vec<(String, i64)> = rig.fleet.store().lock().await.slot_lines().expect("read").into_iter().map(|one| (one.session_id, one.place)).collect();
    assert_eq!(kept, [(first.as_str().to_string(), 0), (second.as_str().to_string(), 1)], "joined in order, and an ended Job left it");

    let manifest = rig.manifest.as_str().to_string();
    let rig = rig.restarted(|_| {});
    assert_eq!(rig.fleet.hosts().slot_queue().ahead_of(&manifest, second.as_str()), 0, "a restarted Fleet holds no line in memory");
    rig.fleet.slot_lines_restored().await;

    let queue = rig.fleet.hosts().slot_queue();
    assert_eq!((queue.ahead_of(&manifest, first.as_str()), queue.ahead_of(&manifest, second.as_str())), (0, 1));
    let asked = Arc::clone(&rig.fleet).get_session(second.clone()).await.expect("read").session.waiting_for;
    assert!(asked.iter().any(|one| one.source == WaitingSource::SlotChoice), "the person is asked again: {asked:?}");
}
