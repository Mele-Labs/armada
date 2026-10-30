//! The autonomy defaults a Workspace or a repository sets, read off a piece file: for a kind of act,
//! Acts, Asks or Off. **A piece of its own kind**, kept in the places every piece is, so where it
//! lives is the owner's to choose and never a place of its own.
//!
//! One file is one layer: `autonomy_id` says which, `scope` says where it applies (a Workspace's id
//! or a repository's; absent is every repository), and `defaults` names the kinds it sets.
//!
//! ```yaml
//! version: 1
//! autonomy_id: my_store
//! scope: my-store
//! defaults:
//!   tracker_write: acts
//!   forge_write: asks
//! ```
//!
//! **Layers are laid over each other, and are not chosen between.** A rule's own `autonomy` wins
//! over all of them, a repository's file over its Workspace's, and a file for every repository
//! under both ([`Autonomies::defaults`]). Two files with one id are chosen between as every piece
//! is, by [`crate::resolving`]. **Text in, not directories**, so a test drives it with no file.

use std::path::{Path, PathBuf};

use core_model::rule::{ActKind, AutonomyDefaults, AutonomyId};
use core_model::{LifecycleScope, ManifestId, PieceSource, ScopeId};
use serde_yaml_ng::Value;

use crate::error::{Fault, LoadError, Refusal};
use crate::lifecycle::id_of;
use crate::piece::{LeftOut, Written};
use crate::resolving::{resolve, Rank, Resolved, Scoped};
use crate::rule::acts::KINDS;
use crate::rule::{AUTONOMY, AUTONOMY_LEGAL};
use crate::workspace::Workspaces;
use crate::yaml::{self, Table};

const TOP_LEVEL: &[&str] = &["version", "autonomy_id", "scope", "defaults"];

/// The one version of the file there is.
const VERSIONS: &[&str] = &["1"];

/// One file of defaults, read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AutonomyPiece {
    id: AutonomyId,
    scope: LifecycleScope,
    defaults: AutonomyDefaults,
}

impl AutonomyPiece {
    pub fn id(&self) -> &AutonomyId {
        &self.id
    }

    pub fn scope(&self) -> &LifecycleScope {
        &self.scope
    }

    pub fn defaults(&self) -> &AutonomyDefaults {
        &self.defaults
    }
}

/// Read a file of defaults from its text. `path` is only named in a refusal.
pub fn parse(path: &Path, text: &str) -> Result<AutonomyPiece, LoadError> {
    let root: Value = serde_yaml_ng::from_str(text)
        .map_err(|cause| LoadError::NotYaml { path: path.to_path_buf(), cause })?;
    let mut out = Vec::new();
    let read = read(&root, &mut out);
    match read.filter(|_| out.is_empty()) {
        Some(piece) => Ok(piece),
        None => Err(LoadError::Refused { path: path.to_path_buf(), refusals: out }),
    }
}

fn read(root: &Value, out: &mut Vec<Refusal>) -> Option<AutonomyPiece> {
    let mut top = Table::open("", root, out)?;
    if let Some(version) = top.required("version", out).and_then(|value| yaml::positive("version", value, out)) {
        if version != 1 {
            out.push(Refusal::new("version", Fault::NotYetCarried { value: version.to_string(), carried: VERSIONS }));
        }
    }
    let id = top.required("autonomy_id", out).and_then(|value| id_of("autonomy_id", value, AutonomyId::new, out));
    let scope = match top.optional("scope") {
        None => Some(LifecycleScope::Everywhere),
        Some(value) => yaml::text("scope", value, out)
            .and_then(|written| ScopeId::new(&written).ok())
            .map(LifecycleScope::Within),
    };
    let defaults = top.required("defaults", out).and_then(|value| defaults_of("defaults", value, out));
    top.close(TOP_LEVEL, out);
    Some(AutonomyPiece { id: id?, scope: scope?, defaults: defaults? })
}

