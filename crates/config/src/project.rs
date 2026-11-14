//! A Project is a set of members, whole repositories or any folder of one, read off a file Fleet keeps
//! in the place the person chose for it. **Nothing here touches the Manifest's workspace**, the
//! monorepo unit `manifest/workspace.rs` reads; a member names a folder, which needs no `armada.yml`.
//!
//! One file is one Project, version 2: `project_id` and `name` say which, `members` lists each as
//! `repository` (a Manifest id) and, for a folder of it, `folder`. An optional `values:` block holds the
//! values the Project sets (`crate::values`), only the keys it sets. `shared_with_macs: true` lets its
//! Jobs be seen from his other Macs, and is written only when on. It is read as a lifecycle's file
//! is, refusing every fault of the file with its key, then what a whole Project must be
//! ([`core_model::Project::declared`]).
//!
//! **A Project is never merged and never in a repository.** Each file is one Project that runs, kept
//! on this Mac, in a synced folder or in the cloud, and two files of one id are both left out, since
//! nothing says which the person meant. [`parse_workspace_v1`] reads the file a Workspace was kept
//! in before, for the migration only.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use core_model::{
    Folder, MalformedFolder, MalformedProject, MalformedStore, ManifestId, Member, Project, ProjectStore, ScopeId, StoreAddress, StoreTeam,
    Timestamp, Title, Ulid, Values, WorkerName,
};
use serde_yaml_ng::Value;

use crate::error::{Fault, LoadError, Refusal};
use crate::piece::named;
use crate::yaml::{self, Table};

const TOP_LEVEL: &[&str] = &["version", "project_id", "name", "members", "values", "shared_with_macs", "store"];
const STORE: &[&str] = &["address", "team", "worker", "deployed", "joins_unlimited"];
const MEMBER: &[&str] = &["repository", "folder"];
const TOP_LEVEL_V1: &[&str] = &["version", "workspace_id", "name", "repositories"];

/// The one version of the file there is.
const VERSIONS: &[&str] = &["2"];
const VERSIONS_V1: &[&str] = &["1"];

/// Where a Project is kept. **Never a repository.**
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ProjectPlace {
    ThisMac,
    Synced,
    Cloud,
}

/// Why a Project file did not load.
#[derive(Debug)]
pub enum ProjectError {
    /// It is not YAML, or the file holds a fault of its own.
    Load(LoadError),
    /// A member's folder is not a path inside its repository.
    Folder { path: PathBuf, at: String, why: MalformedFolder },
    /// Every key is right, and the Project they make is not.
    Malformed { path: PathBuf, why: MalformedProject },
}

impl std::fmt::Display for ProjectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProjectError::Load(load) => load.fmt(f),
            ProjectError::Folder { path, at, why } => write!(f, "{} {at} {why}", path.display()),
            ProjectError::Malformed { path, why } => write!(f, "{} {why}", path.display()),
        }
    }
}

impl std::error::Error for ProjectError {}

/// Read a Project from the text of its file. `path` is only named in a refusal.
pub fn parse_project(path: &Path, text: &str) -> Result<Project, ProjectError> {
    parse(path, text, Version::Two)
}

