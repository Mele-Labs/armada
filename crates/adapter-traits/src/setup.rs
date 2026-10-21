//! The setup a person already has, read so that it can be **shown**.
//!
//! Somebody arrives at Armada with a way of working already built: skills they
//! wrote, plugins they installed, a global file saying how they want an agent
//! to behave, servers they connected. It sits one directory away and nothing
//! read it. [`HarnessSetup`] is that read, and one implementation per harness
//! is the seam: which files those are, and how a harness spells them, is a
//! question only `adapters` answers.

//! # Reading is not granting
//!
//! **Nothing an implementation returns may reach a Drone by having been read.**
//! [`SetupItem`] carries a name, the item's own words for itself and where it
//! came from. For a server a person connected, those words are the program's
//! own file name or the host it is at — **enough to tell two apart, and never
//! what comes after either**: no argument list, no query string, no userinfo,
//! no environment. What survives could not start the server it names, so an
//! import built out of an [`Inventory`] would produce something that does not
//! run rather than something that quietly works. Allowing stays what it is: a
//! separate act on a row a person added themselves, and `KitServer::added`
//! takes no reach.

//! [`SetupFiles`] has **no write method**, so a reader cannot repair, migrate
//! or tidy the directory it is reading — the call does not exist. Writing back
//! into a directory Armada does not own is a different capability and arrives
//! as a different type; `docs/concepts/kit.md` holds the decision.

use alloc::string::String;
use alloc::vec::Vec;

use crate::ci::{FileEntry, FileRead};

/// A person's home directory, read and never written.
///
/// **Rooted at the home rather than at the harness's own folder**, because a
/// harness keeps some configuration beside that folder rather than inside it.
/// Paths are relative and `/`-separated, as
/// [`RepositoryFiles`](crate::RepositoryFiles) already is for a checkout —
/// which this is deliberately not, so that one cannot be passed for the other.
pub trait SetupFiles {
    fn read(&self, path: &str) -> FileRead;
    /// The entries of `dir`, or why it would not list. **A missing directory
    /// is an error here**, and the caller reads it as nothing of that kind.
    fn entries(&self, dir: &str) -> Result<Vec<FileEntry>, String>;
}

/// The kinds `docs/concepts/kit.md` names, in the order a person reads them.
///
/// **Every kind is answered, including the ones nothing reads yet.** A kind
/// left out is drawn as empty, and an empty Kit beside a full home directory is
/// the report this seam exists to stop being given.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SetupKind {
    Skills,
    Plugins,
    /// The global "how I work" file. Kit-only: a Manifest has no counterpart.
    AgentFile,
    /// How a harness is told to answer: the person's, the repository's and a
    /// plugin's, and which one is in force. Read only.
    OutputStyles,
    SubAgents,
    Commands,
    /// Servers the person connected **outside Armada**. Seen, never handed on.
    McpServers,
    /// What a Drone may run without asking.
    Allowlist,
    /// The models a person allows.
    Models,
}

impl SetupKind {
    pub const ALL: &'static [SetupKind] = &[
        SetupKind::Skills,
        SetupKind::Plugins,
        SetupKind::AgentFile,
        SetupKind::OutputStyles,
        SetupKind::SubAgents,
        SetupKind::Commands,
        SetupKind::McpServers,
        SetupKind::Allowlist,
        SetupKind::Models,
    ];

    /// The word the wire spells this kind as — Armada's vocabulary and never a
    /// harness's, so two harnesses answer the same kind under the same word and
    /// one surface draws either.
    pub fn as_wire(&self) -> &'static str {
        match self {
            SetupKind::Skills => "skills",
            SetupKind::Plugins => "plugins",
            SetupKind::AgentFile => "agent_file",
            SetupKind::OutputStyles => "output_styles",
            SetupKind::SubAgents => "sub_agents",
            SetupKind::Commands => "commands",
            SetupKind::McpServers => "mcp_servers",
            SetupKind::Allowlist => "allowlist",
            SetupKind::Models => "models",
        }
    }
}

/// One thing a person has, with enough to name it and say where it came from —
/// and deliberately not enough to run it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SetupItem {
    /// What it is called, as the harness names it.
    pub name: String,
    /// Its own words for what it is, where the file carries them, or what it
    /// is at where it is a server. **Never a whole command and never a whole
    /// URL**, and never more than a sentence or two.
    pub says: Option<String>,
    /// Where it came from, as a person would type it. **The only path here**,
    /// and it names a file rather than carrying one.
    pub source: String,
    /// What only an output style has. `None` on every other kind.
    pub style: Option<StyleFacts>,
    /// The plugin that brought it, by name without its marketplace. `None` for
    /// the person's own and on every kind a plugin does not bring.
    pub plugin: Option<String>,
    /// The command that takes it off this machine, as the harness spells it. On an installed
    /// plugin alone.
    pub uninstall: Option<String>,
}

/// Where an output style came from, as Armada spells it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StyleFrom {
    /// The person's own, under their home.
    Person,
    /// The repository's, under its own copy of the harness folder.
    Repository,
    /// Shipped by an installed plugin.
    Plugin,
    /// One the harness carries and no file holds.
    BuiltIn,
    /// Armada's, which no harness holds.
    Armada,
}

