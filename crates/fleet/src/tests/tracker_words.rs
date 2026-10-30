//! The words a tracker declares for the ticket's states: a repository on that tracker reads its
//! tickets from the first Job, one on a tracker that declares none reads as it always did, and a
//! lifecycle a repository writes for itself still replaces them. **Which words a tracker declares
//! is `adapters`' to assert**; under test here is that Fleet asks per repository and uses them.

use std::path::Path;
use std::sync::Arc;

use adapter_traits::TicketState;
use core_model::{Lifecycle, LifecycleId, ManifestId, Observed, Standing, StateId, Ulid};
use testkit::{FakeHarness, FakeTrackers, FakeVcs, FakeWorkProduct};

use super::session_host::rig_fitted;
use crate::daemon::Fleet;
use crate::repositories::{Located, SetUp};
use crate::tests::daemon::fittings;
use crate::tests::tmp::TempDir;

const FIRST: &str = "01FIXTUREMANIFEST";
const SECOND: &str = "01SECONDMANIFEST";

/// What a tracker with a fixed set of statuses declares, as beads does: nothing for In review.
const FIXED: &[(TicketState, &str)] =
    &[(TicketState::ToDo, "open"), (TicketState::InProgress, "in_progress"), (TicketState::Closed, "closed")];

type Hosted = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

/// The fixture's repository, which names no tracker, and a second that names `fixed`, a provider
/// the fake declares `FIXED` for.
fn two_repositories(home: &TempDir) -> Hosted {
    let mut fittings = fittings(home, FakeWorkProduct::changed(&[]));
    fittings.trackers = Arc::new(FakeTrackers::empty().ticket_words_named("fixed", FIXED));
    let workflows = fittings.starting().workflows.clone();
    let fleet = Fleet::assembled(fittings);
    let root = home.path().join("second");
    let text = format!("version: 1\nid: {SECOND}\ntracker:\n  provider: fixed\n");
    let manifest = config::Manifest::parse(&root.join("armada.yml"), &text).expect("loads");
    fleet
        .repositories()
        .add(Located {
            root: root.to_string_lossy().to_string(),
            records_root: home.path().join("second-records").to_string_lossy().to_string(),
            set_up: Some(SetUp::of(manifest, workflows)),
        })
        .expect("served");
    fleet
}

fn state(written: &str) -> StateId {
    StateId::new(written).expect("a state id")
}

fn ticket_of(fleet: &Hosted, manifest: &str) -> Lifecycle {
    let applying = fleet.lifecycles_applying_to(&ManifestId::carried(Ulid::carried(manifest))).expect("read");
    applying.get(&LifecycleId::new("ticket").expect("an id")).expect("a ticket").clone()
}

fn reads(ticket: &Lifecycle, status: &str) -> Standing {
    ticket.standing_of(&Observed::Tracker(status))
}

#[tokio::test]
async fn a_repository_reads_its_tickets_in_the_words_its_tracker_declares_and_one_that_declares_none_as_armada_carries_them() {
    let home = TempDir::new();
    let fleet = two_repositories(&home);

    let declared = ticket_of(&fleet, SECOND);
    assert_eq!(reads(&declared, "open"), Standing::In(state("todo")));
    assert_eq!(reads(&declared, "in_progress"), Standing::In(state("in_progress")));
    assert_eq!(reads(&declared, "closed"), Standing::In(state("closed")));
    assert!(matches!(reads(&declared, "blocked"), Standing::Unmapped(_)), "a status it declares nothing for stays a fact of its own");

    assert_eq!(ticket_of(&fleet, FIRST), Lifecycle::ticket(), "the repository that names no tracker reads as it always did");
}

#[tokio::test]
async fn a_lifecycle_a_repository_writes_for_its_ticket_replaces_the_words_its_tracker_declares() {
    let home = TempDir::new();
    let fleet = two_repositories(&home);
    let folder = Path::new(&home.path().join("second")).join(".armada").join("lifecycles");
    std::fs::create_dir_all(&folder).expect("a folder");
    std::fs::write(
        folder.join("ticket.yml"),
        "version: 1\nlifecycle_id: ticket\nname: Ticket\nstates:\n  - { id: todo, label: To do, read: { tracker: [Backlog] } }\n  - { id: closed, label: Closed, read: { tracker: [Done] } }\nends: [closed]\n",
    )
    .expect("its file");

    let written = ticket_of(&fleet, SECOND);
    assert_eq!(reads(&written, "Backlog"), Standing::In(state("todo")));
    assert!(matches!(reads(&written, "open"), Standing::Unmapped(_)), "the tracker's word is not laid over a person's");
}

#[tokio::test]
async fn every_state_a_tracker_may_declare_words_for_is_a_state_of_the_ticket_armada_carries() {
    let ticket = Lifecycle::ticket();
    for id in crate::lifecycles::ticket_state_ids() {
        assert!(ticket.state(&state(id)).is_some(), "{id}");
    }
}

#[tokio::test]
async fn a_job_in_a_repository_whose_tracker_declares_words_has_its_ticket_moved_by_them() {
    let rig = rig_fitted(|fittings| fittings.trackers = Arc::new(FakeTrackers::empty().ticket_words(FIXED)));
    let job = rig.start().await;
    let at = rig.fleet.now().as_str().to_string();
    rig.fleet.store().lock().await.take_on_lifecycle_instance(job.as_str(), "ticket", "ARM-41", &[], &at).expect("taken on");

    let opened = rig.fleet.lifecycle_seen(job.as_str(), "ticket", "ARM-41", Observed::Tracker("open")).await.expect("moved");
    assert_eq!(opened.moved.to, Standing::In(state("todo")));
    let started = rig.fleet.lifecycle_seen(job.as_str(), "ticket", "ARM-41", Observed::Tracker("in_progress")).await.expect("moved");
    assert_eq!(started.moved.to, Standing::In(state("in_progress")));
}
