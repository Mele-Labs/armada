//! An adversarial review: one read-only model call that tries to break a thing a person is about to
//! approve, its findings pinned to the part they are about. `docs/concepts/adversarial-review.md`.
//!
//! **Only a press starts one, and a finding reaches the agent only if the person sends it.** The
//! subject is the first kind of thing it reads, a plan; a pull request or a document may become
//! another variant of [`ReviewSubject`] without the rest of the shapes here changing.
//!
//! The latest review of a Job's plan rides on [`SessionPlan::adversarial`](crate::SessionPlan), so it
//! arrives with every `session.changed` and no event of its own is needed.

use serde::{Deserialize, Serialize};

use crate::ids::Instant;
use crate::sessions::SessionId;

/// What a review read.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ReviewSubject {
    /// One version of a Job's plan.
    Plan { session_id: SessionId, version: u32 },
}

/// How far a review has got.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdversarialReviewState {
    Running,
    Done,
    /// It ended without findings that held up, and `failed` says why. **Never a result.**
    Failed,
}

/// The part of the subject a finding is about. Ids are the subject's own: for a plan, the ids of its
/// groups and tasks.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FindingAnchor {
    /// The whole subject.
    Plan,
    Group { id: String },
    Task { id: String },
}

/// How the person settled a finding.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingHow {
    /// Sent to the agent as his message.
    Sent,
    /// Dropped. The agent was told nothing.
    Dismissed,
}

/// A finding once settled.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FindingSettled {
    pub how: FindingHow,
    pub at: Instant,
}

/// One way the subject breaks, in the three parts a judge's concern has.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdversarialFinding {
    /// Unique within its review.
    pub id: String,
    pub about: FindingAnchor,
    /// What breaks, in a line. Heads the finding.
    pub consequence: String,
    pub expected: String,
    pub found: String,
    /// `path` or `path:line`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cites: Vec<String>,
    /// Absent while the finding waits.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settled: Option<FindingSettled>,
}

/// One review of one subject, going or kept.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdversarialReview {
    pub subject: ReviewSubject,
    pub state: AdversarialReviewState,
    pub started_at: Instant,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub finished_at: Option<Instant>,
    /// Why a failed review failed, in Fleet's words.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failed: Option<String>,
    /// None is an answer when the state is `done`: the review read the subject and found nothing.
    pub findings: Vec<AdversarialFinding>,
    /// What the vendor billed, in millionths of a dollar. Absent where the call ended without
    /// saying, which is not a cost of zero.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_micros: Option<u64>,
}

/// `start_adversarial_review`'s body. `POST /adversarial_review`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StartAdversarialReview {
    pub subject: ReviewSubject,
    /// Run it again, whatever is kept. Without it the review kept for this subject is answered as it
    /// stands, and only a subject nothing was kept for is run.
    #[serde(default)]
    pub again: bool,
}

/// `send_finding`'s body. `POST /adversarial_review/send`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SendFinding {
    pub subject: ReviewSubject,
    pub finding_id: String,
}

/// `dismiss_finding`'s body. `POST /adversarial_review/dismiss`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DismissFinding {
    pub subject: ReviewSubject,
    pub finding_id: String,
}
