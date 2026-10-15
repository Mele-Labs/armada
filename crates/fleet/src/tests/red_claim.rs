//! A red main claimed in the central store, with two Fleets on one scripted
//! store: A takes the red and B's hub names A, B's takes are refused naming A,
//! B's take over replaces, a store that cannot be reached refuses the take, a
//! repository with the switch off claims nothing, and green clears both.
//!
//! **Here and not in `crates/acceptance`**: that crate cannot admit a Job
//! without spawning a Drone and never builds a `Fleet`, and a claim is made by
//! `Fleet::fix_main` and read back by the hub.

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{CiState, Claimed, Holder, HolderKind, Store};
use api::Sessions;
use core_model::JobStatus;
use ipc::{SessionFact, SessionId, SessionOrigin, SessionReport};
use testkit::{FakeStore, FakeWorkProduct};

use super::main_ci::{kept, run, the_hub, Fixture, MANIFEST, ONE, TWO};
use super::main_fix::{a_finished_job, bug, goes_red, root, takes};
use crate::daemon::Fleet;
use crate::noticing::Noticing;
use crate::red_claim::{keys_of, Reaching};
use crate::tests::daemon::fittings;
use crate::tests::tmp::TempDir;

pub(super) const SLUG: &str = "mele-labs/armada";

/// What a machine answers for the store: this machine's view of it, or none
/// while it is signed out.
pub(crate) struct Planted {
    store: Option<Arc<FakeStore>>,
}

impl Planted {
    pub(crate) fn signed_out() -> Planted {
        Planted { store: None }
    }
}

/// The view a machine holds, handed out as the store.
struct Viewed(Arc<FakeStore>);

impl Store for Viewed {
    fn claim(&self, repository: &adapter_traits::RepositoryKey, red: &adapter_traits::RedKey, holder: &Holder) -> Result<Claimed, adapter_traits::StoreError> {
        self.0.claim(repository, red, holder)
    }
    fn read_claim(&self, repository: &adapter_traits::RepositoryKey, red: &adapter_traits::RedKey) -> Result<Option<Holder>, adapter_traits::StoreError> {
        self.0.read_claim(repository, red)
    }
    fn release_claim(&self, repository: &adapter_traits::RepositoryKey, red: &adapter_traits::RedKey) -> Result<(), adapter_traits::StoreError> {
        self.0.release_claim(repository, red)
    }
    fn replace_claim(&self, repository: &adapter_traits::RepositoryKey, red: &adapter_traits::RedKey, holder: &Holder) -> Result<Claimed, adapter_traits::StoreError> {
        self.0.replace_claim(repository, red, holder)
    }
    fn clear_repository(&self, repository: &adapter_traits::RepositoryKey) -> Result<(), adapter_traits::StoreError> {
        self.0.clear_repository(repository)
    }
    fn put_owner(&self, key: &adapter_traits::OwnerKey, holder: &Holder) -> Result<(), adapter_traits::StoreError> {
        self.0.put_owner(key, holder)
    }
    fn owners(&self, keys: &[adapter_traits::OwnerKey]) -> Result<std::collections::BTreeMap<adapter_traits::OwnerKey, Holder>, adapter_traits::StoreError> {
        self.0.owners(keys)
    }
    fn delete_owner(&self, key: &adapter_traits::OwnerKey) -> Result<(), adapter_traits::StoreError> {
        self.0.delete_owner(key)
    }
    fn delete_device(&self, device: &str) -> Result<(), adapter_traits::StoreError> {
        self.0.delete_device(device)
    }
}

impl Reaching for Planted {
    fn slug(&self, _root: &str) -> Option<String> {
        Some(SLUG.to_string())
    }

    fn open(&self, _url: &str, _directory: &Path) -> Result<Option<Box<dyn Store + Send>>, String> {
        Ok(self.store.clone().map(|view| Box::new(Viewed(view)) as Box<dyn Store + Send>))
    }
}

