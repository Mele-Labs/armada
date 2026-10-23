//! A slot seen stranded: its last holder told once, and a rescue Scout started with no press when
//! none could be. The Scout and the pool are `rescuing`'s stand-ins; the Session is a hosted one.

use std::sync::Arc;
use std::time::Duration;

use api::HostedSessions;
use ipc::{
    ManifestId, RescueAct, SessionId, SessionRow, SlotFindingState, SlotHolderKind, StartSession,
    WorktreeSlot,
};
use testkit::FakeWorkProduct;

use super::rescuing::{press, rescuing_on, root, work, Rescued};
use super::session_host::{Shared, StandIn};
use crate::tests::daemon::fittings;
use crate::tests::tmp::TempDir;

fn a_fleet(home: &TempDir) -> Arc<Rescued> {
    let stand_in = Arc::new(StandIn::default());
    Arc::new(
        rescuing_on(home, fittings(home, FakeWorkProduct::changed(&[])))
            .hosting_sessions_on(Arc::new(Shared(stand_in)), Duration::from_secs(600)),
    )
}

async fn a_session(fleet: &Arc<Rescued>) -> String {
    let manifest = fleet
        .repositories()
        .first()
        .expect("served")
        .manifest()
        .id()
        .as_str()
        .to_string();
    fleet
        .start_session(StartSession {
            manifest_id: ManifestId::carried(manifest),
            title: None,
            model: None,
            effort: None,
            mode: None,
            root: None,
            fork: None,
            request: None,
            after: None,
        })
        .await
        .expect("started")
        .id
        .as_str()
        .to_string()
}

async fn thread_of(fleet: &Arc<Rescued>, id: &str) -> Vec<String> {
    let rows = Arc::clone(fleet)
        .get_session(SessionId::carried(id))
        .await
        .unwrap()
        .rows;
    rows.into_iter()
        .filter_map(|row| match row {
            SessionRow::Message { text, .. } => Some(text),
            _ => None,
        })
        .collect()
}

async fn notices_of(fleet: &Arc<Rescued>, id: &str) -> Vec<Option<String>> {
    let rows = Arc::clone(fleet).get_session(SessionId::carried(id)).await.unwrap().rows;
    rows.into_iter()
        .filter_map(|row| match row {
            SessionRow::Message { notice, .. } => Some(notice),
            _ => None,
        })
        .collect()
}

fn strand(home: &TempDir, fleet: &Rescued, slot: u32, branch: &str, after: Option<&str>) {
    let (root, diff) = (root(home), "+fn parse() {}\n");
    match after {
        Some(session) => {
            fleet
                .vcs()
                .strand_slot_after(&root, slot, session, work(Some(branch)), diff)
        }
        None => fleet.vcs().strand_slot(&root, slot, work(Some(branch)), diff),
    }
    let src = std::path::PathBuf::from(adapter_traits::slot_path(&root, slot)).join("src");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(src.join("parser.rs"), "fn parse() {}\n").unwrap();
    std::fs::write(src.join("lexer.rs"), "fn lex() {}\n").unwrap();
}

async fn slot(fleet: &Rescued, number: u32) -> WorktreeSlot {
    fleet
        .worktrees_live()
        .await
        .expect("read")
        .slots
        .into_iter()
        .find(|one| one.slot == number)
        .expect("the slot")
}

