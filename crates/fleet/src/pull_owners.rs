//! Who owns a pull request: the Sessions the ledger says hold it, and the Jobs whose it is.
//! `docs/concepts/fleet.md`, *Telling the owner of a pull request*.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use core_model::{JobId, Ulid};
use store::{AttachmentState, HolderKind};

use crate::daemon::Fleet;
use crate::pull_requesting::PR;
use crate::repositories::Served;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Owner {
    Session(String),
    Job(JobId),
}

impl Owner {
    pub(crate) fn key(&self) -> String {
        match self {
            Owner::Session(id) => format!("session:{id}"),
            Owner::Job(id) => format!("job:{}", id.as_str()),
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
    /// Who holds a pull request: the Sessions the ledger says hold it (else its branch), and the
    /// Jobs whose pull request or branch it is. `spent` also takes a Session whose hold was used
    /// up by the merge.
    pub(crate) async fn owners_of(
        &self,
        served: &Served,
        number: u64,
        branch: &str,
        spent: bool,
    ) -> Vec<Owner> {
        let manifest = served.manifest().id().as_str().to_string();
        let store = self.store().lock().await;
        let held = |kind: &str, target: &str| {
            store
                .attachments_at(kind, target, Some(&manifest))
                .unwrap_or_default()
                .into_iter()
                .filter(|row| {
                    row.state == AttachmentState::Standing
                        || (spent && row.state == AttachmentState::Spent)
                })
                .map(|row| (row.holder.kind, row.holder.id))
                .collect::<Vec<_>>()
        };
        let mut holders = held(PR, &number.to_string());
        if !holders.iter().any(|(kind, _)| *kind == HolderKind::Session) {
            holders.extend(held("branch", branch));
        }
        let mut owners = Vec::new();
        let mut add = |owner: Owner| {
            if !owners.contains(&owner) {
                owners.push(owner);
            }
        };
        for (kind, id) in holders {
            match kind {
                HolderKind::Session => {
                    if store
                        .session(&id)
                        .ok()
                        .flatten()
                        .is_some_and(|it| it.state != store::SessionState::Ended)
                    {
                        add(Owner::Session(id));
                    }
                }
                HolderKind::Job => add(Owner::Job(JobId::carried(Ulid::carried(id)))),
            }
        }
        let mine = |job: &JobId| {
            self.served_by_id(job)
                .is_ok_and(|it| it.root() == served.root())
        };
        if let Some(job) = self.job_of_pull(&store, served, number) {
            add(Owner::Job(job));
        }
        for job in store.jobs_on_branch(branch).unwrap_or_default() {
            if mine(&job) {
                add(Owner::Job(job));
            }
        }
        owners
    }
}
