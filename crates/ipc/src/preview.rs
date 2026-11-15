//! The branches Jobs offer for the owner's preview, and the one Refresh row Bridge draws from them.
//!
//! **Fleet keeps each offer and checks the set**: that the branches merge with main and with each
//! other, and compile. `docs/concepts/fleet.md`, *Offering a branch for the preview*.

use serde::{Deserialize, Serialize};

use crate::{Instant, SessionId};

/// `offer_preview`: the Job offers a branch for the owner's preview. Answered with [`PreviewOffered`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OfferPreview {
    /// Which session. **Read only where the connection places no hosted session**, as `AllDone`'s.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionId>,
    /// The branch. Absent is the branch of the slot the Job holds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    /// One line on what the branch holds, at most 120 characters. **Required on a Job's first offer of a
    /// branch**; a re-offer with none keeps the one it had.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub about: Option<String>,
    /// One sentence on what the owner should look at once the preview holds the branch. A re-offer with none
    /// keeps the one it had.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub look: Option<String>,
    /// How to get there, in order. A re-offer with none keeps the ones it had; an empty list clears them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub steps: Option<Vec<PreviewStep>>,
}

/// One step of a preview note: what the owner does, and the act that takes him there.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreviewStep {
    /// What he does, as he reads it: "Press Projects", "Look at the toast pile".
    pub say: String,
    /// An act's id in the acts registry (`crates/core-model/domain/actions.toml`). Bridge shows its key, and
    /// Go runs it. Absent for a step that goes nowhere.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub act: Option<String>,
}

/// Where Fleet's check of an offer stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PreviewState {
    /// Waiting for the check, or in it.
    Checking,
    /// It merges with main and the offers before it, and compiles.
    Ready,
    /// It does not: `why` and `files` say how.
    Failed,
}

/// What `offer_preview` answers: the offer is kept at this tip, and its check has begun.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreviewOffered {
    pub branch: String,
    /// The branch's tip, in full, as it stands now.
    pub tip: String,
    /// Always `checking`: what the check found reaches the Job as a notice.
    pub state: PreviewState,
}

/// `withdraw_preview`: the Job takes an offer back. Answered with [`PreviewWithdrawn`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WithdrawPreview {
    /// Which session. **Read only where the connection places no hosted session**, as `AllDone`'s.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionId>,
    /// The branch. Absent is the branch of the slot the Job holds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
}

/// What `withdraw_preview` answers.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreviewWithdrawn {
    pub branch: String,
}

/// One offer, as Bridge draws its row.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreviewOfferRow {
    pub branch: String,
    /// Where the Job offered it, in full.
    pub tip: String,
    pub session_id: SessionId,
    /// The Job's title, or its id where it has none.
    pub title: String,
    /// The Job's line on what the branch holds.
    pub about: String,
    pub offered_at: Instant,
    pub state: PreviewState,
    /// The first compiler error or conflict that failed it. Present only on a `failed` offer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub why: Option<String>,
    /// The files that failure names.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub files: Vec<String>,
    /// The running build is the preview, and holds this branch at this tip.
    pub in_build: bool,
}

/// The one Refresh row: the ready offers the running build does not hold yet.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreviewRefresh {
    /// A stable hash of the ready (branch, tip) set. `skip_preview_refresh` names it.
    pub id: String,
    /// The branches Refresh would bring in, in offer order.
    pub branches: Vec<String>,
}

/// What `get_preview_offers` answers, and `preview_offers.changed` carries whole.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreviewOffers {
    /// Oldest offer first.
    pub offers: Vec<PreviewOfferRow>,
    /// Absent when every ready offer is in the build, or the row was skipped.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refresh: Option<PreviewRefresh>,
}

/// One Job's note in the build the owner is running: what its branch brought in, and what to look at.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildNote {
    pub branch: String,
    /// The tip the build holds, in full. Looked at is kept for this (branch, tip).
    pub tip: String,
    pub session_id: SessionId,
    /// The Job's title, or its id where it has none.
    pub title: String,
    pub about: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub look: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub steps: Vec<PreviewStep>,
    /// The Job has ended: feedback starts a new Job instead of going to its thread.
    pub job_ended: bool,
    /// The owner marked it looked at.
    pub looked_at: bool,
    /// What he made of it, where he said. Since 15 Nov 2026; a note marked before then is looked at with none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verdict: Option<NoteVerdict>,
}

/// What the owner made of a note when he looked at it. Its Job is told.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NoteVerdict {
    /// The work is fine as it stands.
    Good,
    /// The work needs changes: his words on it follow through `send_build_note_feedback`.
    NeedsWork,
}

/// What `get_build_notes` answers, and `build_notes.changed` carries whole: a note for each offered branch the
/// running build holds. Empty off the preview.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildNotes {
    /// The running build's commit; absent off the preview.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub commit: Option<String>,
    /// In the order the branches were offered.
    pub notes: Vec<BuildNote>,
}

/// `mark_build_note_looked_at`'s body: the owner marks the note of this (branch, tip) looked at, and says what he
/// made of it. A later mark changes the verdict and keeps the first time.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarkBuildNoteLookedAt {
    pub branch: String,
    pub tip: String,
    /// Since 15 Nov 2026. Absent keeps the verdict the note has.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verdict: Option<NoteVerdict>,
}

/// `send_build_note_feedback`'s body: the owner's words on the note of this (branch, tip).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SendBuildNoteFeedback {
    pub branch: String,
    pub tip: String,
    pub words: String,
}

/// What `send_build_note_feedback` answers: the Job the words went to, and whether Fleet started it because the
/// note's Job had ended.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildNoteFeedbackSent {
    pub session_id: SessionId,
    pub started: bool,
}

/// `skip_preview_refresh`'s body: the owner skips the Refresh row with this id.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkipPreviewRefresh {
    pub id: String,
}

/// `skip_preview_refresh`'s answer.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreviewRefreshSkipped {
    pub id: String,
}
