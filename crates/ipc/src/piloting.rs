//! How a taken-over Job's pilot is ended, and how its row says so.
//! `docs/concepts/pilot.md`. Since 23.50.

use serde::{Deserialize, Serialize};

use crate::Instant;

/// What a person says as they end a pilot by attesting or superseding. Never
/// required: the act is the record, and the words are for whoever reads it.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PilotNote {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// A Job's pilot on its row, **beside the status and never instead of it**.
///
/// Present on a `piloted` Job, and still present once the pilot ended, because
/// `exit` is what tells a Job a person attested from one that passed its gates.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Piloted {
    /// `take_over` or `restart_step`. A string, for [`crate::Paused::by`]'s
    /// reason: Bridge draws a chip and nothing matches on it.
    pub reason: String,
    pub since: Instant,
    /// How it ended: `submitted`, `attested` or `superseded`. **Absent while
    /// the person is still working.** `attested` is never verified, and a
    /// surface draws it as what it is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ended_at: Option<Instant>,
    /// The person's words on an attestation or a supersede.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}
