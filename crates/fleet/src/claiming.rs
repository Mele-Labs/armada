//! An agent takes a pull request nobody is working. The pull watch attaches a pull request to
//! whoever held its branch; this is for the one it could not place, which a Session or Job that
//! notices it claims for itself. `claim_pull_request`.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{Caller, Refusal};
use core_model::JobId;
use ipc::{ClaimPullRequest, ManifestId, PullRequestClaimed};
use store::{AttachmentState, Holder, KeptAttachment};

use crate::daemon::Fleet;
use crate::pull_owners::Owner;
use crate::pull_requesting::PR;

const NOT_CLAIMABLE: &str = "fleet.pull_request_not_claimable";

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
    pub(crate) async fn pull_request_claimed(
        &self,
        caller: Option<Caller>,
        claim: ClaimPullRequest,
    ) -> Result<PullRequestClaimed, Refusal> {
        let refuse = |why: String| self.hosted_refusal(NOT_CLAIMABLE, &why);
        // Placed by the connection: a hosted session, then a Drone's Job, then the id a terminal
        // session passes.
        let me = match caller.as_ref() {
            Some(caller) => self
                .session_holding(caller)
                .map(Owner::Session)
                .or_else(|| self.caller_of(caller).ok().map(Owner::Job)),
            None => None,
        }
        .or_else(|| {
            claim
                .session_id
                .as_ref()
                .map(|id| id.as_str().trim().to_string())
                .filter(|id| !id.is_empty())
                .map(Owner::Session)
        })
        .ok_or_else(|| refuse("a pull request is claimed by a session or a Job, and this call is neither".into()))?;
        let served = match &me {
            Owner::Session(id) => {
                let manifest = self
                    .store()
                    .lock()
                    .await
                    .session(id)
                    .map_err(|why| self.ledger_fault(why))?
                    .and_then(|session| session.manifest_id)
                    .ok_or_else(|| refuse(format!("session {id} is in no repository Fleet serves")))?;
                self.served_named(Some(&ManifestId::carried(&manifest)))?
            }
            Owner::Job(job) => self.served_by_id(job).map_err(|why| self.refusal(why))?,
        };
        let root = served.root().to_string();
        let number = claim.number;
        let pull = self
            .forge_asked(&root, |vcs: &V, root: &str| vcs.pull_watch(root))
            .await
            .unwrap_or_default()
            .into_iter()
            .find(|pull| pull.number == number)
            .ok_or_else(|| refuse(format!("#{number} is not an open pull request here")))?;
        let (branch, url) = (
            pull.branch.as_written().to_string(),
            pull.url.as_written().to_string(),
        );
        for owner in self.owners_of(&served, number, &branch, false).await {
            if owner == me {
                continue;
            }
            let live = match &owner {
                Owner::Session(_) => true,
                Owner::Job(job) => self.job_is_live(job).await,
            };
            if live {
                return Err(refuse(format!(
                    "#{number} is held by {}, who is still working",
                    owner.key()
                )));
            }
        }
        let manifest = served.manifest().id().as_str().to_string();
        let now = self.now().as_str().to_string();
        let holder = match &me {
            Owner::Session(id) => Holder::session(id),
            Owner::Job(job) => Holder::job(job.as_str()),
        };
        let mut store = self.store().lock().await;
        // What an ended holder still has standing is given back to it.
        for row in store
            .attachments_at(PR, &number.to_string(), Some(&manifest))
            .unwrap_or_default()
        {
            if row.state == AttachmentState::Standing && row.holder != holder {
                let _ = store.settle(
                    &row.holder,
                    PR,
                    &manifest,
                    &number.to_string(),
                    AttachmentState::GivenBack,
                    &now,
                );
            }
        }
        let changed = store
            .attach(
                &KeptAttachment {
                    holder: holder.clone(),
                    kind: PR.into(),
                    manifest_id: manifest,
                    target: number.to_string(),
                    state: AttachmentState::Standing,
                    detail: [
                        ("state".to_string(), "open".to_string()),
                        ("branch".to_string(), branch.clone()),
                        ("address".to_string(), url.clone()),
                    ]
                    .into(),
                    since: now.clone(),
                    changed_at: now,
                },
                false,
            )
            .map_err(|why| self.ledger_fault(why))?;
        let record = match (&me, changed) {
            (Owner::Session(id), true) => store
                .session(id)
                .ok()
                .flatten()
                .and_then(|session| self.ledger_row(&store, &session).ok()),
            _ => None,
        };
        drop(store);
        if let Some(record) = record {
            self.publish(ipc::Event::SessionChanged(record));
        }
        Ok(PullRequestClaimed {
            number,
            branch,
            url,
            holder_kind: match me {
                Owner::Session(_) => "session",
                Owner::Job(_) => "job",
            }
            .into(),
            holder_id: match &me {
                Owner::Session(id) => id.clone(),
                Owner::Job(job) => job.as_str().to_string(),
            },
        })
    }

    async fn job_is_live(&self, job: &JobId) -> bool {
        self.job_detail(ipc::JobId::from(job))
            .await
            .is_ok_and(|detail| !detail.job.status.domain().is_terminal())
    }
}
