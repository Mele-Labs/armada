//! Who owns a pull request: the Sessions the pool says work it.
//! `docs/concepts/fleet.md`, *Telling the owner of a pull request*.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};

use crate::daemon::Fleet;
use crate::pull_requesting::PR;
use crate::repositories::Served;
use crate::slot_holders;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Owner {
    Session(String),
}

impl Owner {
    pub(crate) fn key(&self) -> String {
        match self {
            Owner::Session(id) => format!("session:{id}"),
        }
    }
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
    /// The pull requests someone holds that `listed` leaves out: a standing `pr` row of a Session.
    /// **What a forge's capped list cannot be trusted to show**, read by number so their owners
    /// are still told.
    pub(crate) async fn held_beyond(&self, served: &Served, listed: &[u64]) -> Vec<u64> {
        let manifest = served.manifest().id().as_str().to_string();
        let store = self.store().lock().await;
        let mut held: Vec<u64> = store
            .standing_in_order(PR, &manifest, None)
            .unwrap_or_default()
            .into_iter()
            .filter_map(|row| row.target.parse().ok())
            .collect();
        held.retain(|number| !listed.contains(number));
        held.sort_unstable();
        held.dedup();
        held
    }

    /// Who holds a pull request: the Sessions the pool confirms work its branch, else the one whose
    /// `pr` row is oldest (`crate::slot_holders`). `spent` also takes a Session whose hold was
    /// used up by the merge.
    pub(crate) async fn owners_of(
        &self,
        served: &Served,
        number: u64,
        branch: &str,
        spent: bool,
    ) -> Vec<Owner> {
        let manifest = served.manifest().id().as_str().to_string();
        let store = self.store().lock().await;
        let mut owners = Vec::new();
        for id in slot_holders::sessions_on_pull(&store, &manifest, served.root(), number, branch, spent) {
            let owner = Owner::Session(id);
            if !owners.contains(&owner) {
                owners.push(owner);
            }
        }
        owners
    }

    /// Who hears news of a pull request, a merge or auto-merge turned on: [`Self::owners_of`], and
    /// every other Session whose `pr` row holds it. **A failure is told to one Job, news to each.**
    pub(crate) async fn holders_told_of(
        &self,
        served: &Served,
        number: u64,
        branch: &str,
        spent: bool,
    ) -> Vec<Owner> {
        let manifest = served.manifest().id().as_str().to_string();
        let store = self.store().lock().await;
        slot_holders::sessions_told_of_pull(&store, &manifest, served.root(), number, branch, spent)
            .into_iter()
            .map(Owner::Session)
            .collect()
    }
}
