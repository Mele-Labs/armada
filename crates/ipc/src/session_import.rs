//! Bringing in the folders a person already works in with another agent CLI —
//! `list_import_folders`, `get_import_runs` and `install_agent_mod` in
//! `crates/ipc/operations/`.
//!
//! **The agent's name is data, never a literal here.** [`RunFact::source`] is a tag the adapter that
//! read the facts supplies, so a second CLI draws on the same screen and the live-Job reader that
//! produces the same facts says `jobs`.

use serde::{Deserialize, Serialize};

use crate::Instant;

/// Whether a folder is a git repository, from the folder alone: nothing runs `git`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum ImportGit {
    /// The folder is a repository's root.
    Root,
    /// Under a repository's root, which is the folder to serve.
    Inside { root: String },
    NotARepository,
    /// The folder is gone, so there is nothing to look in.
    Unknown,
}

/// One folder an agent CLI has kept sessions for.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportFolder {
    /// Absolute, as the sessions' own records name it.
    pub path: String,
    /// Whether it is still there.
    pub exists: bool,
    /// When the newest session's record was last written.
    pub last_session_at: Instant,
    /// How many sessions it kept.
    pub sessions: u32,
    pub git: ImportGit,
}

/// `list_import_folders`' answer, newest session first.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportFolders {
    /// The CLI the folders were read from, in its own name.
    pub agent: String,
    pub folders: Vec<ImportFolder>,
}

/// What a command was run for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunFactKind {
    Tests,
    Lint,
    Build,
    DevServer,
    Typecheck,
    Format,
}

/// One thing a repository was seen to run, in the shape the live-Job reader produces too.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunFact {
    /// Relative to the repository root, `/`-separated; `.` is the root.
    pub workspace: String,
    pub kind: RunFactKind,
    pub command: String,
    /// Where a dev server was seen listening.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
    /// Whether the latest run of it worked.
    pub succeeded: bool,
    pub last_ran: Instant,
    /// Who saw it run: a tag the reader supplies.
    pub source: String,
}

/// `get_import_runs`' answer for one folder.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportRuns {
    pub path: String,
    /// What the workspaces are relative to: the repository's root when the folder is inside one.
    pub root: String,
    pub facts: Vec<RunFact>,
}

/// `install_agent_mod`'s body. `requested` must be `true`: the install runs only for a person who
/// pressed it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstallAgentMod {
    pub requested: bool,
}

/// Which step of the install stopped.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModInstallStep {
    /// Copying the mod into the folder the CLI is pointed at.
    Populate,
    /// Telling the CLI where the mod folder is.
    AddMarketplace,
    Install,
}

/// How the install went. **Tagged**, so a screen shows the CLI's own words either way.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "result", rename_all = "snake_case")]
pub enum ModInstalled {
    Installed { said: String },
    Failed { step: ModInstallStep, said: String },
}
