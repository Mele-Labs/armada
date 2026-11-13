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