impl StyleFrom {
    pub fn as_wire(&self) -> &'static str {
        match self {
            StyleFrom::Person => "person",
            StyleFrom::Repository => "repository",
            StyleFrom::Plugin => "plugin",
            StyleFrom::BuiltIn => "built_in",
            StyleFrom::Armada => "armada",
        }
    }
}

/// Whether an output style is in force.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StyleApplied {
    /// Chosen in the settings files, local over project over user.
    Active,
    /// Present and not chosen.
    Available,
    /// On top of whichever one is active, whatever is chosen.
    Always,
}

impl StyleApplied {
    pub fn as_wire(&self) -> &'static str {
        match self {
            StyleApplied::Active => "active",
            StyleApplied::Available => "available",
            StyleApplied::Always => "always",
        }
    }
}

/// An output style's own facts. **Never its body**, which is the person's
/// writing, or Armada's rules, and a row is not where either is read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StyleFacts {
    pub from: StyleFrom,
    pub applied: StyleApplied,
    /// The plugin's own name, on a style a plugin ships and nowhere else.
    pub plugin: Option<String>,
}

/// Something of the right shape in the right place that would not read.
///
/// **Named rather than skipped.** A skill whose front matter will not parse is
/// one the person has and Armada cannot describe; leaving it out would quietly
/// understate how much they have.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Unreadable {
    pub source: String,
    pub why: String,
}

/// What became of one kind.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WhatWasRead {
    /// Read. **Both may be empty**, which is a person who has none of this
    /// kind rather than a gap in the reading.
    Read {
        items: Vec<SetupItem>,
        unreadable: Vec<Unreadable>,
    },
    /// Not read at all, with why. Drawn as not built rather than as empty.
    NotRead { why: String },
}

/// One kind, and what became of it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KindRead {
    pub kind: SetupKind,
    pub what: WhatWasRead,
}

/// Everything one read made of a person's setup.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Inventory {
    /// The harness, in its own name. **The one vendor word that crosses**, as
    /// data an adapter produced rather than a literal a surface holds — which
    /// is what lets a second harness draw on the same screen.
    pub harness: String,
    /// Where it was read, as a person would type it.
    pub home: String,
    /// Whether that home is there at all. `false` is a person who has not run
    /// this harness here, and every kind below reads as nothing.
    pub present: bool,
    /// One entry per [`SetupKind::ALL`], in that order and complete.
    pub kinds: Vec<KindRead>,
}

impl Inventory {
    /// What was read for one kind, or `None` where the answer left it out —
    /// which an implementation is not supposed to do.
    pub fn kind(&self, kind: SetupKind) -> Option<&WhatWasRead> {
        self.kinds
            .iter()
            .find(|read| read.kind == kind)
            .map(|read| &read.what)
    }
}

/// Reads the setup a person already has, for one harness.
///
/// **A second harness is a second implementation and moves nothing above it.**
/// What a harness calls a skill, where it keeps its plugins and which file
/// holds the servers a person connected are its to know.
///
/// **No failure.** A home that is not there, a directory that will not list and
/// a file that will not parse are all answers this seam carries: a `Result`
/// would let one bad file take the whole reading down, and what a person would
/// see for it is the empty Kit again.
pub trait HarnessSetup {
    fn read(&self) -> Inventory;

    /// The same read, and the output styles and settings of each repository
    /// root too. **A default that ignores them**, so a harness with no
    /// per-repository setup needs nothing here.
    fn read_across(&self, repositories: &[String]) -> Inventory {
        let _ = repositories;
        self.read()
    }

    /// The one file a session's Loadout row names, read whole up to `asked.cap`. **A default that
    /// reads nothing**, so a harness that keeps no such files needs nothing here.
    fn read_item(&self, asked: &ItemAsked<'_>) -> ItemRead {
        let _ = asked;
        ItemRead::NotRead {
            why: String::from("This harness keeps no file for it that Armada reads."),
        }
    }
}

/// A Loadout row with a file behind it, by what it is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemKind {
    Skill,
    Subagent,
    MemoryFile,
    OutputStyle,
}

/// Whose a Loadout row is, which is where its file is looked for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemWhose<'a> {
    /// The person's harness home.
    Kit,
    /// The session's repository, at [`ItemAsked::root`].
    Repository,
    /// An installed plugin, by its own name and without its marketplace.
    Plugin(&'a str),
}

/// One row to read. **Every field but `root` and `cap` is as the query spelled it**, and nothing
/// here has been found safe yet: that is the reader's to refuse.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ItemAsked<'a> {
    pub kind: ItemKind,
    /// The name without a plugin's `<plugin>:`; a memory file's is the path the mod reported it at.
    pub name: &'a str,
    pub whose: ItemWhose<'a>,
    /// The session's repository root, absolute.
    pub root: &'a str,
    /// The most bytes of the body to carry.
    pub cap: usize,
}

/// What one row's file said.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ItemRead {
    Read {
        /// Where, for a person: from the repository's root, `~/…` under the home, or from the
        /// plugin's own folder.
        path: String,
        markdown: bool,
        body: String,
        truncated: bool,
    },
    /// A sentence a person reads.
    NotRead { why: String },
}
