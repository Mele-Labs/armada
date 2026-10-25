//! An adversarial review of a plan, and the two acts on its findings. Its own surface, for
//! [`SharedHome`](super::SharedHome)'s reason: the subject is a plan now and may be a pull request or a
//! document later, so a handler taking only this cannot reach for a Job's record.
//! `docs/concepts/adversarial-review.md`.

use std::future::Future;

use ipc::{AdversarialReview, DismissFinding, SendFinding, StartAdversarialReview};

use crate::daemon::Refusal;

pub trait AdversarialReviews: Send + Sync + 'static {
    /// `start_adversarial_review` — one read-only model call that tries to break the subject. **Answers at
    /// once**, `running`, or the review kept for the subject when `again` is not set; the call runs in a task.
    /// [`Refusal::NotFound`] (`fleet.review_subject_unknown`) for no such Job or no plan,
    /// [`Refusal::IllegalMove`] (`fleet.review_subject_stale`) for a plan version that is not the current one.
    fn start_adversarial_review(
        self: std::sync::Arc<Self>,
        start: StartAdversarialReview,
    ) -> impl Future<Output = Result<AdversarialReview, Refusal>> + Send;

    /// `send_finding` — a finding goes to the agent as the person's message, and is settled. One already
    /// settled is answered as it stands, and a review the plan was recorded past still sends: only a start
    /// is stale. [`Refusal::NotFound`] (`fleet.review_subject_unknown`) for no such Job, no plan or a version
    /// it never reached, and (`fleet.finding_unknown`) for an id the review kept for the version does not hold.
    fn send_finding(
        self: std::sync::Arc<Self>,
        send: SendFinding,
    ) -> impl Future<Output = Result<AdversarialReview, Refusal>> + Send;

    /// `dismiss_finding` — a finding is settled and the agent is told nothing. The same refusals.
    fn dismiss_finding(
        self: std::sync::Arc<Self>,
        dismiss: DismissFinding,
    ) -> impl Future<Output = Result<AdversarialReview, Refusal>> + Send;
}
