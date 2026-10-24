//! Each kind's pieces on each side, in one form: a name and its files, relative to where the
//! kind is kept. Reading either side gives the same shape, so comparing them is comparing maps.
//!
//! Inside the home every kind has a folder of its own named for its wire word: `mods/<name>/…`,
//! `skills/<name>/…`, `sub_agents/<name>.md`, `commands/<name>.md`, `global_instructions/<file>`,
//! `workflows/<file>`, `memory/<repository>/<note>.md`, `plugins/<name>.toml`,
//! `mcp_servers/<name>.toml`, and the allowlist as one file, `allowlist`, a line a command.

use std::collections::BTreeMap;

use adapter_traits::{FileRead, HarnessPiece, HarnessPlace, KeptFolder, PluginRecord};
use core_model::{KitServer, ServerAddress};
use ipc::{HomeKind, HomeUnreadable};
use serde::{Deserialize, Serialize};

use super::{Places, Refused};

/// The most bytes one file may hold to travel. A Kit file is a page of text.
const MOST_BYTES: usize = 512 * 1024;
/// How deep a folder piece is read: a skill's own folders, and theirs.
const MOST_DEPTH: usize = 3;
/// Kit's allowlist, inside Kit's home: `crate::kit_allowlist`'s file.
const ALLOWLIST_HERE: &str = crate::kit_allowlist::FILE;
const ALLOWLIST_HOME: &str = "allowlist";
const WORKFLOWS_HERE: &str = "workflows";
const RECORD: &str = ".toml";

/// One piece, on either side.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Piece {
    pub(crate) name: String,
    /// A memory note's repository, by its folder's name.
    pub(crate) repository: Option<String>,
    /// Each file's text, by its path relative to where the kind is kept: `humanizer/SKILL.md`,
    /// `bug.yml`. Empty for a kind that is only there or not: an allowlist line, a plugin.
    pub(crate) files: BTreeMap<String, String>,
    pub(crate) says: Option<String>,
    pub(crate) source: Option<String>,
}

/// One side of one kind.
#[derive(Default)]
pub(crate) struct Side {
    pub(crate) pieces: Vec<Piece>,
    pub(crate) unreadable: Vec<HomeUnreadable>,
}

/// Where a kind's files sit in one folder.
struct Spot<'a> {
    folder: &'a dyn KeptFolder,
    dir: String,
    shape: Shape,
}

enum Shape {
    /// A folder a piece, and every file under it but a dotted one.
    Folders,
    /// A file a piece. `None` takes any extension, and the name is what comes before it.
    Files(Option<String>),
    /// One file, which is the piece.
    One(String),
}

fn join(dir: &str, rest: &str) -> String {
    match dir.trim_matches('/') {
        "" => rest.to_string(),
        dir => format!("{dir}/{rest}"),
    }
}

/// A name that may be a path segment: from the other machine, so never trusted to be one.
fn plain(name: &str) -> bool {
    !name.is_empty() && !name.starts_with('.') && !name.contains('/') && !name.contains('\\')
}

fn harness_spot<'a>(places: &Places<'a>, piece: HarnessPiece) -> Spot<'a> {
    let folder = places.harness.folder();
    match places.harness.place(piece) {
        HarnessPlace::Folders { dir } => Spot { folder, dir, shape: Shape::Folders },
        HarnessPlace::Files { dir, suffix } => Spot { folder, dir, shape: Shape::Files(Some(suffix)) },
        HarnessPlace::One { path } => {
            let (dir, file) = path.rsplit_once('/').unwrap_or(("", path.as_str()));
            Spot { folder, dir: dir.to_string(), shape: Shape::One(file.to_string()) }
        }
    }
}

fn harness_piece(kind: HomeKind) -> Option<HarnessPiece> {
    match kind {
        HomeKind::Skills => Some(HarnessPiece::Skills),
        HomeKind::SubAgents => Some(HarnessPiece::SubAgents),
        HomeKind::Commands => Some(HarnessPiece::Commands),
        HomeKind::GlobalInstructions => Some(HarnessPiece::GlobalInstructions),
        _ => None,
    }
}

