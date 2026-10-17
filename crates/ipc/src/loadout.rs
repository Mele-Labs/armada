//! A session's Loadout: what its agent knows, was told, and used each turn, each item with where it
//! came from. `docs/practices/protocol.md`, *A Session's Loadout*.
//!
//! **Names, sources and counts only**, except [`LoadoutItemBody`]: never a skill's body, a prompt's
//! text, a tool's input or a file's contents on a record, a fact or the whole Loadout. What a row says
//! is read from disk where Fleet runs, one row at a time, when a person opens it.

use serde::{Deserialize, Serialize};

use crate::ids::Instant;

/// Where an item came from. Kit's `OutputStyleFacts.from` where the two overlap, except that Kit's
/// `person` is `kit` here, as the owner names it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LoadoutSource {
    /// The person's harness home.
    Kit,
    /// The Armada mod, or Armada's own rules.
    Armada,
    /// The repository's own harness files.
    Repository,
    /// Another plugin, named in `from`.
    Plugin,
    /// An MCP server, named in `from`.
    Mcp,
    /// The harness's own.
    BuiltIn,
    /// Set by an organisation: policy, managed or flag settings.
    Managed,
}

/// What an item is. The first five are what the agent knows; the last three what it was told.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LoadoutKind {
    Skill,
    Subagent,
    McpServer,
    Tool,
    MemoryFile,
    OutputStyle,
    Rules,
    Injection,
}

impl LoadoutKind {
    /// Whether the agent was told it, rather than knowing it.
    pub fn told(self) -> bool {
        matches!(self, LoadoutKind::OutputStyle | LoadoutKind::Rules | LoadoutKind::Injection)
    }
}

/// Where an MCP server or a tool stands: a server `connected`, `failed` or `pending`, a tool
/// `deferred` or `loaded`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LoadoutState {
    Connected,
    Failed,
    Pending,
    Deferred,
    Loaded,
}

/// One thing the agent knows or was told.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoadoutItem {
    pub kind: LoadoutKind,
    pub name: String,
    pub source: LoadoutSource,
    /// The plugin or MCP server it came from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<LoadoutState>,
    /// Where the engine's breakdown gives one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tokens: Option<u64>,
}

/// The whole list, as the `equipped` fact carries it.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoadoutItems {
    pub items: Vec<LoadoutItem>,
}

/// What a use was of. Ordinary built-in tools are not reported: they are already on the timeline.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LoadoutUseKind {
    Skill,
    Tool,
    Subagent,
}

/// One use in a turn: a skill called, a tool loaded or called, a subagent dispatched.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoadoutUse {
    /// When the turn it was made in began, which groups uses by turn.
    pub turn_started_at: Instant,
    pub kind: LoadoutUseKind,
    /// The skill's name, the tool's (`mcp__github__get_me`) or the subagent's type.
    pub name: String,
    pub source: LoadoutSource,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
    /// The model a subagent was dispatched on, where the dispatch named one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// The subagent that made the call. Absent for the session's own agent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub by: Option<String>,
}

/// The uses of one turn, in the order they were reported.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoadoutTurn {
    pub started_at: Instant,
    pub used: Vec<LoadoutUse>,
}

/// The Loadout in brief, on every `SessionRecord`, for the header mark and its hover card.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoadoutSummary {
    pub knows: u32,
    pub told: u32,
    /// Absent until a use is reported.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latest_turn: Option<LoadoutTurn>,
}

impl LoadoutSummary {
    pub fn of(items: &[LoadoutItem], latest_turn: Option<LoadoutTurn>) -> LoadoutSummary {
        let told = items.iter().filter(|item| item.kind.told()).count();
        LoadoutSummary {
            knows: u32::try_from(items.len() - told).unwrap_or(u32::MAX),
            told: u32::try_from(told).unwrap_or(u32::MAX),
            latest_turn,
        }
    }
}

/// `get_session_loadout`: the whole Loadout, for the overlay's side panel.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionLoadout {
    pub items: Vec<LoadoutItem>,
    /// Oldest first, the newest last, at most the last fifty.
    pub turns: Vec<LoadoutTurn>,
}

/// How much of an item's contents `read_loadout_item` carries: 256 KiB, and `truncated` past it.
pub const LOADOUT_ITEM_CAP: usize = 256 * 1024;

/// `read_loadout_item`'s query: one row of a session's Loadout, as the Loadout names it. A use's
/// `skill`, `tool` or `subagent` is asked as the kind of the same name.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoadoutItemAsked {
    pub session_id: String,
    pub kind: LoadoutKind,
    pub name: String,
    pub source: LoadoutSource,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
}

/// What an item's contents are written in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LoadoutItemFormat {
    Markdown,
    Text,
}

/// `read_loadout_item`: what one Loadout row says.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "what", rename_all = "snake_case")]
pub enum LoadoutItemBody {
    Read {
        /// Where it was read, for a person: from the repository's root, `~/…` under the home, or
        /// from the plugin's own folder. Absent for text Armada holds itself.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        path: Option<String>,
        format: LoadoutItemFormat,
        body: String,
        /// The body stops at [`LOADOUT_ITEM_CAP`] and the file goes on.
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        truncated: bool,
    },
    /// A sentence a person reads.
    NotRead { why: String },
}
