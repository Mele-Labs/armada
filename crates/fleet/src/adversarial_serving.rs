//! Fleet's side of the adversarial review's three operations: each is a thin call, so the engine that
//! runs a review and settles a finding has its own place and these stay the seam. The engine fills the
//! bodies below; until then each answers the subject as unknown.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{AdversarialReviews, Refusal};
use ipc::{AdversarialReview, DismissFinding, SendFinding, StartAdversarialReview, WireError};

use crate::daemon::Fleet;

/// No Job has the subject's id, or the Job has no plan. A 404.
const REVIEW_SUBJECT_UNKNOWN: &str = "fleet.review_subject_unknown";

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
    // The engine fills this: it answers no subject until it does.
    pub(crate) async fn adversarial_review_started(self: Arc<Self>, _start: StartAdversarialReview) -> Result<AdversarialReview, Refusal> {
        Err(self.review_subject_unknown())
    }

    // The engine fills this: it answers no subject until it does.
    pub(crate) async fn finding_sent(self: Arc<Self>, _send: SendFinding) -> Result<AdversarialReview, Refusal> {
        Err(self.review_subject_unknown())
    }

    // The engine fills this: it answers no subject until it does.
    pub(crate) async fn finding_dismissed(self: Arc<Self>, _dismiss: DismissFinding) -> Result<AdversarialReview, Refusal> {
        Err(self.review_subject_unknown())
    }

    fn review_subject_unknown(&self) -> Refusal {
        Refusal::NotFound(WireError::raised(REVIEW_SUBJECT_UNKNOWN, "no such Job, or it has no plan", self.run_id()))
    }
}

impl<H, V, W> AdversarialReviews for Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    async fn start_adversarial_review(self: Arc<Self>, start: StartAdversarialReview) -> Result<AdversarialReview, Refusal> {
        self.adversarial_review_started(start).await
    }

    async fn send_finding(self: Arc<Self>, send: SendFinding) -> Result<AdversarialReview, Refusal> {
        self.finding_sent(send).await
    }

    async fn dismiss_finding(self: Arc<Self>, dismiss: DismissFinding) -> Result<AdversarialReview, Refusal> {
        self.finding_dismissed(dismiss).await
    }
}
