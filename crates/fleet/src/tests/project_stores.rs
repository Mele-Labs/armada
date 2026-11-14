//! A store per project: two projects in one Fleet, each naming its own store, send each repository's
//! claims to its own project's store; a project that names none makes no call, a store taken out of a
//! project takes what was still to be sent with it, and two projects naming different stores for one
//! repository make none. `docs/contracts/central-store.md`, *A store per project*.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use adapter_traits::{ClaimKind, Claimant, Claimed, HolderKind, Store};
use testkit::{FakeStore, FakeWorkProduct};

use super::main_ci::Fixture;
use super::red_claim::Viewed;
use crate::red_claim::{keys_of, Reaching};
use crate::repositories::{Located, SetUp};
use crate::tests::daemon::fittings;
use crate::tests::tmp::TempDir;

const SECOND_MANIFEST: &str = "version: 1\nid: 01SECONDMANIFEST\nbase: main\nchecks:\n  test:\n    run: cargo nextest run\n";

/// A store for each address, and every address a call opened, with the folder it opened as.
struct ByAddress {
    stores: BTreeMap<String, Arc<FakeStore>>,
    opened: Mutex<Vec<(String, PathBuf)>>,
}

impl Reaching for ByAddress {
    fn slug(&self, root: &str) -> Option<String> {
        let name = Path::new(root).file_name()?.to_str()?;
        Some(format!("mele-labs/{name}"))
    }

    fn open(&self, url: &str, directory: &Path) -> Result<Option<Box<dyn Store + Send>>, String> {
        self.opened.lock().expect("opened").push((url.to_string(), directory.to_path_buf()));
        Ok(self.stores.get(url).map(|view| Box::new(Viewed(Arc::clone(view))) as Box<dyn Store + Send>))
    }

    fn bucket(&self, _address: &adapters::BucketAddress) -> Result<Box<dyn adapter_traits::Bucket + Send>, adapter_traits::BucketError> {
        Err(adapter_traits::BucketError::Unsigned(String::from("this fake machine has no bucket")))
    }
}

struct Rig {
    fleet: Fixture,
    reaching: Arc<ByAddress>,
    first: String,
    second: String,
    one: Arc<FakeStore>,
    two: Arc<FakeStore>,
    _home: TempDir,
}

const ONE: &str = "https://one.test";
const TWO: &str = "https://two.test";