/// The Finding of slot `number` once its Scout has ended.
async fn finding_of(fleet: &Rescued, number: u32) -> ipc::SlotFinding {
    for _ in 0..1000 {
        if let Some(finding) = slot(fleet, number).await.rescue {
            if finding.state != SlotFindingState::Reading {
                return finding;
            }
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("the Finding never ended");
}

/// How many Scouts have been started: the stand-in logs the turn each one was handed.
fn scouts_started(home: &TempDir) -> usize {
    std::fs::read_to_string(home.path().join("stand-in/turns.log"))
        .map(|log| log.lines().count())
        .unwrap_or(0)
}

/// **A Session that held a stranded slot and is still open is told once.** The message names the
/// slot, the branch, what it holds and how to give it back; no Scout starts, and a snapshot read
/// before the telling, or a Fleet that restarted, does not tell it again.
#[tokio::test]
async fn an_open_session_that_held_a_stranded_slot_is_told_once() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    let session = a_session(&fleet).await;
    strand(&home, &fleet, 2, "fleet/half-done", Some(&session));

    let before = fleet.worktrees_live().await.expect("read");
    let work = slot(&fleet, 2).await.stranded.expect("stranded");
    let holder = work.last_holder.expect("the Session is named");
    assert_eq!(
        (holder.kind, holder.id.as_str(), holder.open),
        (SlotHolderKind::Session, session.as_str(), true)
    );
    assert_eq!(work.told, None);

    fleet.strandings_attended(&before).await;

    let said = thread_of(&fleet, &session).await;
    assert_eq!(said.len(), 1, "{said:?}");
    assert!(said[0].contains("slot-2 on branch fleet/half-done"), "{said:?}");
    assert!(said[0].contains("armada worktree release"), "{said:?}");
    assert_eq!(notices_of(&fleet, &session).await, vec![Some(String::from("slot_stranded"))]);
    let tell = fleet.store().lock().await.tell_of(&session).unwrap().expect("the tell is kept for the refusal reader");
    assert_eq!(tell.kind, "stranded_slot");
    let after = slot(&fleet, 2).await;
    assert!(
        after.stranded.expect("stranded").told.is_some(),
        "the wire says it was told"
    );
    assert_eq!(after.rescue, None, "an open holder is not read over");

    fleet.strandings_attended(&before).await;
    fleet
        .strandings_attended(&fleet.worktrees_live().await.unwrap())
        .await;
    assert_eq!(
        thread_of(&fleet, &session).await.len(),
        1,
        "told once a stranding"
    );
    assert_eq!(scouts_started(&home), 0);
}

/// **A slot nobody can be told about is read without a press, once.** The Finding is the one a
/// press of Rescue keeps, and a pass after it started another does not read again.
#[tokio::test]
async fn a_slot_whose_holder_is_gone_gets_its_scout_once() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    strand(&home, &fleet, 2, "fleet/half-done", None);
    assert_eq!(
        slot(&fleet, 2).await.stranded.expect("stranded").last_holder,
        None
    );

    fleet
        .strandings_attended(&fleet.worktrees_live().await.unwrap())
        .await;
    let found = finding_of(&fleet, 2).await;
    assert_eq!(found.state, SlotFindingState::Answered);
    assert_eq!(scouts_started(&home), 1);

    fleet
        .strandings_attended(&fleet.worktrees_live().await.unwrap())
        .await;
    assert_eq!(scouts_started(&home), 1, "once a stranding");
}

/// A Session the ledger no longer holds live is gone, so it is not told and its slot is read.
#[tokio::test]
async fn a_session_that_is_gone_is_not_told_and_its_slot_is_read() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    strand(
        &home,
        &fleet,
        2,
        "fleet/half-done",
        Some("a-session-fleet-never-hosted"),
    );
    let holder = slot(&fleet, 2)
        .await
        .stranded
        .unwrap()
        .last_holder
        .expect("named");
    assert!(!holder.open);

    fleet
        .strandings_attended(&fleet.worktrees_live().await.unwrap())
        .await;

    finding_of(&fleet, 2).await;
    assert_eq!(scouts_started(&home), 1);
}

/// A person who pressed Rescue first has the Scout they asked for and no second one starts.
#[tokio::test]
async fn a_slot_already_read_on_a_press_is_not_read_again() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    strand(&home, &fleet, 2, "fleet/half-done", None);
    Arc::clone(&fleet)
        .rescue_slot(press(RescueAct::Start), None)
        .await
        .expect("started");
    finding_of(&fleet, 2).await;

    fleet
        .strandings_attended(&fleet.worktrees_live().await.unwrap())
        .await;

    assert_eq!(scouts_started(&home), 1);
}

/// Several stranded slots are read one at a time, so a Fleet coming up on a pool of them does not
/// start a model call for each at once.
#[tokio::test]
async fn one_scout_reads_at_a_time() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    // The stand-in waits on a branch naming `slowly`.
    strand(&home, &fleet, 1, "fleet/slowly", None);
    strand(&home, &fleet, 2, "fleet/half-done", None);

    fleet
        .strandings_attended(&fleet.worktrees_live().await.unwrap())
        .await;
    assert_eq!(
        slot(&fleet, 1).await.rescue.expect("read").state,
        SlotFindingState::Reading
    );
    assert_eq!(
        slot(&fleet, 2).await.rescue,
        None,
        "waits for the one reading"
    );

    let mut stop = press(RescueAct::Stop);
    stop.slot = 1;
    Arc::clone(&fleet)
        .rescue_slot(stop, None)
        .await
        .expect("stopped");
    assert_eq!(finding_of(&fleet, 1).await.state, SlotFindingState::Stopped);
    fleet
        .strandings_attended(&fleet.worktrees_live().await.unwrap())
        .await;
    assert_eq!(finding_of(&fleet, 2).await.state, SlotFindingState::Answered);
}