/// The kinds a file sets. **Each one before any is given up on**, so a file with two faults is
/// refused for two. A file that sets none is refused, since a layer that says nothing is a file
/// someone expected to do something.
fn defaults_of(at: &str, value: &Value, out: &mut Vec<Refusal>) -> Option<AutonomyDefaults> {
    let table = Table::open(at, value, out)?;
    if table.is_empty() {
        out.push(Refusal::new(at, Fault::Empty));
        return None;
    }
    let mut set = Some(AutonomyDefaults::new());
    for (name, value) in table.into_entries() {
        let key = format!("{at}.{name}");
        let kind = ActKind::from_wire(&name);
        if kind.is_none() {
            out.push(Refusal::new(key.as_str(), Fault::NotInTheSchema { value: name, legal: KINDS }));
        }
        let autonomy = yaml::word(&key, value, AUTONOMY, AUTONOMY_LEGAL, AUTONOMY_LEGAL, out);
        set = match (set, kind, autonomy) {
            (Some(held), Some(kind), Some(autonomy)) => Some(held.setting(kind, autonomy)),
            _ => None,
        };
    }
    set
}

impl Scoped for AutonomyPiece {
    type Id = AutonomyId;

    fn id(&self) -> &AutonomyId {
        &self.id
    }

    fn id_text(&self) -> &str {
        self.id.as_str()
    }

    fn scope(&self) -> &LifecycleScope {
        &self.scope
    }
}

/// One file of defaults that applies, and what became of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutonomyFile {
    piece: AutonomyPiece,
    rank: Rank,
    path: PathBuf,
    overridden_by: Option<Rank>,
    hides: Option<PieceSource>,
}

impl AutonomyFile {
    pub fn piece(&self) -> &AutonomyPiece {
        &self.piece
    }

    pub fn rank(&self) -> Rank {
        self.rank
    }

    /// Where it was read.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The rank of the one that applies instead of this. Absent on the one that applies.
    pub fn overridden_by(&self) -> Option<Rank> {
        self.overridden_by
    }

    /// The place whose file this one replaced because it says something else.
    pub fn hides(&self) -> Option<PieceSource> {
        self.hides
    }
}

impl From<Resolved<AutonomyPiece>> for AutonomyFile {
    fn from(one: Resolved<AutonomyPiece>) -> AutonomyFile {
        // Armada carries no defaults of its own, so a layer that applies was always read.
        AutonomyFile {
            piece: one.piece,
            rank: one.rank,
            path: one.path.unwrap_or_default(),
            overridden_by: one.overridden_by,
            hides: one.hides,
        }
    }
}

/// The files of defaults that apply to one repository, and the files set aside.
#[derive(Debug, Default)]
pub struct Autonomies {
    files: Vec<AutonomyFile>,
    left_out: Vec<LeftOut>,
}

impl Autonomies {
    /// What applies to `repository`: every file in `written` whose scope names it.
    pub fn applying_to(repository: &ManifestId, workspaces: &Workspaces, written: impl IntoIterator<Item = Written>) -> Autonomies {
        let read = |path: &Path, text: &str| parse(path, text).map_err(|why| why.to_string());
        let (resolved, left_out) = resolve(repository, workspaces, Vec::new(), written, read);
        Autonomies { files: resolved.into_iter().map(AutonomyFile::from).collect(), left_out }
    }

    /// Every file that applies: the one that applies for each id and each one it replaced.
    pub fn files(&self) -> &[AutonomyFile] {
        &self.files
    }

    /// The files set aside, and why.
    pub fn left_out(&self) -> &[LeftOut] {
        &self.left_out
    }

    /// What the files that apply set, the repository's laid over its Workspace's and both over a
    /// file for every repository. **Where two files at one scope set one kind, the later rank
    /// wins** (yours over the repository's), and where the rank is the same too, the later id.
    pub fn defaults(&self) -> AutonomyDefaults {
        let mut applying: Vec<&AutonomyFile> = self.files.iter().filter(|file| file.overridden_by.is_none()).collect();
        applying.sort_by(|a, b| (a.rank, a.piece.id()).cmp(&(b.rank, b.piece.id())));
        applying
            .into_iter()
            .fold(AutonomyDefaults::new(), |wider, file| file.piece.defaults().laid_over(&wider))
    }
}
