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

use adapter_traits::{Claimed, Holder, Store};
use testkit::{FakeStore, FakeWorkProduct};

use super::main_ci::Fixture;
use super::main_ci::MANIFEST;
use super::main_fix::bug;
use crate::daemon::Fleet;
use crate::noticing::Noticing;
use crate::red_claim::Reaching;
use crate::tests::daemon::fittings;
use crate::tests::tmp::TempDir;

pub(super) const SLUG: &str = "mele-labs/armada";

/// What a machine answers for the store: this machine's view of it, or none
/// while it is signed out.
pub(crate) struct Planted {
    store: Option<Arc<FakeStore>>,
    bucket: Option<Arc<testkit::FakeBucket>>,
}

impl Planted {
    pub(crate) fn signed_out() -> Planted {
        Planted { store: None, bucket: None }
    }

    /// A machine with a bucket to reach and no store.
    pub(crate) fn with_bucket(bucket: Arc<testkit::FakeBucket>) -> Planted {
        Planted { store: None, bucket: Some(bucket) }
    }
}

/// The bucket a machine holds, handed out as the bucket.
struct Held(Arc<testkit::FakeBucket>);

impl adapter_traits::Bucket for Held {
    fn get(&self, name: &str) -> Result<Option<Vec<u8>>, adapter_traits::BucketError> {
        self.0.get(name)
    }
    fn put(&self, name: &str, body: &[u8]) -> Result<(), adapter_traits::BucketError> {
        self.0.put(name, body)
    }
}

/// The view a machine holds, handed out as the store.
pub(super) struct Viewed(pub(super) Arc<FakeStore>);

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
    fn put_runs(&self, repository: &adapter_traits::RepositoryKey, runs: &[adapter_traits::RunRow]) -> Result<(), adapter_traits::StoreError> {
        self.0.put_runs(repository, runs)
    }
    fn read_runs(&self, repository: &adapter_traits::RepositoryKey) -> Result<Option<Vec<adapter_traits::RunRow>>, adapter_traits::StoreError> {
        self.0.read_runs(repository)
    }
}

impl Reaching for Planted {
    fn slug(&self, _root: &str) -> Option<String> {
        Some(SLUG.to_string())
    }

    fn open(&self, _url: &str, _directory: &Path) -> Result<Option<Box<dyn Store + Send>>, String> {
        Ok(self.store.clone().map(|view| Box::new(Viewed(view)) as Box<dyn Store + Send>))
    }

    fn bucket(&self, _address: &adapters::BucketAddress) -> Result<Box<dyn adapter_traits::Bucket + Send>, adapter_traits::BucketError> {
        match &self.bucket {
            Some(bucket) => Ok(Box::new(Held(Arc::clone(bucket)))),
            None => Err(adapter_traits::BucketError::Unsigned(String::from("No credentials for this bucket on the fake machine"))),
        }
    }
}

/// One machine: its Fleet, and its own view of the store everybody shares.
pub(super) struct Machine {
    pub(super) fleet: Fixture,
    /// This machine's own handle on the shared store, so a test can switch it off.
    pub(super) view: Arc<FakeStore>,
    pub(super) _home: TempDir,
}

/// A Fleet signed in to the account, named `name`, reading the shared store
/// through a view of its own. `switch` is the Manifest's *Pull request owners*.
pub(super) async fn machine(name: &str, shared: &FakeStore, switch: bool) -> Machine {
    machine_hosting(name, shared, switch, None).await
}

/// [`machine`] whose hosted Sessions run on `stand_in`, where one is given.
pub(super) async fn machine_hosting(
    name: &str,
    shared: &FakeStore,
    switch: bool,
    stand_in: Option<Arc<crate::tests::session_host::StandIn>>,
) -> Machine {
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
    fittings.reaching = Arc::new(Planted { store: Some(Arc::clone(&view)), bucket: None });
    crate::profiles::Profiles::in_machine_directory(home.path())
        .join(
            &adapter_traits::SignedInAs { id: 583_231, login: "octocat".into() },
            crate::profiles::Orgs::Read(vec!["mele-labs".into()]),
            "2026-10-14T09:00:00.000Z",
        )
        .expect("a profile");
    let mut fleet = Fleet::assembled(fittings);
    if let Some(stand_in) = stand_in {
        fleet = fleet.hosting_sessions_on(Arc::new(crate::tests::session_host::Shared(stand_in)), Duration::from_secs(600));
    }
    let changes = [("store.url", "https://store.test"), ("machine.name", name)]
        .into_iter()
        .map(|(key, value)| (key.to_string(), Some(ipc::SettingValue::Text(value.to_string()))))
        .collect();
    api::Settings::save_settings(&fleet, ipc::SaveSettings { changes }).await.expect("saved");
    Machine { fleet, view, _home: home }
}

