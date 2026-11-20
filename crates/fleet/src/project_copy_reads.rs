//! A copy of another Mac's project read as a project of this Mac's: its name, members, values and store as that
//! Mac said them, with what he edited on it since. `docs/concepts/project.md`, *Sharing with his other Macs*.
//!
//! **A read of a copy answers from what the copy carries, with this Mac's own layers and repositories**, so its
//! values resolve over this Mac's Kit, its issues come from its tracker read here, and its features are this Mac's
//! catalog. Nothing is asked of the other Mac.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use core_model::{ProjectStore, StoreAddress, StoreTeam};
use ipc::{Project, WireError};
use store::{FromMac, Store};

use crate::daemon::Fleet;

/// A Job goes into a copy only in a repository the copy lists. A 422.
pub(crate) const COPY_NOT_MEMBER: &str = "fleet.project_copy_not_member";

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
    /// The copy `id`, as shown with what is queued on it, read as a project. **None where no copy has that id**, or
    /// where what it carries does not make one. A caller asks for this Mac's own project first.
    pub(crate) async fn copy_read(&self, id: &str) -> Option<core_model::Project> {
        let copies = self.project_copies().await.ok()?;
        copies.into_iter().find(|one| one.id == id).and_then(|copy| self.copy_as_project(&copy))
    }

    /// Every copy held, each read as a project. **One that does not read is left out.**
    pub(crate) async fn copies_read(&self) -> Vec<core_model::Project> {
        let copies = self.copies_held().await.unwrap_or_default();
        copies.iter().filter_map(|copy| self.copy_as_project(copy)).collect()
    }

    /// [`Fleet::copy_read`] from a store already locked, as the copy's Mac last said it.
    pub(crate) fn copy_read_in(&self, store: &Store, id: &str) -> Option<core_model::Project> {
        let held = store.held_from_macs(FromMac::Projects).ok()?;
        held.into_iter()
            .filter_map(|one| ipc::decode::<Project>("a project from another Mac", one.body.as_bytes()).ok())
            .find(|one| one.id == id)
            .and_then(|copy| self.copy_as_project(&copy))
    }

    /// A Job in `repository` refused a copy that does not list it: its members are the other Mac's to change.
    pub(crate) fn copy_not_member(&self, repository: &str) -> Refusal {
        let said = format!("This project is kept on your other Mac and does not list {repository}. Add it to the project there first.");
        Refusal::Unacceptable(WireError::raised(COPY_NOT_MEMBER, said, self.run_id()))
    }

    fn copy_as_project(&self, copy: &Project) -> Option<core_model::Project> {
        let members = self.members_read(&copy.members).ok()?;
        let values = match &copy.values {
            Some(set) => self.values_read(set).ok()?,
            None => core_model::Values::default(),
        };
        // Joined by address and the login the holder picked: what a read of it, and a claim from here, need.
        let store = copy.store.as_ref().and_then(|store| {
            let team = store.team.as_deref().map(StoreTeam::new).transpose().ok()?;
            Some(ProjectStore { account_id: store.account_id, ..ProjectStore::joined(StoreAddress::new(&store.address).ok()?, team) })
        });
        let project = self.project_declared(&copy.id, &copy.name, members).ok()?;
        Some(project.with_values(values).with_shared_with_macs(copy.shared_with_macs).with_store(store))
    }
}
