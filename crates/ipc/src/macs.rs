//! Another of his Macs: pairing it from Settings, and what its Phone Gateway hands this Fleet once paired.
//! `docs/concepts/pocket.md`, *Another Mac*.
//!
//! What leaves the Mac that holds a Job has passed that Fleet's share filter first. What arrives is kept
//! by this Fleet with the sending Mac's id, and served beside its own.

use serde::{Deserialize, Serialize};

use crate::{BugReport, Event, SessionRecord};

/// `POST /admin/macs/pair/start` on the Gateway: the one-time link the other Mac's Settings takes.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MacPairLink {
    pub link: String,
    /// Unix seconds. After this the link is refused, and pairing is started again.
    pub expires_at: i64,
}

/// `POST /admin/macs/pair/claim`: a link pasted from the other Mac. Answers the [`crate::Mac`] it
/// names, waiting on a Confirm there.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClaimMacLink {
    pub link: String,
}

/// `POST /admin/macs/pair/confirm` and `POST /admin/macs/forget`: the Mac pressed on.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MacNamed {
    pub mac: String,
}

/// `PUT /other_macs/sessions` on Fleet: every Job row Mac `mac` shares, replacing all this Fleet held from it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OtherMacSessions {
    pub mac: String,
    pub sessions: Vec<SessionRecord>,
}

/// `PUT /other_macs/bug_reports` on Fleet: every report Mac `mac` shares, replacing all this Fleet held from it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OtherMacBugReports {
    pub mac: String,
    pub reports: Vec<BugReport>,
}

/// `POST /other_macs/event` on Fleet: one event that crossed from Mac `mac`, for this Fleet to keep and publish.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OtherMacEvent {
    pub mac: String,
    pub event: Event,
}
