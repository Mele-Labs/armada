//! A Workspace means a set of repositories here, read off a piece file and kept in the places a
//! piece is. **Nothing here touches the Manifest's workspace**, the monorepo unit
//! `manifest/workspace.rs` reads; the two share a word and no code.
//!
//! One file is one Workspace: `workspace_id` and `name` say which, and `repositories` lists the
//! Manifest ids it holds. It is read as a lifecycle's file is, refusing every fault of the file with
//! its key, then what a whole Workspace must be ([`core_model::Workspace::declared`]).
//!
//! **A Workspace is yours or the repository's, merged by id with yours winning.** One that spans
//! several repositories and is committed is kept in one repository the person picks, and the others
//! read it from there, so [`Workspaces::of`] is handed the files of every repository Fleet serves
//! and not only the one asked about. Two repositories committing one id is a fault in the
//! repository's place, as two files are in a workflow's, and both are set aside.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use core_model::{
    MalformedWorkspace, ManifestId, PieceSource, ScopeId, Title, Ulid, Workspace,
};
use serde_yaml_ng::Value;

use crate::error::{Fault, LoadError, Refusal};
use crate::piece::{hides, named, LeftOut, Written};
use crate::yaml::{self, Table};

const TOP_LEVEL: &[&str] = &["version", "workspace_id", "name", "repositories"];

/// The one version of the file there is.
const VERSIONS: &[&str] = &["1"];

/// Why a Workspace file did not load.
#[derive(Debug)]
pub enum WorkspaceError {
    /// It is not YAML, or the file holds a fault of its own.
    Load(LoadError),
    /// Every key is right, and the Workspace they make is not.
    Malformed { path: PathBuf, why: MalformedWorkspace },
}

impl std::fmt::Display for WorkspaceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WorkspaceError::Load(load) => load.fmt(f),
            WorkspaceError::Malformed { path, why } => write!(f, "{} {why}", path.display()),
        }
    }
}

impl std::error::Error for WorkspaceError {}

/// Read a Workspace from the text of its file. `path` is only named in a refusal.
pub fn parse_workspace(path: &Path, text: &str) -> Result<Workspace, WorkspaceError> {
    let root: Value = serde_yaml_ng::from_str(text)
        .map_err(|cause| WorkspaceError::Load(LoadError::NotYaml { path: path.to_path_buf(), cause }))?;
    let mut out = Vec::new();
    let Some((id, name, repositories)) = read(&root, &mut out).filter(|_| out.is_empty()) else {
        return Err(WorkspaceError::Load(LoadError::Refused { path: path.to_path_buf(), refusals: out }));
    };
    Workspace::declared(id, name, repositories)
        .map_err(|why| WorkspaceError::Malformed { path: path.to_path_buf(), why })
}

fn read(root: &Value, out: &mut Vec<Refusal>) -> Option<(ScopeId, Title, Vec<ManifestId>)> {
    let mut top = Table::open("", root, out)?;
    if let Some(version) = top.required("version", out).and_then(|value| yaml::positive("version", value, out)) {
        if version != 1 {
            out.push(Refusal::new("version", Fault::NotYetCarried { value: version.to_string(), carried: VERSIONS }));
        }
    }
    let id = top
        .required("workspace_id", out)
        .and_then(|value| yaml::text("workspace_id", value, out))
        .and_then(|written| ScopeId::new(&written).ok());
    let name = top
        .required("name", out)
        .and_then(|value| yaml::text("name", value, out))
        .and_then(|written| Title::new(&written).ok());
    let repositories: Option<Vec<ManifestId>> = top
        .required("repositories", out)
        .and_then(|value| yaml::list("repositories", value, out))
        .map(|items| {
            items
                .iter()
                .filter_map(|(at, item)| yaml::text(at, item, out))
                .map(|written| ManifestId::carried(Ulid::carried(written.trim())))
                .collect()
        });
    top.close(TOP_LEVEL, out);
    Some((id?, name?, repositories?))
}

/// One Workspace file that parsed, and what became of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceFile {
    workspace: Workspace,
    source: PieceSource,
    path: PathBuf,
    overridden_by: Option<PieceSource>,
    hides: Option<PieceSource>,
}

