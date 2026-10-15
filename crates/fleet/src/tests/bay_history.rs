//! A bay's history as Fleet names it: the pool's lines hold a process or an id, and Fleet says whether
//! an id is a Job or a Session and what it is called.

use store::{KeptSession, SessionFigures, SessionState};

use crate::tests::daemon::a_fleet;
use crate::tests::tmp::TempDir;
use testkit::FakeWorkProduct;

type Fixture = crate::Fleet<testkit::FakeHarness, testkit::FakeVcs, FakeWorkProduct>;

fn served(fleet: &Fixture) -> (String, String) {
    let served = fleet.repositories().served().remove(0);
    (served.manifest().id().as_str().to_string(), served.root().trim_end_matches('/').to_string())
}

async fn a_session(fleet: &Fixture, id: &str, title: &str) {
    let now = fleet.now().as_str().to_string();
    fleet
        .store()
        .lock()
        .await
        .keep_session(&KeptSession {
            id: id.to_string(),
            harness: "a-harness".to_string(),
            origin: "terminal".to_string(),
            manifest_id: Some(served(fleet).0),
            cwd: "/slot".to_string(),
            title: Some(title.to_string()),
            state: SessionState::Live,
            started_at: now.clone(),
            last_seen_at: now,
            last_turn_at: None,
            ended_at: None,
            end_reason: None,
            figures: SessionFigures::default(),
            mod_version: None,
        })
        .unwrap();
}

/// A lease a process took with the agent's session in its record is an `agent` named by that Session,
/// one with none is named by its pid, and an id no Job has is a hosted Session. Newest first, a release
/// after the lease it ends.
#[tokio::test]
async fn holders_are_named_for_what_took_the_lease() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    let (manifest, root) = served(&fleet);
    a_session(&fleet, "sess-agent", "Checks --changed Bug").await;
    a_session(&fleet, "sess-hosted", "Hosted one").await;

    let now = fleet.now().epoch_millis().expect("an instant");
    let slots = std::path::Path::new(&root).join(".armada/slots");
    std::fs::create_dir_all(&slots).unwrap();
    let process = "4120 Thu Oct 15 16:06:37 2026";
    let journal = format!(
        "{}\tleased\t{process}\tsess-agent\tchecks\n\
         {}\treleased\t{process}\tsess-agent\t-\n\
         {}\tleased\tjob sess-hosted\t-\tfix\n\
         {}\tleased\t77 Thu Oct 15 16:06:37 2026\t-\tby-hand\n",
        now - 4000,
        now - 3000,
        now - 2000,
        now - 1000,
    );
    std::fs::write(slots.join("slot-10.history"), journal).unwrap();

    let ask = ipc::BayHistoryAsk { manifest_id: ipc::ManifestId::carried(&manifest), slot: 10 };
    let read = fleet.slot_history(ask).await.expect("a history");
    assert!(!read.cut);
    let holders: Vec<(bool, ipc::BayHolderKind, String, Option<String>)> = read
        .entries
        .iter()
        .map(|entry| match entry {
            ipc::BayHistoryEntry::Leased { holder, .. } => (true, holder.kind, holder.id.clone(), holder.title.clone()),
            ipc::BayHistoryEntry::Released { holder, .. } => (false, holder.kind, holder.id.clone(), holder.title.clone()),
            other => panic!("no checkout without a worktree: {other:?}"),
        })
        .collect();
    assert_eq!(
        holders,
        [
            (true, ipc::BayHolderKind::Agent, "77".to_string(), None),
            (true, ipc::BayHolderKind::Session, "sess-hosted".to_string(), Some("Hosted one".to_string())),
            (false, ipc::BayHolderKind::Agent, "sess-agent".to_string(), Some("Checks --changed Bug".to_string())),
            (true, ipc::BayHolderKind::Agent, "sess-agent".to_string(), Some("Checks --changed Bug".to_string())),
        ]
    );
}

/// A repository Fleet does not serve is refused, and a slot nothing was written for answers none.
#[tokio::test]
async fn an_unserved_repository_is_refused_and_a_quiet_slot_is_empty() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    let (manifest, _) = served(&fleet);

    let ask = |manifest: &str| ipc::BayHistoryAsk { manifest_id: ipc::ManifestId::carried(manifest), slot: 3 };
    assert!(fleet.slot_history(ask("not-served")).await.is_err());
    let quiet = fleet.slot_history(ask(&manifest)).await.expect("a history");
    assert!(quiet.entries.is_empty() && !quiet.cut);
}
