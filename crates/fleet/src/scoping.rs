//! What a caller scoped to one repository is listed: the slots and servers its
//! Manifest owns. Absent a Manifest, every one. `#987`.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{ManifestId, ServerList, WorktreesHeld};

use crate::daemon::Fleet;

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
    /// The Manifest named, checked served; a list is never narrowed to one
    /// Fleet does not hold.
    fn scoped(&self, within: Option<ManifestId>) -> Result<Option<ManifestId>, Refusal> {
        match within {
            None => Ok(None),
            Some(named) => Ok(Some(ManifestId::from(
                self.served_named(Some(&named))?.manifest().id(),
            ))),
        }
    }

    pub(crate) fn servers_within(&self, within: Option<ManifestId>) -> Result<ServerList, Refusal> {
        let within = self.scoped(within)?;
        let mut listed = self.server_list();
        listed.servers.retain(|server| match (&within, &server.manifest_id) {
            (None, _) => true,
            (Some(named), Some(owner)) => owner == named,
            (Some(_), None) => false,
        });
        Ok(listed)
    }

    pub(crate) fn worktrees_within(
        &self,
        mut held: WorktreesHeld,
        within: Option<ManifestId>,
    ) -> Result<WorktreesHeld, Refusal> {
        let within = self.scoped(within)?;
        held.slots.retain(|one| {
            within
                .as_ref()
                .is_none_or(|named| *named == one.manifest_id)
        });
        Ok(held)
    }
}
