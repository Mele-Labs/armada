//! An agent takes a pull request nobody is working. The pull watch attaches a pull request to
//! whoever held its branch; this is for the one it could not place, which a Session that
//! notices it claims for itself. `claim_pull_request`.

use adapter_traits::{AgentHarness, Delivery, PullRequestStanding, Vcs, WorkProduct};
use api::{Caller, Refusal};
use ipc::{ClaimPullRequest, ManifestId, PullRequestClaimed};
use store::{AttachmentState, Holder, KeptAttachment};

use crate::daemon::Fleet;
use crate::pull_owners::Owner;
use crate::pull_requesting::PR;
use crate::red_claim::Taker;

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
        // Placed by the connection: a hosted session, then the id a terminal session passes.
        let id = caller
            .as_ref()
            .and_then(|caller| self.session_holding(caller))
            .or_else(|| {
                claim
                    .session_id
                    .as_ref()
                    .map(|id| id.as_str().trim().to_string())
                    .filter(|id| !id.is_empty())
            })
            .ok_or_else(|| refuse("a pull request is claimed by a job, and this call is not one".into()))?;
        let me = Owner::Session(id.clone());
        let manifest = self
            .store()
            .lock()
            .await
            .session(&id)
            .map_err(|why| self.ledger_fault(why))?
            .and_then(|session| session.manifest_id)
            .ok_or_else(|| refuse(format!("job {id} is in no repository Fleet serves")))?;
        let served = self.served_named(Some(&ManifestId::carried(&manifest)))?;
        let root = served.root().to_string();
        let number = claim.number;
        // **Asked for by its number**, not found in the watch: the watch reads the hundred updated
        // last, and four of the owner's older drafts were refused as not open (16 Oct 2026).
        let (branch, url) = self
            .forge_asked(&root, move |vcs: &V, root: &str| vcs.pull_request_facts(root, &number.to_string()))
            .await
            .filter(|facts| matches!(facts.standing, PullRequestStanding::Open | PullRequestStanding::Draft))
            .map(|facts| (facts.branch, facts.url))
            .ok_or_else(|| refuse(format!("#{number} is not an open pull request here")))?;
        if let Some(owner) = self
            .owners_of(&served, number, &branch, false)
            .await
            .into_iter()
            .find(|owner| *owner != me)
        {
            return Err(refuse(format!(
                "#{number} is held by {}, who is still working",
                owner.key()
            )));
        }
        let manifest = served.manifest().id().as_str().to_string();
        let now = self.now().as_str().to_string();
        let holder = Holder::session(&id);
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
        let record = if changed {
            store
                .session(&id)
                .ok()
                .flatten()
                .and_then(|session| self.ledger_row(&store, &session).ok())
        } else {
            None
        };
        drop(store);
        if let Some(record) = record {
            self.publish(ipc::Event::SessionChanged(record));
        }
        // **Never the claim's to fail**: a store that is off, signed out or out of reach leaves
        // the claim made here and the row unwritten.
        let taker = Taker::Session(id.clone());
        let _ = self.pull_request_registered(&root, number, &taker).await;
        let auto_merge_refused = self.auto_land_attached(&id, served.manifest().id().as_str(), number).await;
        Ok(PullRequestClaimed {
            number,
            branch,
            url,
            holder_id: id,
            auto_merge_refused,
        })
    }
}
