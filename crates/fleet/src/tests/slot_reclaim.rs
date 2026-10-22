//! A slot whose branch merged: its holder told, and the slot given back when the holder's turn ends
//! with nothing new in it. `docs/concepts/fleet.md`, *Worktree slots*.

use adapter_traits::{FromOutside, RecentlyMerged, SlotAfterMerge};
use ipc::SessionId;

use crate::main_hub::MergedPulled;

use super::session_host::{eventually, rig_placing, Rig};
use super::slot_leases::{leased, By};

const BRANCH: &str = "armada/9";

/// A session idle after a turn, holding slot 3 on `armada/9`, whose pull request has merged and been
/// seen to. The slot has been told once.
///
/// `held` is what the slot holds when the notice's turn ends.
async fn a_session_told_its_slot_is_free(held: SlotAfterMerge) -> (Rig, SessionId) {
    a_session_whose_slot_branch_merged(held, BRANCH).await
}

async fn a_session_whose_slot_branch_merged(held: SlotAfterMerge, branch: &str) -> (Rig, SessionId) {
    let rig = rig_placing(None);
    rig.fleet.vcs().slot_holds_after_merge(3, held);
    let id = rig.start().await;
    rig.send(&id, "do the thing").await;
    rig.stand_in.init(0);
    rig.stand_in.finishes(0, "done");
    eventually(|| async { rig.turn(&id).await == ipc::SessionTurn::Idle }).await;
    // The end of that first turn looks at the session's slots too, behind it: let it finish
    // before there is a slot to find.
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    leased(&rig.root, 3, branch, By::Job(id.as_str()));
    rig.fleet.vcs().main_ci.followed_are(Some(Vec::new()));
    let merged = merged_pull(branch, &rig);
    rig.fleet.sweeping().lock().await.merged.insert(rig.root.clone(), vec![merged]);
    let served = rig.fleet.repositories().served().remove(0);
    rig.fleet.pulls_told(&served).await;
    assert_eq!(rig.fleet.vcs().parked_slots(), Vec::new(), "the notice alone gives nothing back");
    (rig, id)
}

/// The merged listing a visit reads, as a repository with a base would leave it.
fn merged_pull(branch: &str, rig: &Rig) -> MergedPulled {
    MergedPulled {
        pull: RecentlyMerged {
            number: 9,
            title: FromOutside::verbatim("Fix the reader"),
            branch: FromOutside::verbatim(branch),
            url: FromOutside::verbatim("https://forge.invalid/armada/pull/9"),
            author: None,
            merged_at: FromOutside::verbatim(rig.fleet.now().as_str()),
            commit: None,
        },
    }
}

fn said_to_the_session(rig: &Rig) -> Vec<String> {
    let started = rig.stand_in.starts().len();
    (0..started)
        .flat_map(|at| rig.stand_in.process(at).sent.lock().unwrap().clone())
        .filter(|line| line.contains("slot-3"))
        .collect()
}

/// The turn the notice started, run and ended. **Not waited for**: a turn that ends with the slot
/// kept starts another with what keeps it, so the session need not be seen idle.
fn the_turn_ends(rig: &Rig) {
    rig.stand_in.init(0);
    rig.stand_in.finishes(0, "noted");
}

#[tokio::test]
async fn the_holder_of_a_slot_whose_branch_merged_is_told_once() {
    let (rig, _id) = a_session_told_its_slot_is_free(SlotAfterMerge::Idle).await;

    let sent = said_to_the_session(&rig);
    let [line] = sent.as_slice() else { panic!("told once: {sent:?}") };
    assert!(line.contains("slot-3's branch armada/9 merged. Lease it for new work or give it back"), "{line}");

    let served = rig.fleet.repositories().served().remove(0);
    rig.fleet.pulls_told(&served).await;
    assert_eq!(said_to_the_session(&rig).len(), 1, "a second reading tells nothing more");
}

#[tokio::test]
async fn an_idle_turn_after_the_notice_gives_the_slot_back() {
    let (rig, id) = a_session_told_its_slot_is_free(SlotAfterMerge::Idle).await;

    the_turn_ends(&rig);

    eventually(|| async { rig.fleet.vcs().parked_slots() == vec![(3, id.as_str().to_string())] }).await;
}

#[tokio::test]
async fn a_slot_is_kept_when_the_session_started_new_work_in_it() {
    let (rig, _id) = a_session_told_its_slot_is_free(SlotAfterMerge::Moved).await;

    the_turn_ends(&rig);
    // The release runs behind the turn's end: give it the time a release would take.
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;

    assert_eq!(rig.fleet.vcs().parked_slots(), Vec::new());
    assert_eq!(said_to_the_session(&rig).len(), 1, "new work is not told anything");
}

#[tokio::test]
async fn a_dirty_slot_is_kept_and_its_holder_told_what_keeps_it_once() {
    let held = SlotAfterMerge::Dirty(vec!["src/a.rs".into(), "b.rs".into()]);
    let (rig, _id) = a_session_told_its_slot_is_free(held).await;

    the_turn_ends(&rig);
    eventually(|| async { said_to_the_session(&rig).len() == 2 }).await;
    assert!(said_to_the_session(&rig)[1].contains("slot-3 is kept: 2 uncommitted, first src/a.rs."));
    assert_eq!(rig.fleet.vcs().parked_slots(), Vec::new());

    // The turn that word started ends too, with the slot as it was: nothing is said again.
    the_turn_ends(&rig);
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    assert_eq!(said_to_the_session(&rig).len(), 2);
    assert_eq!(rig.fleet.vcs().parked_slots(), Vec::new());
}

#[tokio::test]
async fn unpushed_commits_keep_the_slot_and_are_named() {
    let (rig, _id) = a_session_told_its_slot_is_free(SlotAfterMerge::Unpushed(1)).await;

    the_turn_ends(&rig);

    eventually(|| async { said_to_the_session(&rig).len() == 2 }).await;
    assert!(said_to_the_session(&rig)[1].contains("slot-3 is kept: 1 commit on no remote or base."));
    assert_eq!(rig.fleet.vcs().parked_slots(), Vec::new());
}

#[tokio::test]
async fn a_slot_nobody_told_is_not_given_back_at_a_turn_s_end() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    leased(&rig.root, 3, BRANCH, By::Job(id.as_str()));
    // Merged, and seen to by no reading: the holder has not been told.
    rig.fleet.sweeping().lock().await.merged.insert(rig.root.clone(), vec![merged_pull(BRANCH, &rig)]);
    rig.send(&id, "do the thing").await;
    rig.stand_in.init(0);
    rig.stand_in.finishes(0, "done");
    eventually(|| async { rig.turn(&id).await == ipc::SessionTurn::Idle }).await;
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;

    assert_eq!(rig.fleet.vcs().parked_slots(), Vec::new());
}

/// A review bay sits on a private `review/<holder>` branch and is never a session's to be offered or
/// given back, whatever merged under that name.
#[tokio::test]
async fn a_review_bay_is_neither_told_nor_given_back() {
    let (rig, _id) = a_session_whose_slot_branch_merged(SlotAfterMerge::Idle, "review/s-1234").await;
    assert_eq!(said_to_the_session(&rig), Vec::<String>::new(), "no notice for a review bay");

    the_turn_ends(&rig);
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;

    assert_eq!(rig.fleet.vcs().parked_slots(), Vec::new());
}
