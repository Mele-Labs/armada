//! The session ledger on the wire: what a harness reports about a session, and
//! what Fleet answers and publishes about one. `docs/concepts/session.md`.
//!
//! **Intake is Armada's own shape and names no harness's event.** A mod, a
//! Bridge-hosted session and a script all send the same [`SessionReport`], so
//! nothing past the adapter that sends it knows what a session was run in.
//!
//! **No instant is reported.** Fleet stamps each fact as it arrives, so a clock
//! on the sender's side is never compared with Fleet's.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::ids::{Instant, ManifestId};

/// A session, by the id its harness knows it by.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SessionId(String);

impl SessionId {
    pub fn carried(value: impl Into<String>) -> Self {
        SessionId(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// What started a session.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionOrigin {
    /// Run by a person in a terminal, and reported by the harness it runs in.
    #[default]
    Terminal,
    /// Hosted by Bridge.
    Bridge,
}

/// Whether a session is still running.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionState {
    Live,
    Ended,
}

/// Where an attachment stands, for every kind of one: a slot let go and a need
/// whose branch landed are different words and the same question.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttachmentState {
    /// In force: the slot is held, the pull request is open.
    Standing,
    /// Used and done with: a merged pull request, a need whose branch landed.
    Spent,
    /// Let go before it was used: a released slot.
    GivenBack,
}

/// What a row of the ledger is held by. Not a foreign key to either table.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HolderKind {
    Session,
    Job,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Holder {
    pub kind: HolderKind,
    pub id: String,
}

/// The figures a session last reported. Each is absent until reported.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionUsage {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_tokens: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_window: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_micros: Option<u64>,
}

/// One thing a harness says a session took or did. `kind` is open text: the
/// ledger keeps a word it has not met, and Fleet gives only `slot` and
/// `branch` a meaning of their own.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttachmentReport {
    pub kind: String,
    pub target: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub detail: BTreeMap<String, String>,
}

/// Which attachment a fact is about.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttachmentNamed {
    pub kind: String,
    pub target: String,
}

/// One fact about a session, as it happened.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SessionFact {
    /// The session began, or a resumed one began again. Fleet resolves `cwd` to
    /// a repository and to the pool slot it stands in.
    Started {
        cwd: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        title: Option<String>,
        #[serde(default)]
        origin: SessionOrigin,
        /// The version of the `armada` mod reporting, so a session whose mod is older than the
        /// repository's can be marked. Absent from a mod that predates it. Since 23.62.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        mod_version: Option<String>,
    },
    /// A title. **`named`** says a person chose it, in the terminal's `/rename`:
    /// it replaces whatever title there is. Otherwise it is the first prompt's
    /// line, kept only where the session has no title yet. `named` since 23.52.
    Titled {
        title: String,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        named: bool,
    },
    /// The session's directory changed.
    Moved {
        cwd: String,
    },
    Attached {
        attachment: AttachmentReport,
    },
    /// An attachment is over, as `spent` or `given_back`.
    Settled {
        attachment: AttachmentNamed,
        state: AttachmentState,
    },
    Measured {
        usage: SessionUsage,
    },
    /// What the terminal runs on: its model, its effort, its permission mode
    /// and the commands it lists. **A field left out is unchanged**, and an
    /// empty `commands` is no new list. Since 23.53.
    Tuned {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        model: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        effort: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        mode: Option<crate::hosted_sessions::SessionMode>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        commands: Vec<TerminalCommand>,
        /// As on `started`. Since 23.62.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        mod_version: Option<String>,
    },
    TurnCompleted,
    Ended {
        reason: String,
    },
}

/// A command a terminal session lists, for `/` to offer.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminalCommand {
    pub name: String,
    #[serde(default)]
    pub says: String,
}

/// What a terminal session runs on, as its mod last said. **Read-only for the
/// mode**: the mods API cannot switch a live session's permission mode
/// (spike 27). Since 23.53.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminalFacts {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effort: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<crate::hosted_sessions::SessionMode>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub commands: Vec<TerminalCommand>,
    /// Whether its mod asked within the last ten seconds, which is what a
    /// message sent to it needs. A live session that is not listening is one
    /// nothing can be said to. Since 23.61.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub listening: bool,
}

/// What a harness sends Fleet. `POST /sessions/report`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionReport {
    pub harness: String,
    pub session_id: SessionId,
    pub fact: SessionFact,
}

/// `rename_session`: a person gave a session a name in Bridge. Since 23.52.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenameSession {
    pub session_id: SessionId,
    pub title: String,
}

/// One row of the ledger as the wire carries it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Attachment {
    pub kind: String,
    /// The repository a slot or a branch is of; absent where it is of none
    /// Fleet serves.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub manifest_id: Option<ManifestId>,
    pub target: String,
    pub state: AttachmentState,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub detail: BTreeMap<String, String>,
    pub since: Instant,
    pub changed_at: Instant,
}

/// A session and everything it holds. Answered by `report_session`, listed by
/// `list_sessions`, and published whole as `session.changed`, so a client
/// replaces the row rather than patching it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionRecord {
    pub id: SessionId,
    pub harness: String,
    pub origin: SessionOrigin,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub manifest_id: Option<ManifestId>,
    pub cwd: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub state: SessionState,
    pub started_at: Instant,
    pub last_seen_at: Instant,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_turn_at: Option<Instant>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ended_at: Option<Instant>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_reason: Option<String>,
    pub usage: SessionUsage,
    pub attachments: Vec<Attachment>,
    /// What a session Bridge hosts carries beyond a terminal's. Absent on one
    /// a harness reports. Since 23.49.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hosted: Option<crate::hosted_sessions::HostedFacts>,
    /// What a session run in a terminal runs on, once its mod has said. Since 23.53.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub terminal: Option<TerminalFacts>,
    /// A session run in a terminal whose `armada` mod is older than the one the repository holds,
    /// or reported no version at all. Absent otherwise, and where Fleet cannot read the
    /// repository's. Since 23.62.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub mod_out_of_date: bool,
}

/// `list_sessions`: the most recently seen first.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionList {
    pub sessions: Vec<SessionRecord>,
}

/// One holder of what `who_owns` was asked about, and its row.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ownership {
    pub holder: Holder,
    pub attachment: Attachment,
    /// The holder's title, where it is a session that has one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

/// `who_owns`: every holder of the thing asked about, those still holding it
/// first. **Empty is an answer**: nothing the ledger knows holds it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Owners {
    pub holders: Vec<Ownership>,
}