impl WorkspaceFile {
    pub fn workspace(&self) -> &Workspace {
        &self.workspace
    }

    pub fn source(&self) -> PieceSource {
        self.source
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The place whose file of this id runs instead of this one. Absent on the one that runs.
    pub fn overridden_by(&self) -> Option<PieceSource> {
        self.overridden_by
    }

    /// The place whose file this one replaced because it says something else. **Two copies that are
    /// the same hide nothing**, so they carry no mark.
    pub fn hides(&self) -> Option<PieceSource> {
        self.hides
    }
}

/// Every Workspace in hand, merged by id, and the files set aside.
#[derive(Debug, Default)]
pub struct Workspaces {
    files: Vec<WorkspaceFile>,
    left_out: Vec<LeftOut>,
}

impl Workspaces {
    /// Parse every file and merge by id, the person's own winning over the repository's.
    ///
    /// **The order the files arrive in decides nothing.** A file that does not fit is left out with
    /// its reason, and two in one place with one id are both left out, naming both.
    pub fn of(written: impl IntoIterator<Item = Written>) -> Workspaces {
        let mut left_out = Vec::new();
        let mut placed: BTreeMap<(ScopeId, PieceSource), Vec<(Workspace, PathBuf)>> = BTreeMap::new();
        for one in written {
            match parse_workspace(one.path(), one.text()) {
                Ok(workspace) => placed
                    .entry((workspace.id().clone(), one.source()))
                    .or_default()
                    .push((workspace, one.path().to_path_buf())),
                Err(why) => left_out.push(LeftOut::new(one.source(), one.path().to_path_buf(), why.to_string())),
            }
        }
        let mut held: BTreeMap<ScopeId, Vec<(Workspace, PieceSource, PathBuf)>> = BTreeMap::new();
        for ((id, source), mut defs) in placed {
            if defs.len() > 1 {
                let paths: Vec<&Path> = defs.iter().map(|(_, path)| path.as_path()).collect();
                let said = format!("`{}` is written twice in one place, in {}", id.as_str(), named(&paths));
                let first = defs.swap_remove(0).1;
                left_out.push(LeftOut::new(source, first, said));
                continue;
            }
            let (workspace, path) = defs.remove(0);
            held.entry(id).or_default().push((workspace, source, path));
        }
        let mut files = Vec::new();
        for (_, mut candidates) in held {
            candidates.sort_by(|a, b| b.1.cmp(&a.1));
            let mut candidates = candidates.into_iter();
            let Some((running, by, path)) = candidates.next() else { continue };
            let mut hid = None;
            let mut beaten = Vec::new();
            for (workspace, source, path) in candidates {
                if hides(by, &running, source, &workspace) {
                    hid = Some(source);
                }
                beaten.push(WorkspaceFile { workspace, source, path, overridden_by: Some(by), hides: None });
            }
            files.push(WorkspaceFile { workspace: running, source: by, path, overridden_by: None, hides: hid });
            files.extend(beaten);
        }
        Workspaces { files, left_out }
    }

    /// Every file that parsed: the one that runs for each id and each one it replaced.
    pub fn files(&self) -> &[WorkspaceFile] {
        &self.files
    }

    /// The files set aside, and why.
    pub fn left_out(&self) -> &[LeftOut] {
        &self.left_out
    }

    /// One Workspace for each id: the one that runs.
    pub fn running(&self) -> impl Iterator<Item = &Workspace> {
        self.files.iter().filter(|file| file.overridden_by.is_none()).map(|file| &file.workspace)
    }

    /// Where the Workspace `id` that runs is kept, if it is committed to a repository. **That
    /// repository is its home**: the others that belong to it read what is scoped to it from there.
    /// Absent for one that is Kit's, which has none.
    pub fn kept_at(&self, id: &ScopeId) -> Option<&Path> {
        self.files
            .iter()
            .find(|file| file.overridden_by.is_none() && file.workspace.id() == id)
            .filter(|file| file.source == PieceSource::Repository)
            .map(|file| file.path.as_path())
    }

    /// The Workspaces that hold `repository`.
    pub fn holding(&self, repository: &ManifestId) -> Vec<&Workspace> {
        self.running().filter(|workspace| workspace.holds(repository)).collect()
    }
}
