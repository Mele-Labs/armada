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
    /// The sketch or page he approved that the branch builds, so his Ready to try card sets it beside the built
    /// work. A re-offer with none keeps the one it had. Since 15 Nov 2026.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approved: Option<OfferApproved>,
    /// Shots of the built work, at most [`PICTURES_AT_MOST`], each a path to a picture, relative to the Job's
    /// checkout or absolute. Read through `get_repository_file`. A re-offer with none keeps the ones it had; an
    /// empty list clears them. Since 15 Nov 2026.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pictures: Option<Vec<String>>,
    /// His words that asked for the work, quoted on the card, until the request record carries them. A re-offer
    /// with none keeps them; empty words clear them. Since 15 Nov 2026.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub words: Option<String>,
}

/// The most pictures an offer carries: the card draws them side by side.
pub const PICTURES_AT_MOST: usize = 4;

/// What an offer names as the thing he approved: one of the Job's artifacts, by its id or its title, at a revision.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OfferApproved {
    /// A `JobArtifact` id, or its title.
    pub artifact: String,
    /// The revision he approved. Absent is the newest.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revision: Option<u32>,
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
    /// The Mac the offer lives on, by id. Absent is this Mac. Since 19 Nov 2026.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mac: Option<String>,
}

/// The one Refresh row: the ready offers and picks the running build does not hold yet, and what it keeps.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreviewRefresh {
    /// A stable hash of the ready (branch, tip) set. `skip_preview_refresh` names it.
    pub id: String,
    /// The branches Refresh would bring in, in offer order, then the picks not yet built.
    pub branches: Vec<String>,
    /// The branches the running build holds that Refresh carries forward, in the order it merged them. Since 23 Nov 2026.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub keeps: Vec<PreviewKept>,
}

/// A branch the running build holds that a refresh keeps, named as Bridge names it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreviewKept {
    pub branch: String,
    /// The title of the Job that held it last, or the branch where no Job here did.
    pub title: String,
}

/// What `get_preview_offers` answers, and `preview_offers.changed` carries whole.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreviewOffers {
    /// Oldest offer first.
    pub offers: Vec<PreviewOfferRow>,
    /// Absent when every ready offer is in the build, or the row was skipped.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refresh: Option<PreviewRefresh>,
    /// The Macs whose offers this answer leaves out, and why. Since 19 Nov 2026.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub left_out: Vec<PreviewLeftOut>,
}

/// A paired Mac whose ready offers are not in this preview's refresh, and the sentence that says why.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreviewLeftOut {
    /// The Mac's id.
    pub mac: String,
    pub name: String,
    pub why: String,
}

/// `PUT /other_macs/preview_offers` on Fleet: every ready offer Mac `mac` shares, replacing all this Fleet held
/// from it. Each row is kept with `mac` set to the sender.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OtherMacPreviewOffers {
    pub mac: String,
    pub offers: Vec<PreviewOfferRow>,
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

/// A Ready to try card: one a (branch, tip) a Job offered, drawn in that Job's thread at the offer. It is the
/// offer's build note seen from the thread, so its answer is the note's verdict. `docs/concepts/fleet.md`,
/// *Ready to try*.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThreadReadyToTry {
    /// `try:<branch>@<tip>`.
    pub id: String,
    /// Where the thread draws it: when the Job offered it, or when a reminder brought it back.
    pub at: Instant,
    pub offered_at: Instant,
    pub branch: String,
    /// In full.
    pub tip: String,
    pub about: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub look: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub steps: Vec<PreviewStep>,
    /// What he approved that this builds, as the offer named it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approved: Option<ReadyApproved>,
    /// Paths of the shots the Job attached, as Fleet resolved them: read each through `get_repository_file`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pictures: Vec<String>,
    /// His words that asked for the work.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub words: Option<String>,
    /// The build he runs is the preview, and holds this branch at this tip.
    pub in_build: bool,
    pub state: ReadyState,
    /// What he said, on an `answered` card. The build note's verdict, so a mark from the notes answers it too.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verdict: Option<NoteVerdict>,
    /// He pressed Skip: approved on the walk, landed untried. The verdict is still `good`; this is what
    /// a card read again says he did.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub skipped: bool,
    /// When it comes back, on a `later` card.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub later_until: Option<Instant>,
}

/// The artifact a Ready to try card sets beside the build: what he approved, at the revision he approved.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReadyApproved {
    /// The `JobArtifact` id: `sketch:<title>`, a page's path, or `draft:<title>`.
    pub artifact: String,
    pub title: String,
    pub form: crate::ArtifactForm,
    pub revision: u32,
    /// When he approved it, where the ledger says.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approved_at: Option<Instant>,
}

/// Where a Ready to try card stands. **One state at a time, in this order**: what he said stands over what
/// the Job did since.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReadyState {
    /// Waiting on him.
    Open,
    /// He asked to be reminded: it comes back at `later_until`.
    Later,
    /// He said Looks good or Needs rework, here or on the build note.
    Answered,
    /// The Job withdrew the offer.
    Withdrawn,
    /// The Job offered the branch again at a newer tip, whose card is the one that asks.
    Replaced,
}

/// `answer_ready_to_try`'s body: his answer on the card of this (branch, tip) in this Job's thread.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnswerReadyToTry {
    pub session_id: SessionId,
    pub branch: String,
    pub tip: String,
    pub answer: ReadyAnswer,
}

/// What he pressed on a Ready to try card.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ReadyAnswer {
    /// Looks good: the note's verdict is `good`, and the Job is told to land.
    LooksGood,
    /// Needs rework: the note's verdict is `needs_work`, and his words go as `send_build_note_feedback` sends them.
    NeedsRework { words: String },
    /// Remind me: the card comes back at `until`, which the client works out from the Later choice he pressed.
    Remind { until: Instant },
    /// Skip: he approved the look on the walk and lands it untried. Closes the step as Looks good does,
    /// and the Job is told to land.
    Skip,
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
