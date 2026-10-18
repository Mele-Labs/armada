//! One Loadout row's file, read whole for a person to open — `read_loadout_item`.
//!
//! **Only the one file the kind and whose imply, and never outside the folder that holds it.** A
//! name is the query's: one that could climb or name a second folder is refused before anything
//! is opened, and the file is resolved through every link and must still be under that folder or
//! the person's home, so a link to anywhere else reads nothing.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use adapter_traits::{ItemAsked, ItemKind, ItemRead, ItemWhose};

use super::{front_matter, Installed, HOME, OUTPUT_STYLES, PLUGINS, PLUGIN_STYLES, SKILLS, SKILL_DEPTH, SKILL_FILE, SUB_AGENTS};

/// A plugin's own folders of skills and agents, under where it is installed.
const PLUGIN_SKILLS: &str = "skills";
const PLUGIN_AGENTS: &str = "agents";

/// How deep under an agents folder a file is looked for by its own name: the depth Kit reads.
const AGENT_DEPTH: usize = 1;

/// The folder a file must stay under, and how a person reads a path under it.
struct Under {
    /// The home, the repository's root, or the plugin's install folder.
    base: PathBuf,
    /// `~/` under the home, nothing under a repository or a plugin.
    lead: &'static str,
    /// Whose folder it is, in a sentence: nothing under the home, which `~/` already says.
    owner: &'static str,
    /// The folder under `base`, `/`-separated, empty for `base` itself.
    dir: String,
}

impl Under {
    fn folder(&self) -> PathBuf {
        match self.dir.is_empty() {
            true => self.base.clone(),
            false => self.base.join(&self.dir),
        }
    }

    /// The folder, in a sentence.
    fn named(&self) -> String {
        match self.dir.is_empty() {
            true => format!("{}root", self.owner),
            false => format!("{}{}{}", self.owner, self.lead, self.dir),
        }
    }

    fn shown(&self, relative: &str) -> String {
        match self.dir.is_empty() {
            true => format!("{}{relative}", self.lead),
            false => format!("{}{}/{relative}", self.lead, self.dir),
        }
    }
}

pub(super) fn read_item(home: &str, asked: &ItemAsked<'_>) -> ItemRead {
    match find(home, asked) {
        Ok(read) => read,
        Err(why) => ItemRead::NotRead { why },
    }
}

fn find(home: &str, asked: &ItemAsked<'_>) -> Result<ItemRead, String> {
    let home = absolute(home).ok_or("Fleet knows no home folder to read it from.")?;
    if asked.kind == ItemKind::MemoryFile {
        return memory_file(&home, asked);
    }
    if !one_part(asked.name) {
        return Err(format!("`{}` is not a name Armada looks a file up by.", asked.name));
    }
    let under = folder_for(&home, asked)?;
    let found = match asked.kind {
        ItemKind::Skill => skill_in(&under, asked.name),
        ItemKind::Subagent => markdown_in(&under, asked.name, AGENT_DEPTH),
        ItemKind::OutputStyle => markdown_in(&under, asked.name, 1),
        ItemKind::MemoryFile => None,
    };
    let what = match asked.kind {
        ItemKind::Skill => "skill",
        ItemKind::Subagent => "agent",
        _ => "output style",
    };
    let relative = found.ok_or_else(|| format!("No {what} called `{}` is in {}.", asked.name, under.named()))?;
    read_under(&under, &relative, &home, asked.cap)
}

/// The folder a skill, an agent or a style of this whose is kept in.
fn folder_for(home: &Path, asked: &ItemAsked<'_>) -> Result<Under, String> {
    let (kit, plugin) = match asked.kind {
        ItemKind::Skill => (SKILLS, PLUGIN_SKILLS),
        ItemKind::Subagent => (SUB_AGENTS, PLUGIN_AGENTS),
        _ => (OUTPUT_STYLES, PLUGIN_STYLES),
    };
    Ok(match asked.whose {
        ItemWhose::Kit => Under { base: home.to_path_buf(), lead: "~/", owner: "", dir: kit.to_string() },
        ItemWhose::Repository => Under { base: repository(asked.root)?, lead: "", owner: "the repository's ", dir: kit.to_string() },
        ItemWhose::Plugin(name) => Under { base: plugin_root(home, name)?, lead: "", owner: "the plugin's ", dir: plugin.to_string() },
    })
}