/// Where a kind of files is kept on this machine. `None` for the kinds that are not files there.
fn here_spot<'a>(places: &Places<'a>, kind: HomeKind) -> Option<Spot<'a>> {
    if let Some(piece) = harness_piece(kind) {
        return Some(harness_spot(places, piece));
    }
    match kind {
        HomeKind::Mods => Some(Spot { folder: places.mods, dir: String::new(), shape: Shape::Folders }),
        HomeKind::Workflows => Some(Spot { folder: places.kit, dir: WORKFLOWS_HERE.to_string(), shape: Shape::Files(None) }),
        _ => None,
    }
}

/// Where the same kind is kept in the home. The global instructions keep this machine's file name.
fn home_spot<'a>(places: &Places<'a>, kind: HomeKind) -> Option<Spot<'a>> {
    let folder = places.home.files();
    let dir = kind.as_wire().to_string();
    let shape = match kind {
        HomeKind::Mods | HomeKind::Skills => Shape::Folders,
        HomeKind::SubAgents | HomeKind::Commands => Shape::Files(Some(".md".to_string())),
        HomeKind::Workflows => Shape::Files(None),
        HomeKind::GlobalInstructions => match harness_spot(places, HarnessPiece::GlobalInstructions).shape {
            Shape::One(file) => Shape::One(file),
            _ => return None,
        },
        _ => return None,
    };
    Some(Spot { folder, dir, shape })
}

fn text_of(folder: &dyn KeptFolder, path: &str) -> Result<Option<String>, String> {
    match folder.read(path) {
        FileRead::Absent => Ok(None),
        FileRead::Unreadable(why) => Err(why),
        FileRead::Bytes(bytes) if bytes.len() > MOST_BYTES => Err(format!("larger than {MOST_BYTES} bytes")),
        FileRead::Bytes(bytes) => String::from_utf8(bytes).map(Some).map_err(|_| "not UTF-8 text".to_string()),
    }
}

/// The files under `dir`, by path relative to `base`, none dotted.
fn walk(spot: &Spot<'_>, base: &str, depth: usize, files: &mut BTreeMap<String, String>, bad: &mut Vec<(String, String)>) {
    let Ok(entries) = spot.folder.entries(&join(&spot.dir, base)) else { return };
    for entry in entries.into_iter().filter(|entry| !entry.name.starts_with('.')) {
        let relative = join(base, &entry.name);
        if entry.is_dir {
            if depth > 1 {
                walk(spot, &relative, depth - 1, files, bad);
            }
            continue;
        }
        match text_of(spot.folder, &join(&spot.dir, &relative)) {
            Ok(Some(text)) => {
                files.insert(relative, text);
            }
            Ok(None) => {}
            Err(why) => bad.push((join(&spot.dir, &relative), why)),
        }
    }
}

fn read_spot(spot: &Spot<'_>, kind: HomeKind, side: &mut Side) {
    let mut bad = Vec::new();
    match &spot.shape {
        Shape::One(file) => match text_of(spot.folder, &join(&spot.dir, file)) {
            Ok(Some(text)) => side.pieces.push(piece(file, BTreeMap::from([(file.clone(), text)]))),
            Ok(None) => {}
            Err(why) => bad.push((join(&spot.dir, file), why)),
        },
        Shape::Folders | Shape::Files(_) => {
            let Ok(entries) = spot.folder.entries(&spot.dir) else { return };
            for entry in entries.into_iter().filter(|entry| plain(&entry.name)) {
                let name = match (&spot.shape, entry.is_dir) {
                    (Shape::Folders, true) => entry.name.clone(),
                    (Shape::Files(Some(suffix)), false) => match entry.name.strip_suffix(suffix.as_str()) {
                        Some(name) if !name.is_empty() => name.to_string(),
                        _ => continue,
                    },
                    (Shape::Files(None), false) => entry.name.rsplit_once('.').map_or(entry.name.as_str(), |(stem, _)| stem).to_string(),
                    _ => continue,
                };
                let mut files = BTreeMap::new();
                match spot.shape {
                    Shape::Folders => walk(spot, &entry.name, MOST_DEPTH, &mut files, &mut bad),
                    _ => match text_of(spot.folder, &join(&spot.dir, &entry.name)) {
                        Ok(Some(text)) => {
                            files.insert(entry.name.clone(), text);
                        }
                        Ok(None) => {}
                        Err(why) => bad.push((join(&spot.dir, &entry.name), why)),
                    },
                }
                if !files.is_empty() {
                    side.pieces.push(piece(&name, files));
                }
            }
        }
    }
    side.unreadable.extend(bad.into_iter().map(|(at, why)| HomeUnreadable { kind, at, why }));
}

