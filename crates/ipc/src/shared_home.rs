//! The shared home on the wire: each Kit row as it stands against the home, the acts that settle
//! one, and what this machine syncs. `docs/concepts/kit.md`, *The shared home*, and
//! `docs/concepts/account.md`, *What syncs*.
//!
//! **Shaped for Bridge's `SharedHome`**: a row is a kind, a name and one of four states, and a
//! row that differs carries both versions so the person can pick one. Nothing is merged.

use serde::{Deserialize, Serialize};

use crate::ModKind;

/// A kind the home carries, in the order What syncs lists them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HomeKind {
    Skills,
    Plugins,
    GlobalInstructions,
    SubAgents,
    Commands,
    McpServers,
    Allowlist,
    Workflows,
    Memory,
    Mods,
}

impl HomeKind {
    pub const ALL: &'static [HomeKind] = &[
        HomeKind::Skills,
        HomeKind::Plugins,
        HomeKind::GlobalInstructions,
        HomeKind::SubAgents,
        HomeKind::Commands,
        HomeKind::McpServers,
        HomeKind::Allowlist,
        HomeKind::Workflows,
        HomeKind::Memory,
        HomeKind::Mods,
    ];

    /// The word the wire spells it as, which is also its folder in the home.
    pub fn as_wire(&self) -> &'static str {
        match self {
            HomeKind::Skills => "skills",
            HomeKind::Plugins => "plugins",
            HomeKind::GlobalInstructions => "global_instructions",
            HomeKind::SubAgents => "sub_agents",
            HomeKind::Commands => "commands",
            HomeKind::McpServers => "mcp_servers",
            HomeKind::Allowlist => "allowlist",
            HomeKind::Workflows => "workflows",
            HomeKind::Memory => "memory",
            HomeKind::Mods => "mods",
        }
    }

    pub fn from_wire(word: &str) -> Option<HomeKind> {
        HomeKind::ALL.iter().copied().find(|kind| kind.as_wire() == word)
    }
}

/// How a row stands against the home.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HomeRowState {
    Same,
    /// On both sides, and the two disagree.
    Differs,
    /// Only on this machine.
    Here,
    /// Only in the home.
    Home,
}

/// One Kit row against the home.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HomeRow {
    pub kind: HomeKind,
    pub name: String,
    pub state: HomeRowState,
    /// Its own words for itself, where it carries any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub says: Option<String>,
    /// Where it comes from: a memory note's repository, the allowlist line's origin.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// A memory note's repository, by its folder's name, which names the row in an act.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,
    /// On an MCP server alone: `false` where this machine has no server of that name, so its
    /// key is not here. The home carries a name and a host and never a key.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key_here: Option<bool>,
    /// On a mod alone: the kind its `mod.toml` names, this machine's where it has one and else
    /// the home's, so a mod only the home has is listed under its own kind.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mod_kind: Option<ModKind>,
    /// This machine's version, on a row that differs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub here: Option<String>,
    /// The home's version, on a row that differs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub there: Option<String>,
}

/// Something of a kind that would not read, named rather than left out.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HomeUnreadable {
    pub kind: HomeKind,
    /// Where, as a person reads it.
    pub at: String,
    pub why: String,
}

/// Which profile's home: **a machine can hold two profiles, and each has its own.** The body of
/// `sync_shared_home`, and the query of `get_shared_home` and `get_home_kinds`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HomeProfile {
    /// The profile's login, whose `<login>/armada-home` it is.
    pub login: String,
}

/// The home this machine holds a copy of.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HomeNamed {
    /// The login the home belongs to.
    pub account: String,
    /// The repository's name under it.
    pub repository: String,
}

/// Where a home is or will be, while this machine holds no copy of it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HomePlanned {
    /// The owner: the profile's login or one of its organisations.
    pub account: String,
    /// The repository's name under it.
    pub repository: String,
    /// The forge already has it, made on another Mac: shown and not changed, and Sync clones it.
    pub exists: bool,
}

/// `get_shared_home`, and what every act on it answers with.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SharedHomeRead {
    /// Left out where this machine holds no copy yet: Sync makes and clones it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub home: Option<HomeNamed>,
    /// Where the home is or will be, served whenever `home` is left out.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub planned: Option<HomePlanned>,
    /// Every row of every kind this machine syncs, kind by kind in [`HomeKind::ALL`]'s order.
    pub rows: Vec<HomeRow>,
    pub unreadable: Vec<HomeUnreadable>,
}

/// A row only on this machine that Sync did not add, and why.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HomeLeft {
    pub kind: HomeKind,
    pub name: String,
    /// A memory note's repository, and only there.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,
    /// A sentence naming the row and why it stayed: a host, never a credential.
    pub why: String,
}

/// `sync_shared_home`: the home read again, and each row only on this machine Sync left here.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HomeSynced {
    #[serde(flatten)]
    pub read: SharedHomeRead,
    pub left: Vec<HomeLeft>,
}

/// Which row an act is on, in which profile's home.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HomeRowAt {
    /// The profile's login.
    pub login: String,
    pub kind: HomeKind,
    pub name: String,
    /// A memory note's repository, and only there.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,
}

/// Which side of a row that differs is kept.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HomeSide {
    /// This machine's, which goes into the home.
    Here,
    /// The home's, which replaces this machine's.
    There,
}

/// `choose_shared_home_side`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChooseHomeSide {
    pub row: HomeRowAt,
    pub side: HomeSide,
}

/// One kind's switch in What syncs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HomeKindSwitch {
    pub kind: HomeKind,
    pub on: bool,
}

/// `set_home_kind`: one kind switched for one profile.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetHomeKind {
    pub login: String,
    pub kind: HomeKind,
    pub on: bool,
}

/// One item switched on in What syncs on its own, whatever its kind's switch says.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct HomeItem {
    pub kind: HomeKind,
    pub name: String,
    /// A memory note's repository, and only there.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,
}

/// `set_home_item`: one item switched for one profile.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetHomeItem {
    pub login: String,
    #[serde(flatten)]
    pub item: HomeItem,
    pub on: bool,
}

/// `get_home_kinds`, and what `set_home_kind` and `set_home_item` answer with: one profile's every
/// kind, in order, off until it is turned on, and each item switched on on its own.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HomeKinds {
    pub login: String,
    pub kinds: Vec<HomeKindSwitch>,
    #[serde(default)]
    pub items: Vec<HomeItem>,
}

/// `set_home_repository`: where one profile's home is made, before it exists.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetHomeRepository {
    pub login: String,
    /// The profile's login or one of its organisations.
    pub owner: String,
    /// The repository's name.
    pub name: String,
}