/// One machine: its Fleet, and its own view of the store everybody shares.
pub(super) struct Machine {
    pub(super) fleet: Fixture,
    pub(super) view: Arc<FakeStore>,
    pub(super) _home: TempDir,
}

/// A Fleet signed in to the account, named `name`, reading the shared store
/// through a view of its own. `switch` is the Manifest's *Pull request owners*.
pub(super) async fn machine(name: &str, shared: &FakeStore, switch: bool) -> Machine {
    let home = TempDir::new();
    let mut fittings = fittings(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    fittings.noticing = Noticing::every(Duration::ZERO);
    let manifest = match switch {
        true => format!("{MANIFEST}policy:\n  pull_request_owners: true\n"),
        false => MANIFEST.to_string(),
    };
    fittings.starting().manifest = config::Manifest::parse(Path::new("armada.yml"), &manifest).expect("parses");
    let mut held = fittings.starting().workflows.clone();
    held.insert(bug().id().clone(), bug());
    fittings.starting().workflows = held;
    let view = Arc::new(shared.view());
    fittings.reaching = Arc::new(Planted { store: Some(Arc::clone(&view)) });
    crate::profiles::Profiles::in_machine_directory(home.path())
        .join(&adapter_traits::SignedInAs { id: 583_231, login: "octocat".into() }, "2026-10-14T09:00:00.000Z")
        .expect("a profile");
    let fleet = Fleet::assembled(fittings);
    let changes = [("store.url", "https://store.test"), ("machine.name", name)]
        .into_iter()
        .map(|(key, value)| (key.to_string(), Some(ipc::SettingValue::Text(value.to_string()))))
        .collect();
    api::Settings::save_settings(&fleet, ipc::SaveSettings { changes }).await.expect("saved");
    Machine { fleet, view, _home: home }
}

/// The row this red is under in the shared store.
fn row_of(store: &FakeStore) -> Option<Holder> {
    let (repository, red) = keys_of(SLUG, ONE).expect("keys");
    store.read_claim(&repository, &red).expect("reads")
}

async fn red_on(machine: &Machine) {
    goes_red(&machine.fleet, 1815);
    machine.fleet.turn().await.unwrap();
}

#[tokio::test]
async fn a_take_is_seen_and_refused_on_the_other_machine_until_it_takes_over() {
    let shared = FakeStore::new();
    let (a, b) = (machine("mac-a", &shared, true).await, machine("mac-b", &shared, true).await);
    red_on(&a).await;
    red_on(&b).await;
    assert!(the_hub(&b.fleet).await.fixing_elsewhere.is_none(), "nobody holds it yet");

    let job = a.fleet.fix_main(&root(&a.fleet).await, None, None, false).await.unwrap();
    let held = row_of(&shared).expect("A claimed the red");
    assert_eq!((held.kind, held.id.as_str(), held.machine.as_str()), (HolderKind::Job, job.id().as_str(), "mac-a"));

    b.fleet.turn().await.unwrap();
    let named = the_hub(&b.fleet).await.fixing_elsewhere.expect("B's hub names A");
    assert_eq!((named.machine.as_str(), named.holder.id.as_str()), ("mac-a", job.id().as_str()));
    assert_eq!(named.holder.kind, ipc::HolderKind::Job);
    assert!(the_hub(&a.fleet).await.fixing_elsewhere.is_none(), "A's own take is `fixing`");

    let refused = b.fleet.fix_main(&root(&b.fleet).await, None, None, false).await.unwrap_err();
    let wire = b.fleet.refusal(refused);
    assert_eq!(wire.status(), 409);
    assert_eq!(wire.error().code, "fleet.main_taken_elsewhere");
    assert!(
        wire.error().message.contains("mac-a") && wire.error().message.contains(job.id().as_str()),
        "{}",
        wire.error().message
    );
    assert!(b.fleet.every_job().await.unwrap().0.jobs.is_empty(), "nothing was proposed");
    assert!(takes(&b.fleet).await.is_empty());

    let over = b.fleet.fix_main(&root(&b.fleet).await, None, None, true).await.unwrap();
    let held = row_of(&shared).expect("a row");
    assert_eq!((held.id.as_str(), held.machine.as_str()), (over.id().as_str(), "mac-b"), "B's row replaced A's");
    assert!(b.view.calls().contains(&"replace_claim"), "{:?}", b.view.calls());
}

#[tokio::test]
async fn a_session_is_refused_naming_the_holder_and_takes_over_when_asked() {
    let shared = FakeStore::new();
    let (a, b) = (machine("mac-a", &shared, true).await, machine("mac-b", &shared, true).await);
    red_on(&a).await;
    red_on(&b).await;
    let job = a.fleet.fix_main(&root(&a.fleet).await, None, None, false).await.unwrap();
    b.fleet
        .report_session(SessionReport {
            harness: adapters::HOSTED_HARNESS.into(),
            session_id: SessionId::carried("s-b"),
            fact: SessionFact::Started { cwd: root(&b.fleet).await, title: None, origin: SessionOrigin::Terminal, mod_version: None },
        })
        .await
        .unwrap();

    let ask = |take_over| ipc::TakeMainRed { session_id: Some(SessionId::carried("s-b")), take_over };
    let refused = b.fleet.take_main_red(None, ask(false)).await.unwrap_err();
    assert_eq!(refused.error().code, "fleet.main_taken_elsewhere");
    assert!(refused.error().message.contains(job.id().as_str()), "{}", refused.error().message);
    assert_eq!(row_of(&shared).unwrap().machine, "mac-a", "refusing changed nothing");

    let taken = b.fleet.take_main_red(None, ask(true)).await.expect("taken over");
    assert!(taken.claimed);
    let held = row_of(&shared).expect("a row");
    assert_eq!((held.kind, held.id.as_str(), held.machine.as_str()), (HolderKind::Session, "s-b", "mac-b"));
}

#[tokio::test]
async fn a_session_takes_a_free_red_and_a_job_here_is_then_refused_naming_it() {
    let shared = FakeStore::new();
    let a = machine("mac-a", &shared, true).await;
    red_on(&a).await;
    a.fleet
        .report_session(SessionReport {
            harness: adapters::HOSTED_HARNESS.into(),
            session_id: SessionId::carried("s-a"),
            fact: SessionFact::Started { cwd: root(&a.fleet).await, title: None, origin: SessionOrigin::Terminal, mod_version: None },
        })
        .await
        .unwrap();
    let taken = a.fleet.take_main_red(None, ipc::TakeMainRed { session_id: Some(SessionId::carried("s-a")), take_over: false })
        .await
        .expect("taken");
    assert!(taken.claimed);

    let refused = a.fleet.fix_main(&root(&a.fleet).await, None, None, false).await.unwrap_err();
    assert!(refused.to_string().contains("Session s-a on mac-a"), "{refused}");
}

#[tokio::test]
async fn a_store_that_cannot_be_reached_refuses_the_take_and_nothing_is_made() {
    let shared = FakeStore::new();
    let a = machine("mac-a", &shared, true).await;
    red_on(&a).await;
    a.view.set_unreachable(true);

    let refused = a.fleet.fix_main(&root(&a.fleet).await, None, None, false).await.unwrap_err();
    let wire = a.fleet.refusal(refused);
    assert_eq!(wire.error().code, "fleet.main_claim_not_made");
    assert!(wire.error().message.contains("could not be reached"), "{}", wire.error().message);
    assert!(a.fleet.every_job().await.unwrap().0.jobs.is_empty());
    assert!(takes(&a.fleet).await.is_empty());
    assert!(row_of(&shared).is_none());

    a.view.set_unreachable(false);
    a.fleet.fix_main(&root(&a.fleet).await, None, None, false).await.expect("taken once it answers");
}

#[tokio::test]
async fn a_hub_whose_store_cannot_be_read_has_no_field_and_no_error() {
    let shared = FakeStore::new();
    let (a, b) = (machine("mac-a", &shared, true).await, machine("mac-b", &shared, true).await);
    red_on(&a).await;
    red_on(&b).await;
    a.fleet.fix_main(&root(&a.fleet).await, None, None, false).await.unwrap();
    b.fleet.turn().await.unwrap();
    assert!(the_hub(&b.fleet).await.fixing_elsewhere.is_some());

    b.view.set_unreachable(true);
    b.fleet.turn().await.unwrap();
    assert!(the_hub(&b.fleet).await.fixing_elsewhere.is_none());
}

#[tokio::test]
async fn a_repository_with_the_switch_off_claims_nothing_and_draws_no_remote_claim() {
    let shared = FakeStore::new();
    let (holder, quiet) = (machine("mac-a", &shared, true).await, machine("mac-b", &shared, false).await);
    red_on(&holder).await;
    red_on(&quiet).await;
    holder.fleet.fix_main(&root(&holder.fleet).await, None, None, false).await.unwrap();
    assert!(row_of(&shared).is_some(), "a claim stands in the store");

    quiet.fleet.turn().await.unwrap();
    assert!(the_hub(&quiet.fleet).await.fixing_elsewhere.is_none(), "it shows nothing of another machine's claim");
    let job = quiet.fleet.fix_main(&root(&quiet.fleet).await, None, None, false).await.expect("proceeds on this machine alone");
    assert_eq!(job.status(), JobStatus::Queued);
    assert_eq!(quiet.view.calls(), Vec::<&str>::new(), "the store was never asked");
    assert_eq!(row_of(&shared).unwrap().machine, "mac-a", "and nothing of its own was written");
}

#[tokio::test]
async fn a_session_in_a_repository_with_the_switch_off_takes_nothing_and_is_told_so() {
    let shared = FakeStore::new();
    let quiet = machine("mac-a", &shared, false).await;
    red_on(&quiet).await;
    quiet
        .fleet
        .report_session(SessionReport {
            harness: adapters::HOSTED_HARNESS.into(),
            session_id: SessionId::carried("s-q"),
            fact: SessionFact::Started { cwd: root(&quiet.fleet).await, title: None, origin: SessionOrigin::Terminal, mod_version: None },
        })
        .await
        .unwrap();
    let taken = quiet.fleet.take_main_red(None, ipc::TakeMainRed { session_id: Some(SessionId::carried("s-q")), take_over: false }).await.expect("answers");
    assert!(!taken.claimed);
    assert_eq!(quiet.view.calls(), Vec::<&str>::new());
}

#[tokio::test]
async fn a_session_asking_again_for_the_red_it_holds_is_not_refused() {
    let shared = FakeStore::new();
    let a = machine("mac-a", &shared, true).await;
    red_on(&a).await;
    a.fleet
        .report_session(SessionReport {
            harness: adapters::HOSTED_HARNESS.into(),
            session_id: SessionId::carried("s-a"),
            fact: SessionFact::Started { cwd: root(&a.fleet).await, title: None, origin: SessionOrigin::Terminal, mod_version: None },
        })
        .await
        .unwrap();
    let ask = || ipc::TakeMainRed { session_id: Some(SessionId::carried("s-a")), take_over: false };
    assert!(a.fleet.take_main_red(None, ask()).await.expect("first").claimed);
    assert!(a.fleet.take_main_red(None, ask()).await.expect("again").claimed);
}

#[tokio::test]
async fn a_machine_signed_out_claims_nothing() {
    let shared = FakeStore::new();
    let out = machine("mac-a", &shared, true).await;
    std::fs::remove_file(out._home.path().join(crate::profiles::FILE)).expect("signed out");
    red_on(&out).await;
    out.fleet.fix_main(&root(&out.fleet).await, None, None, false).await.expect("proceeds");
    assert_eq!(out.view.calls(), Vec::<&str>::new());
    assert!(row_of(&shared).is_none());
}

#[tokio::test]
async fn green_clears_every_claim_for_both_machines() {
    let shared = FakeStore::new();
    let (a, b) = (machine("mac-a", &shared, true).await, machine("mac-b", &shared, true).await);
    red_on(&a).await;
    red_on(&b).await;
    a.fleet.fix_main(&root(&a.fleet).await, None, None, false).await.unwrap();
    b.fleet.turn().await.unwrap();
    assert!(the_hub(&b.fleet).await.fixing_elsewhere.is_some());

    for each in [&a, &b] {
        let forge = &each.fleet.vcs().main_ci;
        forge.head_is(Some(TWO));
        forge.runs_on(TWO, vec![run("ci", "21", CiState::Passed)]);
    }
    a.fleet.turn().await.unwrap();
    assert!(row_of(&shared).is_none(), "A cleared the repository");
    assert_eq!(a.view.calls().iter().filter(|call| **call == "clear_repository").count(), 1, "{:?}", a.view.calls());
    b.fleet.turn().await.unwrap();
    assert!(the_hub(&b.fleet).await.fixing_elsewhere.is_none());
    assert_eq!(kept(&b.fleet).await.unwrap().state, store::MainState::Green);
}

#[tokio::test]
async fn a_take_that_ended_gives_its_claim_back_and_the_other_machine_may_take_the_red() {
    let shared = FakeStore::new();
    let (a, b) = (machine("mac-a", &shared, true).await, machine("mac-b", &shared, true).await);
    red_on(&a).await;
    red_on(&b).await;
    let job = a.fleet.fix_main(&root(&a.fleet).await, None, None, false).await.unwrap();
    assert!(row_of(&shared).is_some());

    a.fleet.kill_job(job.id()).await.unwrap();
    a.fleet.turn().await.unwrap();
    assert!(row_of(&shared).is_none(), "the stopped Job's claim was given back");
    assert!(a.view.calls().contains(&"release_claim"), "{:?}", a.view.calls());

    b.fleet.fix_main(&root(&b.fleet).await, None, None, false).await.expect("B takes the freed red");
    assert_eq!(row_of(&shared).unwrap().machine, "mac-b");
}

#[tokio::test]
async fn a_stopped_job_on_this_machine_does_not_block_its_own_next_take() {
    let shared = FakeStore::new();
    let a = machine("mac-a", &shared, true).await;
    red_on(&a).await;
    let first = a.fleet.fix_main(&root(&a.fleet).await, None, None, false).await.unwrap();
    a.fleet.kill_job(first.id()).await.unwrap();

    let second = a.fleet.fix_main(&root(&a.fleet).await, None, None, false).await.expect("replaces its own stale row");
    assert_eq!(row_of(&shared).unwrap().id, second.id().as_str());
}

#[tokio::test]
async fn the_automatic_pickup_claims_first_and_skips_a_red_held_elsewhere() {
    let shared = FakeStore::new();
    let (a, b) = (machine("mac-a", &shared, true).await, machine("mac-b", &shared, true).await);
    let culprit_a = a_finished_job(&a.fleet, &a._home, "fix the off-by-one in the log reader").await;
    let culprit_b = a_finished_job(&b.fleet, &b._home, "fix the off-by-one in the log reader").await;
    for each in [&a, &b] {
        goes_red(&each.fleet, 1);
    }

    a.fleet.turn().await.unwrap();
    let [take] = takes(&a.fleet).await.try_into().expect("A sent the work back once");
    let held = row_of(&shared).expect("claimed before the send-back");
    assert_eq!((held.id.as_str(), held.machine.as_str()), (take.job.as_str(), "mac-a"));
    assert_ne!(take.job, *culprit_a.id());

    b.fleet.turn().await.unwrap();
    assert!(takes(&b.fleet).await.is_empty(), "B skipped it silently");
    let (loaded, _) = b.fleet.every_job().await.unwrap();
    assert_eq!(loaded.jobs.len(), 1, "only B's own culprit {:?}", culprit_b.id());
    assert_eq!(row_of(&shared).unwrap().machine, "mac-a");
}