fn piece(name: &str, files: BTreeMap<String, String>) -> Piece {
    let says = files.values().find_map(|text| described(text));
    Piece { name: name.to_string(), repository: None, files, says, source: None }
}

/// A file's own `description:` in its front matter, where it opens with one.
fn described(text: &str) -> Option<String> {
    let rest = text.strip_prefix("---")?;
    let block = rest.split("\n---").next()?;
    block.lines().find_map(|line| {
        let said = line.strip_prefix("description:")?.trim().trim_matches(|c| c == '"' || c == '\'');
        (!said.is_empty()).then(|| said.chars().take(240).collect())
    })
}

/// A plugin as the home keeps it.
#[derive(Serialize, Deserialize)]
struct PluginKept {
    name: String,
    marketplace: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    source: Option<String>,
}

/// A server as the home keeps it: **a name and a host, and nothing a server could be started with.**
#[derive(Serialize, Deserialize)]
struct ServerKept {
    name: String,
    host: String,
}

/// What a server is at, as a host: an address's host and port with no userinfo, or a program's
/// own file name. Never a path, a query or an argument.
pub(crate) fn host_of(address: &ServerAddress) -> String {
    match address {
        ServerAddress::Http { url } => {
            let after = url.split_once("://").map_or(url.as_str(), |(_, rest)| rest);
            let authority = after.split(['/', '?', '#']).next().unwrap_or("");
            authority.rsplit_once('@').map_or(authority, |(_, host)| host).to_string()
        }
        ServerAddress::Stdio { command, .. } => command.rsplit('/').next().unwrap_or(command).to_string(),
    }
}

fn record<T: Serialize>(name: &str, kept: &T) -> Result<BTreeMap<String, String>, String> {
    let text = toml::to_string(kept).map_err(|why| why.to_string())?;
    Ok(BTreeMap::from([(format!("{name}{RECORD}"), text)]))
}

fn server_piece(server: &KitServer) -> Option<Piece> {
    let name = server.name.as_str().to_string();
    let files = record(&name, &ServerKept { name: name.clone(), host: host_of(&server.address) }).ok()?;
    Some(Piece { name, repository: None, files, says: Some(host_of(&server.address)), source: None })
}

fn plugin_piece(plugin: &PluginRecord) -> Option<Piece> {
    let kept = PluginKept { name: plugin.name.clone(), marketplace: plugin.marketplace.clone(), source: plugin.source.clone() };
    let files = record(&plugin.name, &kept).ok()?;
    Some(Piece { name: plugin.name.clone(), repository: None, files, says: None, source: None })
}

/// The allowlist's lines as pieces: the command is the name and what follows its tab the source.
fn lines_of(text: &str) -> Vec<Piece> {
    text.lines()
        .map(|line| line.trim_end_matches('\r'))
        .filter(|line| !line.trim().is_empty() && !line.trim_start().starts_with('#'))
        .filter_map(|line| {
            let (run, tail) = line.split_once('\t').map_or((line, None), |(run, tail)| (run, Some(tail.trim())));
            let run = run.trim();
            (!run.is_empty()).then(|| Piece {
                name: run.to_string(),
                repository: None,
                files: BTreeMap::new(),
                says: None,
                source: tail.filter(|tail| !tail.is_empty()).map(str::to_string),
            })
        })
        .collect()
}

fn read_lines(folder: &dyn KeptFolder, path: &str, side: &mut Side) {
    match text_of(folder, path) {
        Ok(Some(text)) => side.pieces.extend(lines_of(&text)),
        Ok(None) => {}
        Err(why) => side.unreadable.push(HomeUnreadable { kind: HomeKind::Allowlist, at: path.to_string(), why }),
    }
}

