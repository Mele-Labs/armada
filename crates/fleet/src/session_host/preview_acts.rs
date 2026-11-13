//! `offer_preview` and `withdraw_preview`: a Job offers its branch for the owner's preview, and takes
//! the offer back. What Fleet does with an offer is `crate::previewing`.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{OfferPreview, PreviewOffered, PreviewState, PreviewWithdrawn, WireError, WithdrawPreview};

use crate::daemon::Fleet;
use crate::previewing::MAIN;
use crate::repositories::Served;

const NO_BRANCH: &str = "fleet.preview_no_branch";
const BRANCH_UNKNOWN: &str = "fleet.preview_branch_unknown";
const NOT_AHEAD: &str = "fleet.preview_not_ahead";
const ABOUT_NEEDED: &str = "fleet.preview_about_needed";
const ABOUT_INVALID: &str = "fleet.preview_about_invalid";
const UNAVAILABLE: &str = "fleet.preview_unavailable";
const NOT_OFFERED: &str = "fleet.preview_not_offered";

/// The most an `about` says.
const ABOUT_AT_MOST: usize = 120;

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
    /// `offer_preview`: keep the Job's offer of a branch at the tip it stands at, and ask for a check.
    /// **The answer is that the check has begun**: what it found reaches the Job as a notice.
    pub(crate) async fn preview_offered(self: &Arc<Self>, caller: Option<api::Caller>, offer: OfferPreview) -> Result<PreviewOffered, Refusal> {
        let id = self.waiting_session(caller, offer.session_id.as_ref())?;
        let Some((checkout, served)) = self.preview_place() else {
            return Err(self.hosted_refusal(UNAVAILABLE, "the repository this Fleet serves has no scripts/preview to offer a branch to"));
        };
        let held = self.preview_branch(&id, offer.branch.as_deref(), Some(&served)).await?;
        {
            let store = self.store().lock().await;
            let session = store
                .session(&id)
                .map_err(|why| self.ledger_fault(why))?
                .ok_or_else(|| self.hosted_refusal(NO_BRANCH, "a job Fleet knows"))?;
            if session.state == store::SessionState::Ended {
                return Err(self.closed(&id));
            }
            if session.manifest_id.as_deref() != Some(served.manifest().id().as_str()) {
                return Err(self.hosted_refusal(UNAVAILABLE, "a job of the repository whose preview this is"));
            }
        }
        let (root, branch) = (checkout, held.clone());
        let (tip, ahead) = tokio::task::spawn_blocking(move || {
            let tip = adapters::branch_tip(&root.to_string_lossy(), &branch)?;
            let ahead = !adapters::onto_base::is_ancestor(&root, &tip, MAIN);
            Some((tip, ahead))
        })
        .await
        .ok()
        .flatten()
        .ok_or_else(|| self.hosted_refusal(BRANCH_UNKNOWN, &format!("a branch this repository holds: {held} is not one")))?;
        if !ahead {
            return Err(self.hosted_refusal(NOT_AHEAD, &format!("a branch with commits main lacks: {held} has none past main")));
        }
        let about = self.preview_about(&id, &held, offer.about.as_deref()).await?;
        let at = self.now().as_str().to_string();
        self.store().lock().await.keep_preview_offered(&id, &held, &tip, &about, &at).map_err(|why| self.ledger_fault(why))?;
        self.preview_moved().await;
        self.preview_check_wanted();
        Ok(PreviewOffered { branch: held, tip, state: PreviewState::Checking })
    }

    /// `withdraw_preview`: take the Job's offer of a branch away and check the rest again.
    pub(crate) async fn preview_withdrawn(self: &Arc<Self>, caller: Option<api::Caller>, withdraw: WithdrawPreview) -> Result<PreviewWithdrawn, Refusal> {
        let id = self.waiting_session(caller, withdraw.session_id.as_ref())?;
        let served = self.preview_place().map(|(_, served)| served);
        let branch = self.preview_branch(&id, withdraw.branch.as_deref(), served.as_ref()).await?;
        let removed = self.store().lock().await.remove_preview_offered(&id, &branch).map_err(|why| self.ledger_fault(why))?;
        if !removed {
            return Err(Refusal::NotFound(WireError::raised(NOT_OFFERED, format!("{branch} is not on the preview list from this job"), self.run_id())));
        }
        self.preview_moved().await;
        self.preview_check_wanted();
        Ok(PreviewWithdrawn { branch })
    }

    /// The branch named, or the one branch the Job holds a slot on.
    async fn preview_branch(&self, id: &str, named: Option<&str>, served: Option<&Served>) -> Result<String, Refusal> {
        if let Some(named) = named.map(str::trim).filter(|named| !named.is_empty()) {
            return Ok(named.to_string());
        }
        let held = match served {
            Some(served) => crate::build_notices::branches_held(&*self.store().lock().await, served).remove(id).unwrap_or_default(),
            None => Default::default(),
        };
        let mut held = held.into_iter();
        match (held.next(), held.next()) {
            (Some(only), None) => Ok(only),
            (None, _) => Err(self.hosted_refusal(NO_BRANCH, "a branch to name in `branch`: this job holds none")),
            (Some(_), Some(_)) => Err(self.hosted_refusal(NO_BRANCH, "a branch to name in `branch`: this job holds more than one")),
        }
    }

    /// The line on what the branch holds: the one given, else the one kept from the last offer.
    async fn preview_about(&self, id: &str, branch: &str, given: Option<&str>) -> Result<String, Refusal> {
        let given = given.map(str::trim).filter(|about| !about.is_empty());
        if let Some(about) = given {
            return if about.chars().count() > ABOUT_AT_MOST || about.contains(['\n', '\r']) {
                Err(self.hosted_refusal(ABOUT_INVALID, &format!("about as one line of at most {ABOUT_AT_MOST} characters")))
            } else {
                Ok(about.to_string())
            };
        }
        let kept = self.store().lock().await.preview_offered_of(id, branch).map_err(|why| self.ledger_fault(why))?;
        kept.map(|one| one.about).ok_or_else(|| self.hosted_refusal(ABOUT_NEEDED, "about, one line on what this branch holds, with a first offer of it"))
    }
}