/// Read the file a Workspace was kept in before Projects: `workspace_id`, `name` and `repositories`,
/// each repository a whole member. **For the migration only**, which writes version 2 in its place.
pub fn parse_workspace_v1(path: &Path, text: &str) -> Result<Project, ProjectError> {
    parse(path, text, Version::One)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Version {
    One,
    Two,
}

fn parse(path: &Path, text: &str, version: Version) -> Result<Project, ProjectError> {
    let root: Value = serde_yaml_ng::from_str(text)
        .map_err(|cause| ProjectError::Load(LoadError::NotYaml { path: path.to_path_buf(), cause }))?;
    let mut out = Vec::new();
    let mut bad_folder = None;
    let read = read(&root, version, &mut out, &mut bad_folder);
    if let Some((at, why)) = bad_folder.filter(|_| out.is_empty()) {
        return Err(ProjectError::Folder { path: path.to_path_buf(), at, why });
    }
    let Some((id, name, members, values, shared, store)) = read.filter(|_| out.is_empty()) else {
        return Err(ProjectError::Load(LoadError::Refused { path: path.to_path_buf(), refusals: out }));
    };
    Project::declared(id, name, members)
        .map(|project| project.with_values(values).with_shared_with_macs(shared).with_store(store))
        .map_err(|why| ProjectError::Malformed { path: path.to_path_buf(), why })
}

fn read(
    root: &Value,
    version: Version,
    out: &mut Vec<Refusal>,
    bad_folder: &mut Option<(String, MalformedFolder)>,
) -> Option<(ScopeId, Title, Vec<Member>, Values, bool, Option<ProjectStore>)> {
    let (id_key, members_key, known, carried, wanted) = match version {
        Version::One => ("workspace_id", "repositories", TOP_LEVEL_V1, VERSIONS_V1, 1),
        Version::Two => ("project_id", "members", TOP_LEVEL, VERSIONS, 2),
    };
    let mut top = Table::open("", root, out)?;
    if let Some(version) = top.required("version", out).and_then(|value| yaml::positive("version", value, out)) {
        if version != wanted {
            out.push(Refusal::new("version", Fault::NotYetCarried { value: version.to_string(), carried }));
        }
    }
    let id = top
        .required(id_key, out)
        .and_then(|value| yaml::text(id_key, value, out))
        .and_then(|written| ScopeId::new(&written).ok());
    let name = top
        .required("name", out)
        .and_then(|value| yaml::text("name", value, out))
        .and_then(|written| Title::new(&written).ok());
    let members = top.required(members_key, out).and_then(|value| match (version, value) {
        // A Project made empty is written with `members: []`.
        (Version::Two, Value::Sequence(items)) if items.is_empty() => Some(Vec::new()),
        (Version::One, Value::Sequence(items)) if items.is_empty() => Some(Vec::new()),
        (Version::One, value) => yaml::list(members_key, value, out).map(|items| {
            items
                .iter()
                .filter_map(|(at, item)| yaml::text(at, item, out))
                .map(|written| Member::whole(ManifestId::carried(Ulid::carried(written.trim()))))
                .collect()
        }),
        (Version::Two, value) => yaml::list(members_key, value, out).map(|items| {
            items.iter().filter_map(|(at, item)| member(at, item, out, bad_folder)).collect()
        }),
    });
    // Only a version 2 file has `values`, and the block holds only the keys it sets.
    let values_at = top.at("values");
    let values = match version {
        Version::Two => top.optional("values").map(|value| crate::values::read_block(&values_at, value, true, out)).unwrap_or_default(),
        Version::One => Values::default(),
    };
    // Off where it is not written, and in a version 1 file, which has no such key.
    let shared = match version {
        Version::Two => top.optional("shared_with_macs").and_then(|value| yaml::flag("shared_with_macs", value, out)).unwrap_or(false),
        Version::One => false,
    };
    // None where it is not written, which is a project that sends nothing.
    let store_at = top.at("store");
    let store = match version {
        Version::Two => top.optional("store").and_then(|value| store(&store_at, value, out)),
        Version::One => None,
    };
    top.close(known, out);
    Some((id?, name?, members?, values, shared, store))
}

fn member(at: &str, item: &Value, out: &mut Vec<Refusal>, bad_folder: &mut Option<(String, MalformedFolder)>) -> Option<Member> {
    let mut table = Table::open(at, item, out)?;
    let repository = table
        .required("repository", out)
        .and_then(|value| yaml::text(&format!("{at}.repository"), value, out))
        .map(|written| ManifestId::carried(Ulid::carried(written.trim())));
    let folder_at = format!("{at}.folder");
    let folder = table.optional("folder").and_then(|value| yaml::text(&folder_at, value, out));
    table.close(MEMBER, out);
    let repository = repository?;
    match folder {
        None => Some(Member::whole(repository)),
        Some(written) => match Folder::new(&written) {
            Ok(folder) => Some(Member::folder(repository, folder)),
            Err(why) => {
                bad_folder.get_or_insert((folder_at, why));
                None
            }
        },
    }
}

/// A `store:` block: `address`, and `team`, `worker` and `deployed` where they are written.
fn store(at: &str, value: &Value, out: &mut Vec<Refusal>) -> Option<ProjectStore> {
    let mut table = Table::open(at, value, out)?;
    let mut read = |name: &str, required: bool, out: &mut Vec<Refusal>| {
        let key = format!("{at}.{name}");
        let value = if required { table.required(name, out) } else { table.optional(name) };
        value.and_then(|value| yaml::text(&key, value, out)).map(|written| (key, written))
    };
    let (address, team, worker, deployed) =
        (read("address", true, out), read("team", false, out), read("worker", false, out), read("deployed", false, out));
    let unlimited_at = format!("{at}.joins_unlimited");
    let joins_unlimited = table.optional("joins_unlimited").and_then(|value| yaml::flag(&unlimited_at, value, out)).unwrap_or(false);
    table.close(STORE, out);
    let address = store_part(address, StoreAddress::new, out);
    let team = store_part(team, StoreTeam::new, out);
    let worker = store_part(worker, WorkerName::new, out);
    let deployed = store_part(deployed, |written| {
        let at = Timestamp::from_rfc3339(written);
        at.epoch_millis().map(|_| at).ok_or_else(|| MalformedStore::Deployed(written.to_string()))
    }, out);
    Some(ProjectStore { address: address.ok()??, team: team.ok()?, worker: worker.ok()?, deployed: deployed.ok()?, joins_unlimited })
}

/// One part of `store:` made from its text, or `Err` with the fault pushed where it is not one.
fn store_part<T>(
    read: Option<(String, String)>,
    make: impl Fn(&str) -> Result<T, MalformedStore>,
    out: &mut Vec<Refusal>,
) -> Result<Option<T>, ()> {
    let Some((key, written)) = read else { return Ok(None) };
    make(&written).map(Some).map_err(|why| out.push(Refusal::new(key, Fault::NotAStore { why })))
}

/// `project` as the text of its version 2 file. A Project with no members is written `members: []`.
pub fn project_text(project: &Project) -> Result<String, String> {
    use serde_yaml_ng::{Mapping, Value};
    let text = |written: &str| Value::String(written.to_string());
    let members = project
        .members()
        .iter()
        .map(|member| {
            let mut one = Mapping::new();
            one.insert(text("repository"), text(member.repository().as_str()));
            if let Some(folder) = member.folder_of() {
                one.insert(text("folder"), text(folder.as_str()));
            }
            Value::Mapping(one)
        })
        .collect();
    let mut top = Mapping::new();
    top.insert(text("version"), Value::Number(2.into()));
    top.insert(text("project_id"), text(project.id().as_str()));
    top.insert(text("name"), text(project.name().as_str()));
    top.insert(text("members"), Value::Sequence(members));
    if let Some(values) = crate::values::block_value(project.values()) {
        top.insert(text("values"), values);
    }
    if project.shared_with_macs() {
        top.insert(text("shared_with_macs"), Value::Bool(true));
    }
    if let Some(store) = project.store() {
        let mut block = Mapping::new();
        block.insert(text("address"), text(store.address.as_str()));
        if let Some(team) = &store.team {
            block.insert(text("team"), text(&team.to_string()));
        }
        if let Some(worker) = &store.worker {
            block.insert(text("worker"), text(worker.as_str()));
        }
        if let Some(deployed) = &store.deployed {
            block.insert(text("deployed"), text(deployed.as_str()));
        }
        if store.joins_unlimited {
            block.insert(text("joins_unlimited"), Value::Bool(true));
        }
        top.insert(text("store"), Value::Mapping(block));
    }
    crate::piece_text(&Value::Mapping(top))
}

/// One Project file as text, the place it was read from and where.
#[derive(Debug, Clone)]
pub struct ProjectWritten {
    place: ProjectPlace,
    path: PathBuf,
    text: String,
}

impl ProjectWritten {
    /// `path` is the file, or the object a bucket holds it as.
    pub fn new(place: ProjectPlace, path: PathBuf, text: String) -> ProjectWritten {
        ProjectWritten { place, path, text }
    }
}

/// One Project file that parsed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectFile {
    project: Project,
    place: ProjectPlace,
    path: PathBuf,
}