fn read_records(spot: Spot<'_>, kind: HomeKind, side: &mut Side) {
    let mut read = Side::default();
    read_spot(&spot, kind, &mut read);
    side.unreadable.extend(read.unreadable);
    side.pieces.extend(read.pieces.into_iter().map(|piece| Piece { says: None, ..piece }));
}

/// A note in a memory folder, but the folder's index.
fn read_memory(places: &Places<'_>, dir: String, repository: &str, side: &mut Side, folder: &dyn KeptFolder) {
    let index = places.harness.memory_index();
    let mut read = Side::default();
    read_spot(&Spot { folder, dir, shape: Shape::Files(Some(".md".to_string())) }, HomeKind::Memory, &mut read);
    side.unreadable.extend(read.unreadable);
    side.pieces.extend(
        read.pieces
            .into_iter()
            .filter(|piece| !piece.files.contains_key(&index))
            .map(|piece| Piece { repository: Some(repository.to_string()), source: Some(repository.to_string()), ..piece }),
    );
}

/// Every piece of `kind` on this machine.
pub(crate) fn here(places: &Places<'_>, kind: HomeKind) -> Side {
    let mut side = Side::default();
    match kind {
        HomeKind::Allowlist => read_lines(places.kit, ALLOWLIST_HERE, &mut side),
        HomeKind::McpServers => side.pieces.extend(places.servers.iter().filter_map(server_piece)),
        HomeKind::Plugins => match places.harness.plugins() {
            Ok(plugins) => side.pieces.extend(plugins.iter().filter_map(plugin_piece)),
            Err(why) => side.unreadable.push(HomeUnreadable { kind, at: "the installed plugins".to_string(), why }),
        },
        HomeKind::Memory => {
            for repository in places.repositories {
                let dir = places.harness.memory_dir(&repository.root);
                read_memory(places, dir, &repository.name, &mut side, places.harness.folder());
            }
        }
        _ => {
            if let Some(spot) = here_spot(places, kind) {
                read_spot(&spot, kind, &mut side);
            }
        }
    }
    side
}

/// Every piece of `kind` in the home's copy here.
pub(crate) fn home(places: &Places<'_>, kind: HomeKind) -> Side {
    let mut side = Side::default();
    let folder = places.home.files();
    match kind {
        HomeKind::Allowlist => read_lines(folder, ALLOWLIST_HOME, &mut side),
        HomeKind::McpServers | HomeKind::Plugins => {
            let spot = Spot { folder, dir: kind.as_wire().to_string(), shape: Shape::Files(Some(RECORD.to_string())) };
            read_records(spot, kind, &mut side);
        }
        HomeKind::Memory => {
            let Ok(entries) = folder.entries(kind.as_wire()) else { return side };
            for entry in entries.into_iter().filter(|entry| entry.is_dir && plain(&entry.name)) {
                read_memory(places, join(kind.as_wire(), &entry.name), &entry.name, &mut side, folder);
            }
        }
        _ => {
            if let Some(spot) = home_spot(places, kind) {
                read_spot(&spot, kind, &mut side);
            }
        }
    }
    side
}

fn cannot(why: impl std::fmt::Display) -> Refused {
    Refused::Cannot(why.to_string())
}

/// Write `piece`'s files at `dir`, and take away what `replacing` had that it has not. The paths
/// it touched.
fn put_files(folder: &dyn KeptFolder, dir: &str, piece: &Piece, replacing: Option<&Piece>) -> Result<Vec<String>, Refused> {
    let mut touched = Vec::new();
    for (path, text) in &piece.files {
        let at = join(dir, path);
        folder.write(&at, text).map_err(cannot)?;
        touched.push(at);
    }
    for gone in replacing.into_iter().flat_map(|was| was.files.keys()).filter(|path| !piece.files.contains_key(*path)) {
        let at = join(dir, gone);
        folder.remove(&at).map_err(cannot)?;
        touched.push(at);
    }
    Ok(touched)
}

