//! The shared home: a private repository on the person's own forge account that every machine they
//! use reads and writes, and the folders on this machine its pieces come from and go to.
//! `docs/concepts/kit.md`, *The shared home*.
//!
//! **Writing is a capability of its own.** [`SetupFiles`](crate::SetupFiles) reads a person's
//! setup and has no write; [`KeptFolder`] is the different type that one writes through, handed
//! only to what acts on a person's press.

use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;

use crate::ci::{FileEntry, FileRead};

/// The home's repository name, under the person's own login.
pub const HOME_REPOSITORY: &str = "armada-home";

/// A folder on this machine that Armada reads and writes: the home's copy, the mods folder, Kit's
/// home, the person's home for a harness. Paths are relative and `/`-separated, and one that
/// would climb above the folder is refused.
pub trait KeptFolder: Send + Sync {
    fn read(&self, path: &str) -> FileRead;
    /// The entries of `dir`. **A missing directory is an error**, read by the caller as nothing.
    fn entries(&self, dir: &str) -> Result<Vec<FileEntry>, String>;
    /// Write `text` whole, making the folders above it.
    fn write(&self, path: &str, text: &str) -> Result<(), String>;
    /// Take one file away. One that is not there is not an error.
    fn remove(&self, path: &str) -> Result<(), String>;
}

/// Why the home did not move.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HomeFailure {
    /// The forge on this machine does not hold the profile's login. **A whole sentence**, naming the
    /// login and how to add it, which only the adapter knows.
    NotSignedIn(String),
    /// The forge or the remote did not answer: worth another try.
    Unreachable(String),
    /// It answered no, or the copy here would not take the change.
    Refused(String),
}

impl HomeFailure {
    /// A sentence for a person.
    pub fn said(&self) -> String {
        match self {
            HomeFailure::NotSignedIn(said) => said.clone(),
            HomeFailure::Unreachable(said) => alloc::format!("The shared home could not be reached: {said}"),
            HomeFailure::Refused(said) => alloc::format!("The shared home refused: {said}"),
        }
    }
}

/// Every profile's home, one a login: a machine can hold a personal profile and a work one, and
/// each has its own `<login>/armada-home`.
pub trait HomeRepositories: Send + Sync {
    /// `login`'s home, acting as `login` alone. **Asks the forge nothing**: whether it holds the
    /// login is found by the first act. A login that cannot be one is refused.
    fn of(&self, login: &str) -> Result<Arc<dyn HomeRepository>, HomeFailure>;
}

/// One login's home, and this machine's copy of it. **It acts as that login and no other**: there
/// is no call that names another.
///
/// **Nothing here pushes anything but named paths**: [`HomeRepository::publish`] is the one way
/// a change leaves the machine, and it takes the paths it commits.
pub trait HomeRepository: Send + Sync {
    /// `<login>/<repository>` where this machine holds a copy, `None` where it does not yet.
    fn here(&self) -> Option<String>;
    /// Make the home where it does not exist, private, then clone it here, or bring the copy here
    /// up to date where there is one.
    fn open(&self) -> Result<(), HomeFailure>;
    /// Bring the copy here up to date.
    fn pull(&self) -> Result<(), HomeFailure>;
    /// The copy here, read and written. A write stays on this machine until it is published.
    fn files(&self) -> &dyn KeptFolder;
    /// Commit exactly `paths`, written or removed, and push them.
    fn publish(&self, paths: &[String], message: &str) -> Result<(), HomeFailure>;
}

/// The pieces a harness keeps under the person's home that the home carries as files.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HarnessPiece {
    Skills,
    SubAgents,
    Commands,
    /// The global "how I work" file.
    GlobalInstructions,
}

/// How a harness lays one kind out, relative to the person's home.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HarnessPlace {
    /// A folder a piece, named for it, and every file under it: a skill.
    Folders { dir: String },
    /// A file a piece, `<name><suffix>` in `dir`: a sub agent, a command.
    Files { dir: String, suffix: String },
    /// One file that is the piece: the global instructions.
    One { path: String },
}

/// An installed plugin, as the home carries it: enough to install it again, and nothing of this
/// machine's.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PluginRecord {
    /// As the harness names it, its marketplace and all.
    pub name: String,
    pub marketplace: String,
    /// Where the marketplace is added from, `owner/repository`, where it is a forge repository.
    /// `None` for one kept in a folder, which another machine cannot reach.
    pub source: Option<String>,
}

/// The harness's own folder under the person's home: where each kind is kept, a repository's
/// memory, and its plugins. **Which folder that is, and how a plugin installs, are `adapters`'.**
pub trait HarnessHome: Send + Sync {
    /// The person's home, which every place below is relative to.
    fn folder(&self) -> &dyn KeptFolder;
    fn place(&self, piece: HarnessPiece) -> HarnessPlace;
    /// The folder a repository's memory notes are kept in, by its root on this machine.
    fn memory_dir(&self, root: &str) -> String;
    /// The file in [`HarnessHome::memory_dir`] that lists the notes, which is not one itself.
    fn memory_index(&self) -> String;
    /// Every plugin installed here.
    fn plugins(&self) -> Result<Vec<PluginRecord>, String>;
    /// Install one, through the harness's own command. **A person's press only.**
    fn install(&self, plugin: &PluginRecord) -> Result<(), String>;
}
