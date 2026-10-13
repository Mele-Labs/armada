//! One repository's Helm Session as one quotable record. `#1367`.
//!
//! **The error treatment's debug payload, one subject over** —
//! `docs/contracts/design-system.md`, *The debug payload*. A bad answer could
//! only be reported by retyping the conversation, without the note, the
//! tools, the authority or the events that decided it.
//!
//! **Fields, not text**, for `errors/ErrorNotice/payload.ts`'s reason: one
//! producer writes the artifact and the expanded view renders that same
//! string. A Fleet that formatted the record and a Bridge that framed it would
//! be two producers of one artifact.
//!
//! **Read once, never polled.** The Session's thread streams to the dock as `session.row`; this
//! is one artifact taken at a moment.

use serde::{Deserialize, Serialize};

use crate::health::HelmActionAuthority;
use crate::ids::{Instant, ManifestId};
use crate::protocol_id::ProtocolId;
use crate::since::EventsSince;

/// Everything a person can carry to whoever could fix a bad Helm answer.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HelmDebugInfo {
    /// Which repository this conversation answers for.
    pub manifest_id: ManifestId,
    /// The checkout the session opens in. **A folder on this person's own
    /// disk**, which is why the surface says so before it is sent anywhere.
    pub checkout: String,
    /// The Machine setting, resolved — what this session may do, as
    /// `get_health` already answers it.
    pub authority: HelmActionAuthority,
    /// What the Session runs on: the model it was tuned to, else `default`, the agent CLI's own.
    pub model: String,
    /// The note appended to the Session's system prompt, whole, as the next process is started
    /// with it. What a Session of Helm's is told it is. Was `brief`.
    pub note: String,
    /// The MCP server Armada's own door is registered under, which prefixes
    /// every name in [`tools`](HelmDebugInfo::tools) inside the session.
    pub door: String,
    /// Every tool the door offered this session, by name, as the inventory
    /// orders them. **The roster is the answer to "no tool here reads X"**,
    /// which is the commonest complaint about an answer.
    pub tools: Vec<String>,
    /// The Helm Session's id, which is the agent CLI's own. Absent where the repository has none
    /// live: before the first message, and after a person starts fresh.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
    /// The thread, oldest first, bounded — see [`cut`](HelmDebugInfo::cut).
    pub thread: Vec<HelmDebugLine>,
    /// How many older lines the thread left out. **Said rather than implied**,
    /// the way a log tail says what it dropped: a record that is quietly short
    /// reads exactly like a session that was quiet.
    pub cut: u64,
    /// What this session's last `get_events_since` was answered, verbatim.
    ///
    /// **Kept in memory as the door answers it**, so it is absent on a session
    /// that has not polled since this Fleet started rather than on one that
    /// never polled at all.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub polled: Option<EventsSince>,
    /// Which Fleet process answered. **Fleet holds no application version**,
    /// the way Bridge holds none, so this and the protocol below are the pair
    /// that names what produced the answer.
    pub run_id: String,
    /// The protocol Fleet speaks.
    pub protocol_id: ProtocolId,
    /// When the record was taken, on Fleet's clock. **Taken, not raised**, for
    /// the debug payload's reason: nothing here happened at this instant.
    pub at: Instant,
}

/// One line of the thread, as the record holds it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HelmDebugLine {
    pub at: Instant,
    #[serde(flatten)]
    pub said: HelmDebugSaid,
}

/// What one line was.
///
/// **A narrowing of the thread's own vocabulary and not a second copy of it.**
/// `SessionRow` carries what a live viewer draws; this carries what somebody
/// reading a bad answer needs — what was asked, what came back, what was
/// called, what was refused, and where words were sent.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "line", rename_all = "snake_case")]
pub enum HelmDebugSaid {
    /// What the person typed. **Their own words**, which is half of why this
    /// record is read before it is sent.
    Asked { text: HelmDebugText },
    /// What Helm wrote back.
    Said { text: HelmDebugText },
    /// A tool it called, and what it called it on, as the thread's `tool` row has it.
    Called { tool: String, detail: String },
    /// A call the Auto gate refused, or nobody answered: why an answer stops where it does.
    Refused { tool: String, because: String },
    /// Words Helm's dock sent to a Session or a Job, and which. Never told to Helm's process.
    Sent { text: HelmDebugText, to: String },
}

/// A piece of prose, bounded.
///
/// **The bound is the record's, not the thread's.** A thread keeps what was
/// said whole; a record is pasted into an issue, and one reply of forty
/// thousand characters would be the whole artifact.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HelmDebugText {
    pub text: String,
    /// How long it was before it was cut. **Absent where nothing was cut**, so
    /// a reader holding a value can say what proportion of it they have rather
    /// than inferring it from a trailing character.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub of: Option<usize>,
}

impl HelmDebugText {
    /// `text` whole where it fits in `most`, cut with its length where it does
    /// not. **Cut on a character boundary**, so a multi-byte character does
    /// not arrive in an issue body as half of itself.
    pub fn bounded(text: &str, most: usize) -> HelmDebugText {
        let length = text.chars().count();
        if length <= most {
            return HelmDebugText {
                text: text.to_string(),
                of: None,
            };
        }
        HelmDebugText {
            text: text.chars().take(most).collect(),
            of: Some(length),
        }
    }
}
