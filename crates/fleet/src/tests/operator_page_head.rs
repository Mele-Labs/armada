//! The claim, written before the code: **the Operator's page names each repository's main and the
//! slots in use for its head, and each Landed row carries the time its pull request merged.**
//! `docs/practices/protocol/the-operators-page-fleet-keeps.md`.

use api::{Next, Sessions};
use ipc::operator_page::OperatorPage;
use ipc::{AttachmentReport, Event, MainCiState, SessionFact, SessionReport};
use store::{MainCi, MainState};

use super::autopilot_decisions_repository::red_main;
use super::backlog::manifest;
use super::session_host::{rig_placing, Rig};
use super::session_plan::{read, running};
use crate::main_ci::{MainChange, MainChanged};

async fn page(rig: &Rig) -> OperatorPage {
    rig.fleet.operator_page_wire().await.expect("read")
}

#[tokio::test]
async fn a_landed_row_carries_the_time_its_pull_request_merged() {
    let rig = rig_placing(None);
    let id = running(&rig).await;
    let harness = read(&rig, &id).await.harness;
    let detail = [("state".to_string(), "merged".to_string()), ("title".to_string(), "Draw the head".to_string())].into_iter().collect();
    let fact = SessionFact::Attached { attachment: AttachmentReport { kind: "pr".into(), target: "3430".into(), detail } };
    rig.fleet.report_session(SessionReport { harness, session_id: id.clone(), fact }).await.expect("attached");
    let merged_at = {
        let store = rig.fleet.store().lock().await;
        store.attachments_of(&store::Holder::session(id.as_str())).expect("rows").into_iter().find(|one| one.kind == "pr").expect("the pull request").changed_at
    };
    tokio::time::sleep(std::time::Duration::from_millis(30)).await;

    let landed = page(&rig).await.digest.landed;
    assert_eq!(landed.iter().map(|one| one.number.as_str()).collect::<Vec<_>>(), ["3430"]);
    assert_eq!(landed[0].at.as_str(), merged_at, "the merge's own time, not the time the page was read");
}

fn main_at(rig: &Rig, state: MainState, commit: &str, red: bool) -> MainCi {
    let at = rig.fleet.now();
    MainCi {
        repository: rig.root.clone(),
        base: "main".into(),
        commit: commit.into(),
        state,
        read_at: at.clone(),
        red_at: red.then(|| at.clone()),
        red_commit: red.then(|| commit.into()),
        decided_commit: Some(commit.into()),
        newer_running: 0,
        unfinished: 0,
        failed: Vec::new(),
        merge: None,
    }
}

#[tokio::test]
async fn the_head_names_each_repositorys_main_red_while_it_is_red_and_green_after() {
    let rig = rig_placing(None);
    let first = page(&rig).await;
    assert_eq!(first.mains.len(), 1, "one entry for the one repository served");
    assert_eq!((first.mains[0].state, first.mains[0].commit.as_deref()), (None, None), "nothing read yet");
    let name = std::path::Path::new(&rig.root).file_name().expect("a name").to_string_lossy().to_string();
    assert_eq!((first.mains[0].manifest_id.as_str(), first.mains[0].repository.as_str()), (manifest(&rig).as_str(), name.as_str()));

    red_main(&rig, &"b".repeat(40)).await;
    let red = page(&rig).await.mains.remove(0);
    assert_eq!((red.state, red.commit.as_deref()), (Some(MainCiState::Red), Some("b".repeat(40).as_str())));
    assert!(red.since.is_some(), "a red says since when");

    let green = main_at(&rig, MainState::Green, &"c".repeat(40), false);
    rig.fleet.store().lock().await.record_main_ci(&green).expect("kept");
    let green = page(&rig).await.mains.remove(0);
    assert_eq!((green.state, green.commit.as_deref(), green.since), (Some(MainCiState::Green), Some("c".repeat(40).as_str()), None));
}

#[tokio::test]
async fn a_red_held_under_a_newer_run_is_still_red_on_the_head() {
    let rig = rig_placing(None);
    let mut held = main_at(&rig, MainState::Running, &"d".repeat(40), true);
    held.red_commit = Some("b".repeat(40));
    held.newer_running = 1;
    rig.fleet.store().lock().await.record_main_ci(&held).expect("kept");

    let main = page(&rig).await.mains.remove(0);

    assert_eq!((main.state, main.commit.as_deref()), (Some(MainCiState::Red), Some("b".repeat(40).as_str())), "the red's own commit");
}

#[tokio::test]
async fn the_head_counts_the_slots_in_use_of_every_slot_in_the_pool() {
    let rig = rig_placing(None);
    let before = page(&rig).await.slots.expect("a pool is read");
    assert_eq!(before.used, 0);
    assert!(before.total >= 1, "{before:?}");

    // The lease, then the fact that says so, which is what a read behind the page waits for.
    rig.fleet.vcs().hold_slot(&rig.root, 2, "01SESSION");
    rig.fleet.session_slot("01SESSION", None, None, 2, true);

    let after = page(&rig).await.slots.expect("a pool is read");
    assert_eq!((after.used, after.total), (1, before.total), "a Job holds one slot");
}

async fn changes(watching: &mut api::Subscription) -> usize {
    let mut found = 0;
    while let Ok(next) = tokio::time::timeout(std::time::Duration::from_millis(300), watching.next()).await {
        match next {
            Some(Next::Send(delivered)) => found += usize::from(matches!(delivered.event, Event::OperatorPageChanged(_))),
            other => panic!("the stream ended or dropped: {other:?}"),
        }
    }
    found
}

#[tokio::test]
async fn main_turning_red_or_green_announces_the_page_once_for_each_and_nothing_else_does() {
    let rig = rig_placing(None);
    red_main(&rig, &"b".repeat(40)).await;
    let red = rig.fleet.store().lock().await.main_ci(&rig.root).unwrap().unwrap();
    let mut watching = rig.fleet.events().subscribe();

    rig.fleet.red_offered(&[MainChanged { change: MainChange::WentRed, now: red.clone() }]).await;
    assert_eq!(changes(&mut watching).await, 1, "the Operator is off, and the pills still moved");

    rig.fleet.red_offered(&[MainChanged { change: MainChange::WentGreen, now: red }]).await;
    assert_eq!(changes(&mut watching).await, 1);

    rig.fleet.red_offered(&[]).await;
    assert_eq!(changes(&mut watching).await, 0, "a reading that changed nothing says nothing");
}