/// A memory file, at the path the mod reported: `~/.claude/…`, from the repository's root, or an
/// absolute path under one of those two.
fn memory_file(home: &Path, asked: &ItemAsked<'_>) -> Result<ItemRead, String> {
    let name = asked.name;
    let kit = || Under { base: home.to_path_buf(), lead: "~/", owner: "", dir: HOME.to_string() };
    let refused = || format!("Armada reads a memory file only under the job's repository or ~/{HOME}, and not `{name}`.");
    let (under, relative) = if let Some(rest) = name.strip_prefix(&format!("~/{HOME}/")) {
        (kit(), rest.to_string())
    } else if name.starts_with('/') {
        let at = Path::new(name);
        match repository(asked.root).ok().filter(|root| at.starts_with(root)) {
            Some(root) => {
                let rest = at.strip_prefix(&root).map(|rest| rest.to_string_lossy().to_string()).unwrap_or_default();
                (Under { base: root, lead: "", owner: "the repository's ", dir: String::new() }, rest)
            }
            None => match at.strip_prefix(home.join(HOME)) {
                Ok(rest) => (kit(), rest.to_string_lossy().to_string()),
                Err(_) => return Err(refused()),
            },
        }
    } else if name.starts_with('~') {
        return Err(refused());
    } else {
        (Under { base: repository(asked.root)?, lead: "", owner: "the repository's ", dir: String::new() }, name.to_string())
    };
    let relative = parts(&relative).ok_or_else(refused)?;
    read_under(&under, &relative, home, asked.cap)
}

/// The session's repository root, which Fleet supplies and the query does not.
fn repository(root: &str) -> Result<PathBuf, String> {
    absolute(root).ok_or_else(|| "Fleet knows no folder this job stands in.".to_string())
}

fn absolute(path: &str) -> Option<PathBuf> {
    let path = path.trim_end_matches('/');
    (path.starts_with('/') && !path.contains('\0')).then(|| PathBuf::from(path))
}

/// Where the harness installed a plugin, off its own record of what is installed.
fn plugin_root(home: &Path, plugin: &str) -> Result<PathBuf, String> {
    let missing = || format!("The plugin `{plugin}` is not installed in ~/{HOME}, so there is no folder of its to read.");
    if !one_part(plugin) || plugin.contains('@') {
        return Err(format!("`{plugin}` is not a name a plugin is installed under."));
    }
    let bytes = fs::read(home.join(PLUGINS)).map_err(|_| missing())?;
    let installed = ipc::decode::<Installed>("installed plugins", &bytes).map_err(|_| missing())?;
    installed
        .plugins
        .iter()
        .find(|(name, _)| name.split('@').next() == Some(plugin))
        .and_then(|(_, versions)| versions.first())
        .and_then(|held| held.install_path.as_deref())
        .and_then(absolute)
        .ok_or_else(missing)
}

/// A skill's folder: by its own name first, then any folder a few levels down whose name or whose
/// `SKILL.md` says it, which is where a synced set lands.
fn skill_in(under: &Under, name: &str) -> Option<String> {
    let direct = format!("{name}/{SKILL_FILE}");
    if under.folder().join(&direct).is_file() {
        return Some(direct);
    }
    skill_under(&under.folder(), "", name, SKILL_DEPTH)
}

fn skill_under(folder: &Path, at: &str, name: &str, depth: usize) -> Option<String> {
    for entry in sorted(&folder.join(at)) {
        let here = joined(at, &entry);
        let file = format!("{here}/{SKILL_FILE}");
        match fs::read(folder.join(&file)) {
            Ok(bytes) => {
                let said = front_matter(&bytes).ok().and_then(|front| front.name);
                if entry == name || said.as_deref() == Some(name) {
                    return Some(file);
                }
            }
            Err(_) if depth > 1 && folder.join(&here).is_dir() => {
                if let Some(found) = skill_under(folder, &here, name, depth - 1) {
                    return Some(found);
                }
            }
            Err(_) => {}
        }
    }
    None
}

