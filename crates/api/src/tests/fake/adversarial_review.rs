//! An adversarial review as the fake answers it, read off the one Job's record the test holds in
//! `session_row`: its plan is the subject, and the review kept on `plan.adversarial` is what a start finds
//! and what a finding is settled on. No call is made. What runs, reads and fails is `fleet`'s.

use ipc::{
    AdversarialReview, AdversarialReviewState, DismissFinding, FindingHow, FindingSettled, ReviewSubject,
    SendFinding, SessionRecord, StartAdversarialReview, WireError,
};

use super::FakeDaemon;
use crate::tests::shapes::run_id;
use crate::{AdversarialReviews, Refusal};

fn unknown() -> Refusal {
    Refusal::NotFound(WireError::raised("fleet.review_subject_unknown", "no such Job, or it has no plan", run_id()))
}

fn stale() -> Refusal {
    Refusal::IllegalMove(WireError::raised("fleet.review_subject_stale", "the plan was recorded again since", run_id()))
}

fn at() -> ipc::Instant {
    ipc::Instant::carried("2026-10-25T09:00:00.000Z")
}

impl FakeDaemon {
    /// The Job the subject names, with its plan at the version the subject read.
    fn plan_of(&self, subject: &ReviewSubject) -> Result<SessionRecord, Refusal> {
        let ReviewSubject::Plan { session_id, version } = subject;
        let row = self.session_row.lock().expect("session row").clone().filter(|row| &row.id == session_id);
        let plan_version = row.as_ref().and_then(|row| row.plan.as_ref()).map(|plan| plan.version);
        match (row, plan_version) {
            (Some(row), Some(held)) if held == *version => Ok(row),
            (Some(_), Some(_)) => Err(stale()),
            _ => Err(unknown()),
        }
    }

    /// Settle one finding on the kept review, as the engine does, and answer the review.
    fn settled(&self, subject: &ReviewSubject, finding_id: &str, how: FindingHow) -> Result<AdversarialReview, Refusal> {
        let mut row = self.plan_of(subject)?;
        let review = row.plan.as_mut().and_then(|plan| plan.adversarial.as_mut()).ok_or_else(|| finding_unknown(finding_id))?;
        let found = review.findings.iter_mut().find(|found| found.id == finding_id).ok_or_else(|| finding_unknown(finding_id))?;
        if found.settled.is_none() {
            found.settled = Some(FindingSettled { how, at: at() });
        }
        let kept = review.clone();
        *self.session_row.lock().expect("session row") = Some(row);
        Ok(kept)
    }
}

fn finding_unknown(id: &str) -> Refusal {
    Refusal::NotFound(WireError::raised("fleet.finding_unknown", format!("no finding {id} in this review"), run_id()))
}

impl AdversarialReviews for FakeDaemon {
    async fn start_adversarial_review(self: std::sync::Arc<Self>, start: StartAdversarialReview) -> Result<AdversarialReview, Refusal> {
        let row = self.plan_of(&start.subject)?;
        if let Some(kept) = row.plan.and_then(|plan| plan.adversarial).filter(|kept| kept.subject == start.subject) {
            if !start.again {
                return Ok(kept);
            }
        }
        Ok(AdversarialReview {
            subject: start.subject,
            state: AdversarialReviewState::Running,
            started_at: at(),
            finished_at: None,
            failed: None,
            findings: Vec::new(),
            cost_micros: None,
        })
    }

    async fn send_finding(self: std::sync::Arc<Self>, send: SendFinding) -> Result<AdversarialReview, Refusal> {
        self.settled(&send.subject, &send.finding_id, FindingHow::Sent)
    }

    async fn dismiss_finding(self: std::sync::Arc<Self>, dismiss: DismissFinding) -> Result<AdversarialReview, Refusal> {
        self.settled(&dismiss.subject, &dismiss.finding_id, FindingHow::Dismissed)
    }
}
