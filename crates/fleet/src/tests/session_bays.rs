//! A bay held by a hosted Session, or by something it started, is served as
//! that Session, so Bridge draws its title rather than a lease's id or a pid.

use adapter_traits::SlotHeld;

use super::session_host::{rig_placing, Rig};
use crate::leasing::Reading;
use crate::session_host::following::pid_in;
use crate::tests::peer::Placing;

/// The bay `slot` of the first served repository, as it crosses the wire.
async fn on_the_wire(rig: &Rig, slot: u32) -> ipc::SlotHolding {
    let slots = rig.fleet.pool_slots(Reading::Live).await;
    let one = slots
        .iter()
        .find(|one| one.reading.slot == slot)
        .expect("the slot is in the pool");
    crate::wire::worktree_slot(one).held
}

#[tokio::test]
async fn a_lease_in_a_hosted_sessions_id_is_served_as_that_session_with_its_title() {
    let rig = rig_placing(None);
    let id = rig.start_titled(Some("Debug Bridge on Merge line")).await;
    rig.fleet.vcs().hold_slot(&rig.root, 2, id.as_str());

    assert_eq!(
        on_the_wire(&rig, 2).await,
        ipc::SlotHolding::HostedSession {
            session_id: id.as_str().to_string(),
            session_title: Some(String::from("Debug Bridge on Merge line")),
        }
    );
}

#[tokio::test]
async fn a_lease_in_an_id_nothing_knows_is_served_as_a_session_with_no_title() {
    let rig = rig_placing(None);
    rig.fleet.vcs().hold_slot(&rig.root, 2, "01UNKNOWN");

    assert_eq!(
        on_the_wire(&rig, 2).await,
        ipc::SlotHolding::HostedSession {
            session_id: String::from("01UNKNOWN"),
            session_title: None,
        }
    );
}

#[tokio::test]
async fn a_process_the_sessions_agent_started_holds_for_that_session() {
    let peers = Placing::nothing();
    let rig = rig_placing(Some(std::sync::Arc::clone(&peers)));
    let id = rig.start_titled(Some("Debug Bridge on Merge line")).await;
    rig.send(&id, "go").await;
    // The stand-in's agent is pid 1000; a subagent's tool is two below it.
    peers.started(1000, 2002);
    peers.started(2002, 2003);

    for pid in [1000, 2002, 2003] {
        assert_eq!(
            rig.fleet.session_holding_pid(pid, ""),
            Some(id.as_str().to_string()),
            "pid {pid}"
        );
    }
}

#[tokio::test]
async fn a_process_no_hosted_session_started_holds_for_nobody() {
    let peers = Placing::nothing();
    let rig = rig_placing(Some(std::sync::Arc::clone(&peers)));
    let id = rig.start().await;
    rig.send(&id, "go").await;
    peers.started(4000, 4001);

    assert_eq!(rig.fleet.session_holding_pid(4001, ""), None, "a terminal");
    assert_eq!(rig.fleet.session_holding_pid(999_999, ""), None, "a pid long gone");
}

#[tokio::test]
async fn a_session_with_no_agent_yet_is_found_by_the_keeper_serving_its_socket() {
    let peers = Placing::nothing();
    let rig = rig_placing(Some(std::sync::Arc::clone(&peers)));
    let id = rig.start().await;
    let socket = std::path::Path::new(&rig.fleet.host().keepers_dir).join(format!("{}.sock", id.as_str()));
    let listing = format!("3000 armada session-keep {}\n", socket.display());
    peers.started(3000, 3001);

    assert_eq!(
        rig.fleet.session_holding_pid(3001, &listing),
        Some(id.as_str().to_string())
    );
    assert_eq!(rig.fleet.session_holding_pid(3001, ""), None);
}

#[test]
fn the_pid_is_read_out_of_the_way_the_pool_names_a_process() {
    assert_eq!(pid_in("node (pid 2838)"), Some(2838));
    assert_eq!(pid_in("zsh (pid 4120)"), Some(4120));
    assert_eq!(pid_in("Session s7"), None);
    assert_eq!(pid_in(" (pid x)"), None);
}

#[tokio::test]
async fn a_process_holder_carries_the_session_it_began_from_on_the_wire() {
    use crate::session_host::following::SlotSession;
    let rig = rig_placing(None);
    let slots = rig.fleet.pool_slots(Reading::Live).await;
    let mut one = slots.into_iter().next().expect("a slot");
    one.reading.held = SlotHeld::Session(String::from("node (pid 2838)"));
    one.session = Some(SlotSession {
        id: String::from("5a21fab8"),
        title: Some(String::from("Debug Bridge on Merge line")),
    });

    assert_eq!(
        crate::wire::worktree_slot(&one).held,
        ipc::SlotHolding::Session {
            holder: String::from("node (pid 2838)"),
            session_id: Some(String::from("5a21fab8")),
            session_title: Some(String::from("Debug Bridge on Merge line")),
        }
    );
    one.session = None;
    assert_eq!(
        crate::wire::worktree_slot(&one).held,
        ipc::SlotHolding::Session {
            holder: String::from("node (pid 2838)"),
            session_id: None,
            session_title: None,
        }
    );
}