fn append_line(folder: &dyn KeptFolder, path: &str, piece: &Piece) -> Result<Vec<String>, Refused> {
    let mut text = text_of(folder, path).map_err(cannot)?.unwrap_or_default();
    if !text.is_empty() && !text.ends_with('\n') {
        text.push('\n');
    }
    match &piece.source {
        Some(source) => text.push_str(&format!("{}\t{source}\n", piece.name)),
        None => text.push_str(&format!("{}\n", piece.name)),
    }
    folder.write(path, &text).map_err(cannot)?;
    Ok(vec![path.to_string()])
}

fn memory_root<'a>(places: &Places<'a>, piece: &Piece) -> Result<&'a str, Refused> {
    let named = piece.repository.as_deref().unwrap_or("");
    places
        .repositories
        .iter()
        .find(|repository| repository.name == named)
        .map(|repository| repository.root.as_str())
        .ok_or_else(|| cannot(format!("{named} is not a repository this machine serves, so its memory has nowhere to go")))
}

/// List a note brought here in its folder's index, where the index does not name it yet.
fn indexed(places: &Places<'_>, dir: &str, piece: &Piece) -> Result<(), Refused> {
    let folder = places.harness.folder();
    let index = join(dir, &places.harness.memory_index());
    let Some(file) = piece.files.keys().next() else { return Ok(()) };
    let mut text = text_of(folder, &index).map_err(cannot)?.unwrap_or_default();
    if text.contains(&format!("({file})")) {
        return Ok(());
    }
    if !text.is_empty() && !text.ends_with('\n') {
        text.push('\n');
    }
    let says = piece.says.as_deref().map(|says| format!(" — {says}")).unwrap_or_default();
    text.push_str(&format!("- [{}]({file}){says}\n", piece.name));
    folder.write(&index, &text).map_err(cannot)
}

/// Put `piece` on this machine, over `replacing` where it replaces this machine's own.
pub(crate) fn put_here(places: &Places<'_>, kind: HomeKind, piece: &Piece, replacing: Option<&Piece>) -> Result<(), Refused> {
    match kind {
        HomeKind::Allowlist => append_line(places.kit, ALLOWLIST_HERE, piece).map(|_| ()),
        HomeKind::McpServers => Err(cannot(
            "The home carries a server's name and host and never its key, so it is added here in Kit, with its key",
        )),
        HomeKind::Plugins => {
            let text = piece.files.values().next().ok_or_else(|| cannot("the home's record of it is empty"))?;
            let kept: PluginKept = toml::from_str(text).map_err(|why| cannot(format!("the home's record of it would not read: {why}")))?;
            let plugin = PluginRecord { name: kept.name, marketplace: kept.marketplace, source: kept.source };
            places.harness.install(&plugin).map_err(cannot)
        }
        HomeKind::Memory => {
            let dir = places.harness.memory_dir(memory_root(places, piece)?);
            put_files(places.harness.folder(), &dir, piece, replacing)?;
            indexed(places, &dir, piece)
        }
        _ => {
            let spot = here_spot(places, kind).ok_or_else(|| cannot("this kind has no place on this machine"))?;
            put_files(spot.folder, &spot.dir, piece, replacing).map(|_| ())
        }
    }
}

/// Put `piece` in the home's copy here, and the paths it touched, to publish.
pub(crate) fn put_home(places: &Places<'_>, kind: HomeKind, piece: &Piece, replacing: Option<&Piece>) -> Result<Vec<String>, Refused> {
    let folder = places.home.files();
    match kind {
        HomeKind::Allowlist => append_line(folder, ALLOWLIST_HOME, piece),
        HomeKind::McpServers | HomeKind::Plugins => put_files(folder, kind.as_wire(), piece, replacing),
        HomeKind::Memory => {
            let repository = piece.repository.as_deref().filter(|name| plain(name)).ok_or_else(|| cannot("a memory note names no repository"))?;
            put_files(folder, &join(kind.as_wire(), repository), piece, replacing)
        }
        _ => {
            let spot = home_spot(places, kind).ok_or_else(|| cannot("this kind has no place in the home"))?;
            put_files(folder, &spot.dir, piece, replacing)
        }
    }
}
