//! Autopilot on the wire: a repository working its own backlog, what it picked, and the acts on
//! it. #2092, `docs/concepts/autopilot.md`.

use serde::{Deserialize, Serialize};

use crate::{Instant, ManifestId, SessionId, WorkflowId};

/// Whether it picks. `stopped` is the end: a new start replaces it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutopilotState {
    Running,
    Paused,
    Stopped,
}

/// The cap that stopped it picking.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutopilotCap {
    Until,
    Spend,
    Sessions,
}

/// What became of one pick.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutopilotPickState {
    /// A Session came of it: Autopilot was sure, or a person dispatched it from the pile.
    Started,
    /// Autopilot was unsure, and left it for a person with its reason.
    SetAside,
    /// Its Session had not run when Autopilot was stopped, and was closed.
    Withdrawn,
    /// A person put it away from the pile.
    Dismissed,
}

/// One pick. `workflow_id` is what the picker chose, absent where it was unsure of that too.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutopilotPick {
    pub pick_id: String,
    /// The issue's own name in its tracker: `#123`, `ABC-123`.
    pub issue_reference: String,
    pub issue_title: String,
    pub issue_url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_id: Option<WorkflowId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionId>,
    pub state: AutopilotPickState,
    /// The picker's reason, in its words.
    pub reason: String,
    pub at: Instant,
}

/// One repository's Autopilot, whole. `get_autopilot`'s answer, every act's, and `autopilot.changed`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Autopilot {
    pub manifest_id: ManifestId,
    pub instruction: String,
    pub at_once: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub until: Option<Instant>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spend_cap_micros: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_cap: Option<u32>,
    pub state: AutopilotState,
    pub started_at: Instant,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ended_at: Option<Instant>,
    /// The cap that stopped it, where one did rather than a person.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capped_by: Option<AutopilotCap>,
    /// What its Sessions have spent so far, summed from each Session's own cost.
    pub spent_micros: u64,
    /// How many Sessions it has started, the count `session_cap` is held to.
    pub sessions_started: u32,
    /// How many of its Sessions hold a place now: live, at work, and not waiting on a person.
    pub at_work: u32,
    pub picks: Vec<AutopilotPick>,
    /// Why the last pass picked nothing, in Fleet's words: the tracker would not answer, or the
    /// picker's answer was refused. Absent once a pass goes through.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fault: Option<String>,
}

/// `list_autopilots`: every repository's, stopped ones included.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutopilotList {
    pub autopilots: Vec<Autopilot>,
}

/// `propose_autopilot`: the sentence a person typed, and their own clock, so *until 7am* is theirs.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProposeAutopilot {
    pub sentence: String,
    /// RFC 3339 with the person's offset, as their machine reads it now.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub local_now: Option<String>,
}

/// The settings one sentence was read as, for the person to confirm or change. **Nothing starts
/// from this**: `start_autopilot` takes what they confirmed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutopilotSettings {
    pub instruction: String,
    pub at_once: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub until: Option<Instant>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spend_cap_micros: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_cap: Option<u32>,
}

/// `propose_autopilot`'s answer: the settings, and the words each was read from. #2105.
///
/// **The settings are flattened in**, so a client that reads a proposal as [`AutopilotSettings`]
/// still reads it, and the start form sends back the settings alone.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutopilotProposal {
    #[serde(flatten)]
    pub settings: AutopilotSettings,
    #[serde(default)]
    pub quoted: AutopilotQuoted,
}

/// Per setting, the person's own words it was read from, keyed as [`AutopilotSettings`] keys
/// them. **Absent where the sentence said nothing about it**, and where the reader quoted words
/// the sentence does not hold.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutopilotQuoted {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instruction: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at_once: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub until: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spend_cap_micros: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_cap: Option<String>,
}
