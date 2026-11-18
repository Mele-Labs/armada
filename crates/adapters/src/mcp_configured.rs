//! The MCP servers the agent CLI's own files configure for a folder, read whole, so a process that
//! is running can be handed one added after it started. The CLI never reads these files again
//! once it is up: `docs/spikes/040-can-a-running-job-load-a-server-it-added.md`.
//!
//! **Kept whole, and never shown.** `existing_setup` keeps a server's program and drops its
//! arguments and environment, because it draws a row. This keeps every key, because the process
//! needs them to start the server, so [`McpConfig`] has no `Debug` that prints one and nothing
//! here reaches the wire.

use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

/// The person's own file: user scope at the top, local scope under `projects`.
const USER_FILE: &str = ".claude.json";
/// A folder's file: project scope.
const PROJECT_FILE: &str = ".mcp.json";

/// One server's settings as its file holds them, handed to the process unread.
#[derive(Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(transparent)]
pub struct McpConfig(Json);

impl fmt::Debug for McpConfig {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        out.write_str("McpConfig(..)")
    }
}

/// Any JSON, so this crate keeps a value without a JSON library of its own.
#[derive(Clone, PartialEq, Deserialize, Serialize)]
#[serde(untagged)]
enum Json {
    Null,
    Bool(bool),
    Whole(i64),
    Fraction(f64),
    Text(String),
    List(Vec<Json>),
    Map(BTreeMap<String, Json>),
}

/// JSON holds no NaN, so no value read here is unequal to itself.
impl Eq for Json {}

/// A server a file names, and the scope the CLI would list it under.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Configured {
    pub scope: &'static str,
    pub config: McpConfig,
}

/// When each file was last written, so a caller reads them again only once one has changed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConfigStamp(Vec<Option<(SystemTime, u64)>>);

/// Where the CLI reads one folder's servers from.
#[derive(Clone, Debug)]
pub struct ConfigPlaces {
    user: PathBuf,
    /// `.mcp.json` in the folder and in each folder above it, nearest first.
    project: Vec<PathBuf>,
    /// The keys of `projects` the local scope may sit under: the folder, and the main checkout,
    /// which is where `claude mcp add` puts it when run in a worktree (measured, spike 40).
    local_keys: Vec<String>,
}

impl ConfigPlaces {
    /// The places for a process with `home` as its home and `folder` as where it runs.
    pub fn of(home: &Path, folder: &Path) -> ConfigPlaces {
        let mut local_keys = vec![folder.to_string_lossy().into_owned()];
        if let Some(main) = main_checkout(folder) {
            let main = main.to_string_lossy().into_owned();
            if !local_keys.contains(&main) {
                local_keys.push(main);
            }
        }
        ConfigPlaces {
            user: home.join(USER_FILE),
            project: folder.ancestors().map(|above| above.join(PROJECT_FILE)).collect(),
            local_keys,
        }
    }

    pub fn stamp(&self) -> ConfigStamp {
        ConfigStamp(
            std::iter::once(&self.user)
                .chain(&self.project)
                .map(|path| fs::metadata(path).ok().and_then(|meta| Some((meta.modified().ok()?, meta.len()))))
                .collect(),
        )
    }

    /// Every server these places name, by name, taken as the CLI takes a name found twice: local
    /// scope first, then the nearest `.mcp.json`, then user scope. `Err` names a file that is there
    /// and could not be read, which a caller reads as "nothing known", never as "no servers".
    pub fn read(&self) -> Result<BTreeMap<String, Configured>, String> {
        let mut found = BTreeMap::new();
        let user: UserFile = match fs::read(&self.user) {
            Ok(bytes) => ipc::decode("the agent's own settings", &bytes)
                .map_err(|why| format!("{}: {why}", self.user.display()))?,
            Err(why) if why.kind() == std::io::ErrorKind::NotFound => UserFile::default(),
            Err(why) => return Err(format!("{}: {why}", self.user.display())),
        };
        let mut turned_off = Vec::new();
        for key in &self.local_keys {
            if let Some(project) = user.projects.get(key) {
                take(&mut found, &project.servers, "local");
                turned_off.extend(project.turned_off.iter().cloned());
            }
        }
        for path in &self.project {
            let bytes = match fs::read(path) {
                Ok(bytes) => bytes,
                Err(why) if why.kind() == std::io::ErrorKind::NotFound => continue,
                Err(why) => return Err(format!("{}: {why}", path.display())),
            };
            let file: ProjectFile =
                ipc::decode("a folder's servers", &bytes).map_err(|why| format!("{}: {why}", path.display()))?;
            let kept: BTreeMap<String, McpConfig> =
                file.servers.into_iter().filter(|(name, _)| !turned_off.contains(name)).collect();
            take(&mut found, &kept, "project");
        }
        take(&mut found, &user.servers, "user");
        Ok(found)
    }
}

fn take(found: &mut BTreeMap<String, Configured>, servers: &BTreeMap<String, McpConfig>, scope: &'static str) {
    for (name, config) in servers {
        found.entry(name.clone()).or_insert_with(|| Configured { scope, config: config.clone() });
    }
}

/// The folder whose `.git` is a directory, for a folder inside a worktree of it. A worktree's
/// `.git` is a file naming its git dir, whose `commondir` names the main one.
fn main_checkout(folder: &Path) -> Option<PathBuf> {
    let root = folder.ancestors().find(|above| above.join(".git").exists())?;
    let dot_git = root.join(".git");
    if dot_git.is_dir() {
        return Some(root.to_path_buf());
    }
    let pointer = fs::read_to_string(&dot_git).ok()?;
    let git_dir = root.join(pointer.trim().strip_prefix("gitdir:")?.trim());
    let common = fs::read_to_string(git_dir.join("commondir")).ok()?;
    let common = git_dir.join(common.trim());
    let common = fs::canonicalize(&common).unwrap_or(common);
    (common.file_name()? == ".git").then(|| common.parent().map(Path::to_path_buf)).flatten()
}

/// The person's own file. **Every key but the servers is dropped**: it also holds the account and
/// each project's history.
#[derive(Default, Deserialize)]
struct UserFile {
    #[serde(default, rename = "mcpServers")]
    servers: BTreeMap<String, McpConfig>,
    #[serde(default)]
    projects: BTreeMap<String, LocalScope>,
}

#[derive(Deserialize)]
struct LocalScope {
    #[serde(default, rename = "mcpServers")]
    servers: BTreeMap<String, McpConfig>,
    /// The `.mcp.json` servers the person turned down for this folder.
    #[serde(default, rename = "disabledMcpjsonServers")]
    turned_off: Vec<String>,
}

#[derive(Deserialize)]
struct ProjectFile {
    #[serde(default, rename = "mcpServers")]
    servers: BTreeMap<String, McpConfig>,
}