/// An agent or a style: `<name>.md`, or the markdown file whose front matter or path names it, as
/// Kit names one.
fn markdown_in(under: &Under, name: &str, depth: usize) -> Option<String> {
    let direct = format!("{name}.md");
    if under.folder().join(&direct).is_file() {
        return Some(direct);
    }
    markdown_under(&under.folder(), "", name, depth)
}

fn markdown_under(folder: &Path, at: &str, name: &str, depth: usize) -> Option<String> {
    for entry in sorted(&folder.join(at)) {
        let here = joined(at, &entry);
        let full = folder.join(&here);
        if full.is_dir() {
            if depth > 1 {
                if let Some(found) = markdown_under(folder, &here, name, depth - 1) {
                    return Some(found);
                }
            }
            continue;
        }
        let Some(stem) = here.strip_suffix(".md") else { continue };
        let said = fs::read(&full).ok().and_then(|bytes| front_matter(&bytes).ok()).and_then(|front| front.name);
        if stem.replace('/', ":") == name || said.as_deref() == Some(name) {
            return Some(here);
        }
    }
    None
}

fn sorted(folder: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(folder)
        .map(|entries| entries.flatten().map(|entry| entry.file_name().to_string_lossy().to_string()).collect())
        .unwrap_or_default();
    names.sort();
    names
}

fn joined(at: &str, name: &str) -> String {
    match at.is_empty() {
        true => name.to_string(),
        false => format!("{at}/{name}"),
    }
}

/// The file at `relative` under `under`, resolved through every link, and refused where that lands
/// outside both the folder and the person's home. **A link out to the home is followed**, as Kit
/// follows one (`home.rs`): it is where a dotfiles repository puts what it links in.
fn read_under(under: &Under, relative: &str, home: &Path, cap: usize) -> Result<ItemRead, String> {
    let shown = under.shown(relative);
    let gone = || format!("Nothing is at {shown} now.");
    let folder = fs::canonicalize(under.folder()).map_err(|_| gone())?;
    let file = fs::canonicalize(under.folder().join(relative)).map_err(|_| gone())?;
    let home = fs::canonicalize(home).ok();
    if !file.starts_with(&folder) && !home.is_some_and(|home| file.starts_with(home)) {
        return Err(format!("{shown} is a link to outside {} and your home, and Armada reads nothing there.", under.named()));
    }
    if !file.is_file() {
        return Err(format!("{shown} is not a file."));
    }
    let mut bytes = Vec::new();
    let opened = fs::File::open(&file).map_err(|why| format!("{shown} would not open: {why}."))?;
    opened
        .take(u64::try_from(cap).unwrap_or(u64::MAX).saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|why| format!("{shown} would not read: {why}."))?;
    let truncated = bytes.len() > cap;
    bytes.truncate(cap);
    Ok(ItemRead::Read { markdown: relative.ends_with(".md"), body: text_of(bytes, truncated), path: shown, truncated })
}

/// The bytes as text. A character the cap cut in half is left off rather than drawn as a mark.
fn text_of(bytes: Vec<u8>, truncated: bool) -> String {
    match String::from_utf8(bytes) {
        Ok(text) => text,
        Err(failed) => {
            let error = failed.utf8_error();
            let mut bytes = failed.into_bytes();
            if truncated && error.error_len().is_none() {
                bytes.truncate(error.valid_up_to());
            }
            String::from_utf8_lossy(&bytes).into_owned()
        }
    }
}

/// One name, not a path: nothing that could climb or reach a second folder.
fn one_part(name: &str) -> bool {
    !name.is_empty() && name != "." && name != ".." && !name.contains(['/', '\\', '\0'])
}

/// A relative path with no part that climbs, `/`-joined; `None` for one that would.
fn parts(path: &str) -> Option<String> {
    if path.starts_with('/') || path.contains(['\\', '\0']) {
        return None;
    }
    let kept: Vec<&str> = path.split('/').filter(|part| !part.is_empty() && *part != ".").collect();
    (!kept.is_empty() && !kept.contains(&"..")).then(|| kept.join("/"))
}
