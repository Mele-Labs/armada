//! Which Session or Job claimed a pull request, kept in the central store so another machine
//! signed in to the account can tell. `docs/concepts/session.md`, *Which machine owned a pull
//! request*.
//!
//! **Written when a claim is made, read with the open pull requests, deleted when one leaves
//! them.** The write is `claim_pull_request` (a Session or a Job) and a Job opening its own
//! pull request; the read rides `notice_pulls`, one `owners` call per repository per reading;
//! the delete is the reading that no longer lists a number it listed before.
//!
//! **Nothing here can fail a claim or a delivery.** A store that cannot be reached, a profile
//! that is signed out and a repository whose switch is off all leave the row unwritten and the
//! field absent. The store is held and gated by [`Fleet::at_the_store_of`], the red claim's own.

use std::collections::BTreeMap;

use adapter_traits::{
    AgentHarness, Delivery, Holder, HolderKind, OwnerKey, StoreError, Vcs, WorkProduct,
};

use crate::daemon::Fleet;
use crate::red_claim::{owner_key_of, Taker};

/// The most keys one `GET /owners` answers, `docs/contracts/central-store.md`.
const MOST_KEYS: usize = 100;

/// A pull request's registered owner, and whether this machine wrote the row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PullOwner {
    pub(crate) holder: Holder,
    pub(crate) here: bool,
}

impl PullOwner {
    pub(crate) fn on_the_wire(&self) -> ipc::PullRequestOwner {
        ipc::PullRequestOwner {
            kind: match self.holder.kind {
                HolderKind::Session => ipc::HolderKind::Session,
                HolderKind::Job => ipc::HolderKind::Job,
            },
            id: self.holder.id.clone(),
            machine: self.holder.machine.clone(),
            this_machine: self.here,
        }
    }
}

/// The name a row carries for `machine`, which the store's own limits shortened.
fn as_written(machine: &str) -> String {
    Holder::new(HolderKind::Job, "x", machine, 0).map_or_else(|| machine.to_string(), |fitted| fitted.machine)
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
    /// Register `by` as the owner of pull request `number` in the repository at `root`.
    /// `Ok(false)` is no store applying: the switch is off, this machine is signed out or no
    /// address is set, and nothing was asked.
    pub(crate) async fn pull_request_registered(
        &self,
        root: &str,
        number: u64,
        by: &Taker,
    ) -> Result<bool, StoreError> {
        let (kind, id) = by.parts();
        let id = id.to_string();
        let since = self.unix_seconds();
        let put = self
            .at_the_store_of(root, move |opened| {
                let key = owner_key_of(&opened.slug, number)
                    .ok_or_else(|| StoreError::Refused("the pull request could not be keyed".to_string()))?;
                let holder = Holder::new(kind, &id, &opened.machine, since)
                    .ok_or_else(|| StoreError::Refused("the holder's id could not be written to the store".to_string()))?;
                opened.store.put_owner(&key, &holder)?;
                Ok(holder)
            })
            .await?;
        let Some(holder) = put else { return Ok(false) };
        // Seen at once rather than at the next visit: the hub and the panel read this.
        let mut sweep = self.sweeping().lock().await;
        if let Some(open) = sweep
            .pulls
            .get_mut(root)
            .and_then(|read| read.iter_mut().find(|one| one.pull.number == number))
        {
            open.owner = Some(PullOwner { holder, here: true });
        }
        Ok(true)
    }

    /// The registered owners among `numbers`, one `owners` call. **An empty answer is also what
    /// a store that did not answer, a signed-out machine and a switch that is off give**, since
    /// the field is absent in each.
    pub(crate) async fn owners_read(&self, root: &str, numbers: Vec<u64>) -> BTreeMap<u64, PullOwner> {
        if numbers.is_empty() {
            return BTreeMap::new();
        }
        let read = self
            .at_the_store_of(root, move |opened| {
                let keyed: BTreeMap<OwnerKey, u64> = numbers
                    .iter()
                    .filter_map(|number| Some((owner_key_of(&opened.slug, *number)?, *number)))
                    .collect();
                let keys: Vec<OwnerKey> = keyed.keys().cloned().collect();
                let mut found = BTreeMap::new();
                for chunk in keys.chunks(MOST_KEYS) {
                    for (key, holder) in opened.store.owners(chunk)? {
                        if let Some(number) = keyed.get(&key) {
                            found.insert(*number, holder);
                        }
                    }
                }
                Ok((found, as_written(&opened.machine)))
            })
            .await;
        let Ok(Some((found, machine))) = read else { return BTreeMap::new() };
        found
            .into_iter()
            .map(|(number, holder)| {
                let here = holder.machine == machine;
                (number, PullOwner { holder, here })
            })
            .collect()
    }

    /// Delete the rows of pull requests that are no longer open.
    pub(crate) async fn owners_deleted(&self, root: &str, numbers: Vec<u64>) {
        if numbers.is_empty() {
            return;
        }
        let _ = self
            .at_the_store_of(root, move |opened| {
                for number in numbers {
                    if let Some(key) = owner_key_of(&opened.slug, number) {
                        opened.store.delete_owner(&key)?;
                    }
                }
                Ok(())
            })
            .await;
    }

    /// The pull requests the last reading listed for `root` and this one does not.
    pub(crate) async fn pulls_that_left(&self, root: &str, listed: &[u64]) -> Vec<u64> {
        self.sweeping()
            .lock()
            .await
            .pulls
            .get(root)
            .map(|before| {
                before
                    .iter()
                    .map(|one| one.pull.number)
                    .filter(|number| !listed.contains(number))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// The owner the last reading found for a pull request, **as the wire carries it**. Absent
    /// where the repository's switch is off, whatever an earlier reading kept.
    pub(crate) async fn owner_known(&self, root: &str, number: u64) -> Option<ipc::PullRequestOwner> {
        if !self.registers_owners(root) {
            return None;
        }
        self.sweeping()
            .lock()
            .await
            .pulls
            .get(root)?
            .iter()
            .find(|one| one.pull.number == number)?
            .owner
            .as_ref()
            .map(PullOwner::on_the_wire)
    }
}