impl ProjectFile {
    pub fn project(&self) -> &Project {
        &self.project
    }

    pub fn place(&self) -> ProjectPlace {
        self.place
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

/// A Project file that was set aside, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectLeftOut {
    place: ProjectPlace,
    path: PathBuf,
    said: String,
}

impl ProjectLeftOut {
    /// A file Fleet could not read at all, so there is no text to parse: `said` is why.
    pub fn new(place: ProjectPlace, path: PathBuf, said: String) -> ProjectLeftOut {
        ProjectLeftOut { place, path, said }
    }

    pub fn place(&self) -> ProjectPlace {
        self.place
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The sentence a person reads, naming the key or the id at fault.
    pub fn said(&self) -> &str {
        &self.said
    }
}

/// Every Project in hand and the files set aside.
#[derive(Debug, Default, Clone)]
pub struct Projects {
    files: Vec<ProjectFile>,
    left_out: Vec<ProjectLeftOut>,
}

impl Projects {
    /// Parse every file. **The order the files arrive in decides nothing**: a file that does not fit
    /// is left out with its reason, and two files of one id are both left out, naming both. `unread`
    /// are files that could not be read at all, set aside as they are.
    pub fn of(written: impl IntoIterator<Item = ProjectWritten>, unread: Vec<ProjectLeftOut>) -> Projects {
        let mut left_out = unread;
        let mut by_id: BTreeMap<ScopeId, Vec<ProjectFile>> = BTreeMap::new();
        for one in written {
            match parse_project(&one.path, &one.text) {
                Ok(project) => by_id.entry(project.id().clone()).or_default().push(ProjectFile {
                    project,
                    place: one.place,
                    path: one.path,
                }),
                Err(why) => left_out.push(ProjectLeftOut::new(one.place, one.path, why.to_string())),
            }
        }
        let mut files = Vec::new();
        for (id, mut same) in by_id {
            if same.len() > 1 {
                let paths: Vec<&Path> = same.iter().map(|file| file.path.as_path()).collect();
                let said = format!("`{}` is written twice, in {}, and neither runs", id.as_str(), named(&paths));
                left_out.extend(same.iter().map(|file| ProjectLeftOut::new(file.place, file.path.clone(), said.clone())));
                continue;
            }
            files.extend(same.pop());
        }
        Projects { files, left_out }
    }

    /// Every file that parsed, one for each id.
    pub fn files(&self) -> &[ProjectFile] {
        &self.files
    }

    /// The files set aside, and why.
    pub fn left_out(&self) -> &[ProjectLeftOut] {
        &self.left_out
    }

    /// Every Project that runs.
    pub fn running(&self) -> impl Iterator<Item = &Project> {
        self.files.iter().map(|file| &file.project)
    }

    /// The Project `id`, where its file read.
    pub fn get(&self, id: &ScopeId) -> Option<&Project> {
        self.running().find(|project| project.id() == id)
    }

    /// The file of Project `id`, where it read.
    pub fn file(&self, id: &ScopeId) -> Option<&ProjectFile> {
        self.files.iter().find(|file| file.project.id() == id)
    }

    /// The Projects that hold `repository`.
    pub fn holding(&self, repository: &ManifestId) -> Vec<&Project> {
        self.running().filter(|project| project.holds(repository)).collect()
    }
}