/// One Fleet serving two repositories, signed in as one profile that both are bound to, and two
/// stores to reach. No project names either yet.
async fn rig() -> Rig {
    let home = TempDir::new();
    let (one, two) = (Arc::new(FakeStore::new()), Arc::new(FakeStore::new()));
    let reaching =
        Arc::new(ByAddress { stores: BTreeMap::from([(ONE.into(), Arc::clone(&one)), (TWO.into(), Arc::clone(&two))]), opened: Mutex::default() });
    let mut fittings = fittings(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    fittings.reaching = Arc::clone(&reaching) as Arc<dyn Reaching>;
    crate::profiles::Profiles::in_machine_directory(home.path())
        .join(
            &adapter_traits::SignedInAs { id: 583_231, login: "octocat".into() },
            crate::profiles::Orgs::Read(vec!["mele-labs".into()]),
            "2026-10-14T09:00:00.000Z",
        )
        .expect("a profile");
    let fleet = crate::daemon::Fleet::assembled(fittings);
    let second = home.path().join("site").to_string_lossy().to_string();
    let parsed = config::Manifest::parse(&Path::new(&second).join("armada.yml"), SECOND_MANIFEST).expect("parses");
    fleet
        .repositories()
        .add(Located {
            root: second.clone(),
            records_root: home.path().join("site-records").to_string_lossy().to_string(),
            set_up: Some(SetUp::of(parsed, BTreeMap::new())),
        })
        .expect("served");
    let first = super::main_fix::root(&fleet).await;
    Rig { fleet, reaching, first, second, one, two, _home: home }
}

impl Rig {
    /// A project holding the repository at `root` and naming the store at `address`, or none. Its id.
    async fn project(&self, root: &str, address: Option<&str>) -> String {
        let manifest = self.fleet.manifest_id_at(root).expect("served");
        let members = Some(vec![ipc::ProjectMember { manifest_id: ipc::ManifestId::carried(manifest.as_str()), folder: None }]);
        let new = ipc::NewProject { name: format!("Holds {root}"), members, place: None, values: None };
        let id = api::Projects::add_project(&self.fleet, new).await.expect("added").projects.last().expect("the project").id.clone();
        if let Some(address) = address {
            self.store(&id, Some(address)).await;
        }
        id
    }

    async fn store(&self, project: &str, address: Option<&str>) {
        let store = address.map(|address| core_model::ProjectStore::joined(core_model::StoreAddress::new(address).expect("an address"), None));
        self.fleet.project_store_written(project, store).await.expect("its store named");
    }

    /// Claim a red for the repository at `root` through the gate every call passes. `None` is no call.
    async fn claim(&self, root: &str, commit: &str) -> Option<Claimed> {
        let commit = commit.to_string();
        self.fleet
            .at_the_store_switched(root, true, move |opened| {
                let (repository, key) = keys_of(&opened.slug, &commit).expect("keys");
                let holder = adapter_traits::Holder { kind: HolderKind::Session, id: "s-1".into(), machine: opened.machine.clone(), since: 1 };
                opened.store.claim(&repository, ClaimKind::Red, &key, &Claimant { holder, name: opened.login.clone(), title: String::new() })
            })
            .await
            .expect("answered")
    }

    fn opened(&self) -> Vec<String> {
        self.reaching.opened.lock().expect("opened").iter().map(|(url, _)| url.clone()).collect()
    }

    fn held(store: &FakeStore, slug: &str, commit: &str) -> bool {
        let (repository, key) = keys_of(slug, commit).expect("keys");
        store.read_claim(&repository, ClaimKind::Red, &key).expect("read").is_some()
    }
}

#[tokio::test]
async fn each_projects_claims_reach_its_own_store_and_a_project_with_none_makes_no_call() {
    let rig = rig().await;
    rig.project(&rig.first, Some(ONE)).await;
    let none = rig.project(&rig.second, None).await;

    assert!(rig.claim(&rig.first, "aaa").await.is_some(), "the first project's store answered");
    assert!(rig.claim(&rig.second, "bbb").await.is_none(), "a project with none makes no call");
    assert_eq!(rig.opened(), vec![ONE.to_string()], "only the first project's store was opened");

    rig.store(&none, Some(TWO)).await;
    assert!(rig.claim(&rig.second, "bbb").await.is_some(), "the second project's store answered");

    let first_slug = format!("mele-labs/{}", Path::new(&rig.first).file_name().and_then(|it| it.to_str()).expect("a name"));
    assert!(Rig::held(&rig.one, &first_slug, "aaa"));
    assert!(!Rig::held(&rig.two, &first_slug, "aaa"), "the first repository's claim is not in the second store");
    assert!(Rig::held(&rig.two, "mele-labs/site", "bbb"));
    assert!(!Rig::held(&rig.one, "mele-labs/site", "bbb"), "the second repository's claim is not in the first store");
    assert_eq!(rig.opened(), vec![ONE.to_string(), TWO.to_string()]);
}

#[tokio::test]
async fn a_store_taken_out_of_a_project_takes_what_was_still_to_send_with_it() {
    let rig = rig().await;
    let project = rig.project(&rig.first, Some(ONE)).await;
    rig.store(&project, None).await;

    assert!(rig.claim(&rig.first, "aaa").await.is_none(), "a project with none sends nothing, whenever the claim was made");
    assert!(rig.opened().is_empty(), "no store was opened");
}

#[tokio::test]
async fn a_repository_in_two_projects_naming_different_stores_makes_no_call() {
    let rig = rig().await;
    rig.project(&rig.first, Some(ONE)).await;
    let other = rig.project(&rig.first, Some(TWO)).await;

    assert!(rig.claim(&rig.first, "aaa").await.is_none(), "nothing says which store was meant");
    assert!(rig.opened().is_empty());

    rig.store(&other, Some(ONE)).await;
    assert!(rig.claim(&rig.first, "aaa").await.is_some(), "two projects naming one store is that store");
}

#[tokio::test]
async fn one_profile_keeps_a_device_folder_for_each_store() {
    let rig = rig().await;
    rig.project(&rig.first, Some(ONE)).await;
    rig.project(&rig.second, Some(TWO)).await;
    rig.claim(&rig.first, "aaa").await;
    rig.claim(&rig.second, "bbb").await;

    let folders: Vec<PathBuf> = rig.reaching.opened.lock().expect("opened").iter().map(|(_, folder)| folder.clone()).collect();
    assert_eq!(folders.len(), 2);
    assert_ne!(folders[0], folders[1], "two stores, two devices");
    assert_eq!(folders[0].parent(), folders[1].parent(), "both under the profile's `stores` folder");
    assert!(folders[0].parent().is_some_and(|stores| stores.ends_with("stores")), "{folders:?}");
}
