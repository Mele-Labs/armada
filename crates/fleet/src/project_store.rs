//! Which claims store a repository's calls go to: **the one its projects name, or none.**
//! `docs/contracts/central-store.md`, *A store per project*.
//!
//! There is no machine-wide address. A repository in no project, or in projects that name no store,
//! makes no call, and that holds for what was waiting to be sent too, because the store is read when
//! a call is made and never kept beside it. A repository in two projects that name two different
//! stores also makes none: nothing says which the person meant, so it is said on the log and no claim
//! leaves for either.

use std::path::{Path, PathBuf};

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use core_model::{ProjectStore, StoreAddress};
use sha2::{Digest, Sha256};

use crate::daemon::Fleet;

/// A project's store as the wire carries it.
pub(crate) fn store_on_the_wire(store: &ProjectStore) -> ipc::ProjectStore {
    ipc::ProjectStore {
        address: store.address.as_str().to_string(),
        team: store.team.as_ref().map(ToString::to_string),
        worker: store.worker.as_ref().map(|worker| worker.as_str().to_string()),
        deployed: store.deployed.as_ref().map(|at| at.as_str().to_string()),
    }
}

/// Where a profile keeps its key and device id for the store at `address`: a folder of its own under
/// the profile's, named for the address, so one profile enrolled with two stores keeps two devices.
pub(crate) fn store_directory(profile: &Path, address: &StoreAddress) -> PathBuf {
    let digest = Sha256::digest(address.as_str().as_bytes());
    let name: String = digest.iter().take(8).map(|byte| format!("{byte:02x}")).collect();
    profile.join("stores").join(name)
}

/// What the projects holding one repository say of its store.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum StoreOf {
    /// No project holds it, or none that does names a store.
    None,
    One(StoreAddress),
    /// Two projects holding it name different stores.
    Differs,
}

/// The store of the repository `manifest` from the projects in `projects`.
pub(crate) fn store_of<'p>(projects: impl Iterator<Item = &'p core_model::Project>, manifest: &core_model::ManifestId) -> StoreOf {
    let mut found: Option<&StoreAddress> = None;
    for project in projects.filter(|project| project.holds(manifest)) {
        let Some(store) = project.store() else { continue };
        match found {
            Some(already) if already != &store.address => return StoreOf::Differs,
            _ => found = Some(&store.address),
        }
    }
    found.map_or(StoreOf::None, |address| StoreOf::One(address.clone()))
}

impl<H, V, W> Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// The store the repository at `root` calls, from the projects that hold it **as Fleet holds them
    /// now**. `None` makes no call.
    pub(crate) fn store_of_root(&self, root: &str) -> Option<StoreAddress> {
        let manifest = core_model::ManifestId::carried(core_model::Ulid::carried(self.manifest_id_at(root)?));
        let held = self.projects_held();
        match store_of(held.running(), &manifest) {
            StoreOf::One(address) => Some(address),
            StoreOf::None => None,
            StoreOf::Differs => {
                self.values_warned(
                    "a repository's claims were not sent",
                    Some(manifest.as_str()),
                    "two projects that hold it name different stores",
                );
                None
            }
        }
    }

    /// Name `store` as project `project`'s, or none, in its file, and answer the projects as every
    /// write does. **Nothing is called here**: a join or a deploy reaches the store first.
    pub(crate) async fn project_store_written(&self, project: &str, store: Option<ProjectStore>) -> Result<ipc::Projects, Refusal> {
        self.project_not_held(project).await?;
        let held = self.project_read(project)?;
        let named = held.clone().with_store(store);
        if named != held {
            self.project_stored(&named).await?;
        }
        self.projects_written_reading(|_, _| Ok(()), true).await
    }
}
