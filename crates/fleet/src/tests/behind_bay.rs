//! A Session whose slot is far behind its base is told once to merge it, and again only after the
//! slot has caught up and fallen behind again. The readings are the pool's own list with `behind`
//! set by hand, since the fake pool counts nothing; the Session is a hosted one.

use std::sync::Arc;
use std::time::Duration;

use api::HostedSessions;
use ipc::{
    ManifestId, SessionId, SessionRow, SlotHolding, StartSession, WorktreeSlot, WorktreesHeld,
};
use testkit::FakeWorkProduct;

use super::rescuing::{rescuing_on, Rescued};
use super::session_host::{Shared, StandIn};
use crate::tests::daemon::fittings;
use crate::tests::tmp::TempDir;

async fn thread_of(fleet: &Arc<Rescued>, id: &str) -> Vec<String> {
    Arc::clone(fleet)
        .get_session(SessionId::carried(id))
        .await
        .unwrap()
        .rows
        .into_iter()
        .filter_map(|row| match row {
            SessionRow::Message { text, .. } => Some(text),
            _ => None,
        })
        .collect()
}

/// The pool as read, with one slot held as `held` and `behind` commits behind its base.
async fn pool_with(
    fleet: &Rescued,
    held: SlotHolding,
    behind: Option<u32>,
) -> (WorktreesHeld, WorktreeSlot) {
    let mut slot = fleet.worktrees_live().await.unwrap().slots.remove(0);
    slot.held = held;
    slot.behind = behind;
    slot.branch = Some(String::from("armada/x"));
    (WorktreesHeld { slots: vec![slot.clone()] }, slot)
}

/// **Told once at the threshold, and not again until it has caught up and fallen behind again.**
/// A slot just under the threshold is not told, and neither is one no Session holds.
#[tokio::test]
async fn a_session_far_behind_is_told_once_until_it_catches_up() {
    let home = TempDir::new();
    let stand_in = Arc::new(StandIn::default());
    let fleet = Arc::new(
        rescuing_on(&home, fittings(&home, FakeWorkProduct::changed(&[])))
            .hosting_sessions_on(Arc::new(Shared(stand_in)), Duration::from_secs(600)),
    );
    let manifest = fleet.repositories().first().expect("served").manifest().id().as_str().to_string();
    let session = fleet
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
        .to_string();
    let held = || SlotHolding::HostedSession {
        session_id: session.clone(),
        session_title: None,
    };

    let (pool, slot) = pool_with(&fleet, held(), Some(19)).await;
    fleet.behind_bays_told(&pool).await;
    assert_eq!(thread_of(&fleet, &session).await.len(), 0, "19 is under the threshold");

    let (pool, _) = pool_with(&fleet, SlotHolding::Free, Some(684)).await;
    fleet.behind_bays_told(&pool).await;
    assert_eq!(thread_of(&fleet, &session).await.len(), 0, "no Session holds it");

    let (pool, _) = pool_with(&fleet, held(), Some(684)).await;
    fleet.behind_bays_told(&pool).await;
    let said = thread_of(&fleet, &session).await;
    assert_eq!(
        said,
        vec![format!(
            "slot-{} (branch armada/x) is 684 commits behind {base}. Merge {base} into your \
             branch before your next push.",
            slot.slot,
            base = slot.base
        )]
    );

    for behind in [684, 700, 20] {
        let (pool, _) = pool_with(&fleet, held(), Some(behind)).await;
        fleet.behind_bays_told(&pool).await;
    }
    assert_eq!(thread_of(&fleet, &session).await.len(), 1, "told once while behind");

    let (pool, _) = pool_with(&fleet, held(), Some(0)).await;
    fleet.behind_bays_told(&pool).await;
    let (pool, _) = pool_with(&fleet, held(), Some(25)).await;
    fleet.behind_bays_told(&pool).await;
    let said = thread_of(&fleet, &session).await;
    assert_eq!(said.len(), 2, "told again after catching up and falling behind: {said:?}");
    assert!(said[1].contains("25 commits behind"), "{said:?}");
}

/// **Slots one Session holds that are behind in the same pass are told in one message.**
#[tokio::test]
async fn slots_behind_in_one_pass_are_told_in_one_message() {
    let home = TempDir::new();
    let stand_in = Arc::new(StandIn::default());
    let fleet = Arc::new(
        rescuing_on(&home, fittings(&home, FakeWorkProduct::changed(&[])))
            .hosting_sessions_on(Arc::new(Shared(stand_in)), Duration::from_secs(600)),
    );
    let manifest = fleet.repositories().first().expect("served").manifest().id().as_str().to_string();
    let session = fleet
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
        .to_string();
    let (_, first) = pool_with(
        &fleet,
        SlotHolding::HostedSession { session_id: session.clone(), session_title: None },
        Some(20),
    )
    .await;
    let mut second = first.clone();
    second.slot = first.slot + 1;
    second.branch = Some(String::from("armada/y"));
    second.behind = Some(21);
    let pool = WorktreesHeld { slots: vec![first.clone(), second.clone()] };

    fleet.behind_bays_told(&pool).await;

    let said = thread_of(&fleet, &session).await;
    assert_eq!(
        said,
        vec![format!(
            "slot-{} (branch armada/x) is 20 commits behind {base}. slot-{} (branch armada/y) is \
             21 commits behind {base}. Merge {base} into your branch before your next push.",
            first.slot,
            second.slot,
            base = first.base
        )]
    );
}
