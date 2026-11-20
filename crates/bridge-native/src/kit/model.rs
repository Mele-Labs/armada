//! What Kit draws, worked out from what Fleet read: the kinds, a kind's pieces grouped by where they come
//! from, which of them are on in the scoped project, and the one setting that says so for skills. Nothing
//! here touches the screen or the wire.

use std::collections::BTreeMap;

use ipc::{KitInventory, KitPieceKind, KitServerRow, SettingValue, SettingsList, WhatWasRead};

use crate::look::{token, Token};

/// Skills a repository's Jobs start without, by Manifest id (`tokens.skillsOff`).
pub const SKILLS_OFF: &str = "tokens.skillsOff";

/// One kind of piece, a row of the rail.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    Skills,
    Plugins,
    SubAgents,
    Commands,
    Instructions,
    OutputStyles,
    Servers,
    Allowed,
}

pub const KINDS: [Kind; 8] = [Kind::Skills, Kind::Plugins, Kind::SubAgents, Kind::Commands, Kind::Instructions, Kind::OutputStyles, Kind::Servers, Kind::Allowed];

impl Kind {
    /// The word the palette and a deep link name it by.
    pub fn id(self) -> &'static str {
        match self {
            Kind::Skills => "skills",
            Kind::Plugins => "plugins",
            Kind::SubAgents => "sub-agents",
            Kind::Commands => "commands",
            Kind::Instructions => "instructions",
            Kind::OutputStyles => "output-styles",
            Kind::Servers => "servers",
            Kind::Allowed => "allowlist",
        }
    }

    pub fn from_id(id: &str) -> Option<Kind> {
        KINDS.into_iter().find(|kind| kind.id() == id)
    }

    pub fn label(self) -> &'static str {
        match self {
            Kind::Skills => "Skills",
            Kind::Plugins => "Plugins",
            Kind::SubAgents => "Sub agents",
            Kind::Commands => "Commands",
            Kind::Instructions => "Instructions",
            Kind::OutputStyles => "Output styles",
            Kind::Servers => "MCP servers",
            Kind::Allowed => "Allowed commands",
        }
    }

    /// The registry glyph the kind wears.
    pub fn glyph(self) -> &'static str {
        match self {
            Kind::Skills => "zap",
            Kind::Plugins => "package",
            Kind::SubAgents => "bot",
            Kind::Commands => "terminal",
            Kind::Instructions => "scroll-text",
            Kind::OutputStyles => "file-text",
            Kind::Servers => "server",
            Kind::Allowed => "shield-check",
        }
    }

    /// The hue of its tile and glyph (`.h-sk` to `.h-al` in the approved mock).
    pub fn hue(self) -> Token {
        match self {
            Kind::Skills => token::KIND_STEP,
            Kind::Plugins => token::KIND_GATE,
            Kind::SubAgents => token::OPERATOR,
            Kind::Commands => token::TOOL_RUN,
            Kind::Instructions => token::KIND_SETUP,
            Kind::OutputStyles => token::TOOL_CHANGE,
            Kind::Servers => token::STATUS_RUNNING,
            Kind::Allowed => token::KIND_DELIVERY,
        }
    }

    /// What `get_kit_file` and `save_kit_file` call it, where a piece of the kind is a file.
    pub fn file(self) -> Option<ipc::KitFileKind> {
        match self {
            Kind::Skills => Some(ipc::KitFileKind::Skills),
            Kind::SubAgents => Some(ipc::KitFileKind::SubAgents),
            Kind::Commands => Some(ipc::KitFileKind::Commands),
            Kind::Instructions => Some(ipc::KitFileKind::AgentFile),
            Kind::OutputStyles => Some(ipc::KitFileKind::OutputStyles),
            Kind::Plugins | Kind::Servers | Kind::Allowed => None,
        }
    }

    /// What `open_kit_piece` calls it: a file kind, or a plugin, which opens in Finder alone.
    pub fn opens(self) -> Option<ipc::KitOpenKind> {
        match self {
            Kind::Plugins => Some(ipc::KitOpenKind::Plugins),
            kind => kind.file().map(ipc::KitOpenKind::from),
        }
    }

    /// What Fleet calls it in `get_kit_inventory`.
    fn word(self) -> &'static str {
        match self {
            Kind::Skills => "skills",
            Kind::Plugins => "plugins",
            Kind::SubAgents => "sub_agents",
            Kind::Commands => "commands",
            Kind::Instructions => "agent_file",
            Kind::OutputStyles => "output_styles",
            Kind::Servers => "mcp_servers",
            Kind::Allowed => "allowlist",
        }
    }
}

/// Where a piece comes from, which is how the list groups them. `Plain` is a quiet chip: a harness's built-in
/// word, a server connected outside Armada, or where an allowed command came from.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Mark {
    Yours,
    Plugin(String),
    /// A marketplace, which the plugins of Plugins are grouped under.
    Market(String),
    /// A folder's own, where more than one is listed: a repository's `.mcp.json`, or a project's local scope.
    Folder(String),
    Repository,
    Armada,
    /// A server Kit holds itself, added in Kit.
    Kit,
    Plain(String),
}

impl Mark {
    pub fn label(&self) -> String {
        match self {
            Mark::Yours => "Yours".into(),
            // A plugin's group says so, so it never reads as Armada's own.
            Mark::Plugin(name) => format!("{name} plugin"),
            Mark::Market(name) | Mark::Folder(name) | Mark::Plain(name) => name.clone(),
            Mark::Repository => "This repository".into(),
            Mark::Armada => "Armada".into(),
            Mark::Kit => "Kit".into(),
        }
    }

    /// The order groups are drawn in: Yours, then each plugin, This repository, Armada, then the quiet ones.
    fn order(&self) -> (u8, String) {
        match self {
            Mark::Yours => (0, String::new()),
            Mark::Plugin(name) | Mark::Market(name) => (1, name.to_lowercase()),
            Mark::Repository => (2, String::new()),
            Mark::Folder(name) => (2, name.to_lowercase()),
            Mark::Armada | Mark::Kit => (3, String::new()),
            Mark::Plain(name) => (4, name.to_lowercase()),
        }
    }
}

/// What Remove takes out, by the name the inventory spells.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Piece {
    pub kind: KitPieceKind,
    pub name: String,
}

/// A server Kit holds, and what this Manifest says of it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Server {
    /// `stdio` or `http`.
    pub transport: &'static str,
    pub address: String,
    /// Kit's own default: whether a Drone gets it where no Manifest says otherwise.
    pub by_default: bool,
    /// Fleet's resolution, drawn as given.
    pub resolves: bool,
}

/// One piece of a kind, as the list and the detail draw it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    pub id: String,
    pub kind: Kind,
    pub name: String,
    pub says: Option<String>,
    pub mark: Mark,
    /// The file or folder it is kept in, absolute.
    pub path: Option<String>,
    /// Set where Remove takes this piece out: a skill of his own, or a plugin.
    pub piece: Option<Piece>,
    /// The plugin that brought it, as it is drawn, which is what removes it.
    pub comes_with: Option<String>,
    /// The same plugin as the inventory spells it, whole: what Fleet compares on every file act.
    pub plugin_id: Option<String>,
    /// The name the list of skills switched off holds, where Jobs can start without this skill.
    pub jobs: Option<String>,
    /// A server Kit holds: its reach is the project's switch.
    pub server: Option<Server>,
    /// An output style in force.
    pub in_force: Option<bool>,
    /// An allowed command Remove takes back, as the allowlist spells it.
    pub command: Option<String>,
    /// A plugin's marketplace, which its name is listed without.
    pub marketplace: Option<String>,
    /// What the file says, where Fleet has handed it over.
    pub file: Option<String>,
    /// Set on a row only on a linked Mac: that Mac's name. It has no file or act here.
    pub elsewhere: Option<String>,
}

/// What a kind came to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Read {
    Rows(Vec<Row>),
    NotRead(String),
}

/// A folder's own name: the last part of its path.
fn last_part(folder: &str) -> String {
    folder.trim_end_matches('/').rsplit('/').next().unwrap_or(folder).to_string()
}

/// The harness's name for a plugin, without its marketplace.
fn provider_of(plugin: &str) -> &str {
    plugin.split('@').next().unwrap_or(plugin)
}

/// The file or folder an item's `source` names: none where it is empty, Armada's, or the harness's own.
pub fn source_path(source: &str, home: &str) -> Option<String> {
    if source.is_empty() || source == "Armada" || source.starts_with("built in") {
        return None;
    }
    Some(if source.starts_with('/') { source.to_string() } else { format!("{}/{source}", home.trim_end_matches('/')) })
}

/// Where an allowed command came from, in words.
fn source_words(source: &str) -> String {
    if source.starts_with("retro item") {
        "Retro item".into()
    } else if source == "always allow" {
        "Always allow".into()
    } else if source == "written by hand" {
        "Written by hand".into()
    } else {
        source.to_string()
    }
}

fn mark_of(item: &ipc::SetupItem, home: &str) -> Mark {
    if let Some(plugin) = &item.plugin {
        return Mark::Plugin(provider_of(plugin).to_string());
    }
    if let Some(style) = &item.style {
        return match style.from.as_str() {
            "person" => Mark::Yours,
            "repository" => Mark::Repository,
            "plugin" => style.plugin.as_deref().map_or_else(|| Mark::Plain("A plugin".into()), |plugin| Mark::Plugin(provider_of(plugin).to_string())),
            "armada" => Mark::Armada,
            _ => Mark::Plain("Built in".into()),
        };
    }
    if item.source.is_empty() || item.source == home || item.source.starts_with(&format!("{home}/")) {
        Mark::Yours
    } else if item.source == "Armada" {
        Mark::Armada
    } else {
        Mark::Repository
    }
}

fn row_of(kind: Kind, item: &ipc::SetupItem, home: &str) -> Row {
    let path = source_path(&item.source, home);
    let mut row = Row {
        id: format!("{}:{}:{}", kind.id(), item.source, item.name),
        kind,
        name: item.name.clone(),
        says: item.says.clone(),
        mark: mark_of(item, home),
        path,
        piece: None,
        comes_with: None,
        plugin_id: item.plugin.clone().or_else(|| item.style.as_ref().and_then(|style| style.plugin.clone())),
        jobs: None,
        server: None,
        in_force: None,
        command: None,
        marketplace: None,
        file: None,
        elsewhere: None,
    };
    match kind {
        Kind::Plugins => {
            let (plain, marketplace) = match item.name.split_once('@') {
                Some((plain, marketplace)) => (plain.to_string(), Some(marketplace.to_string())),
                None => (item.name.clone(), None),
            };
            row.name = plain;
            row.mark = marketplace.clone().map_or(Mark::Yours, Mark::Market);
            row.marketplace = marketplace;
            row.piece = Some(Piece { kind: KitPieceKind::Plugin, name: item.name.clone() });
        }
        Kind::Skills => match &item.plugin {
            Some(plugin) => {
                row.comes_with = Some(provider_of(plugin).to_string());
                if provider_of(plugin) != "armada" {
                    row.jobs = Some(format!("{}:{}", provider_of(plugin), item.name));
                }
            }
            None => {
                if row.path.is_some() && row.mark == Mark::Yours {
                    row.piece = Some(Piece { kind: KitPieceKind::Skill, name: item.name.clone() });
                }
                if matches!(row.mark, Mark::Yours) {
                    row.jobs = Some(item.name.clone());
                }
            }
        },
        Kind::SubAgents | Kind::Commands | Kind::OutputStyles => row.comes_with = row.plugin_id.as_deref().map(|plugin| provider_of(plugin).to_string()),
        Kind::Servers => {
            // Where it is configured: the person's own file, a plugin's, a repository's `.mcp.json`, or a project's local scope.
            row.path = item.server.as_ref().map(|server| server.folder.clone());
            row.comes_with = row.plugin_id.as_deref().map(|plugin| provider_of(plugin).to_string());
            row.mark = match (&item.plugin, &item.server) {
                (Some(plugin), _) => Mark::Plugin(provider_of(plugin).to_string()),
                (None, Some(server)) if server.from == "repository" => Mark::Repository,
                (None, Some(server)) => Mark::Folder(last_part(&server.folder)),
                (None, None) => Mark::Yours,
            };
        }
        Kind::Allowed => {
            row.mark = Mark::Plain(source_words(&item.source));
            row.path = None;
            row.command = Some(item.name.clone());
        }
        Kind::Instructions => {}
    }
    if kind == Kind::OutputStyles {
        row.in_force = item.style.as_ref().map(|style| style.applied == "active" || style.applied == "always");
    }
    row
}

/// A server's address as a person reads it.
fn address_words(address: &ipc::ServerAddress) -> String {
    match address {
        ipc::ServerAddress::Stdio { command, args } => std::iter::once(command.as_str()).chain(args.iter().map(String::as_str)).collect::<Vec<_>>().join(" "),
        ipc::ServerAddress::Http { url } => url.clone(),
    }
}

/// One kind's pieces off the read. MCP servers are the ones Kit holds first, then those connected outside Armada.
pub fn read_kind(kind: Kind, inventory: &KitInventory, servers: &[KitServerRow]) -> Read {
    let found = inventory.kinds.iter().find(|row| row.kind == kind.word());
    let mut rows: Vec<Row> = Vec::new();
    match found.map(|row| &row.read) {
        Some(WhatWasRead::NotRead { why }) if kind != Kind::Servers => return Read::NotRead(why.clone()),
        Some(WhatWasRead::Read { items, .. }) => rows.extend(items.iter().map(|item| row_of(kind, item, &inventory.home))),
        _ => {}
    }
    if kind == Kind::Servers {
        let held = servers.iter().map(|server| Row {
            id: format!("servers:kit:{}", server.name),
            kind,
            name: server.name.clone(),
            says: None,
            mark: Mark::Kit,
            path: None,
            piece: None,
            comes_with: None,
            plugin_id: None,
            jobs: None,
            server: Some(Server {
                transport: match server.address {
                    ipc::ServerAddress::Stdio { .. } => "stdio",
                    ipc::ServerAddress::Http { .. } => "http",
                },
                address: address_words(&server.address),
                by_default: server.drones.as_wire() == "yes",
                resolves: server.resolves,
            }),
            in_force: None,
            command: None,
            marketplace: None,
            file: None,
            elsewhere: None,
        });
        rows.splice(0..0, held);
        // Where several repositories each list servers, each is grouped under its own folder's name.
        let mut roots: Vec<&String> = rows.iter().filter(|row| row.mark == Mark::Repository).filter_map(|row| row.path.as_ref()).collect();
        roots.sort();
        roots.dedup();
        if roots.len() > 1 {
            for row in rows.iter_mut().filter(|row| row.mark == Mark::Repository) {
                if let Some(path) = &row.path {
                    row.mark = Mark::Folder(last_part(path));
                }
            }
        }
    }
    Read::Rows(rows)
}

/// Whether `words` holds every word of `query`, ignoring case.
pub fn holds(words: &str, query: &str) -> bool {
    let said = words.to_lowercase();
    query.to_lowercase().split_whitespace().all(|one| said.contains(one))
}

/// A piece taken out and not yet gone from the list: amber, struck through, with Restore until he leaves Kit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Gone {
    pub row: Row,
    /// Where a skill's folder went in the Trash. None for a plugin, which is uninstalled.
    pub trashed: Option<String>,
}

/// One piece as the list holds it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Shown {
    pub row: Row,
    pub gone: Option<Option<String>>,
}

/// A line of the list: a group's chip, or a piece by its place in [`Listed::shown`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Line {
    Head(Mark),
    Row(usize),
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Listed {
    pub shown: Vec<Shown>,
    pub lines: Vec<Line>,
}

/// The pieces of a kind that the filter keeps, grouped by where they come from. A removed piece stays in its
/// group, last, until he leaves.
pub fn listed(rows: &[Row], gone: &[Gone], query: &str) -> Listed {
    let mut all: Vec<Shown> = rows.iter().cloned().map(|row| Shown { row, gone: None }).collect();
    all.extend(gone.iter().filter(|one| !rows.iter().any(|row| row.id == one.row.id)).map(|one| Shown { row: one.row.clone(), gone: Some(one.trashed.clone()) }));
    all.retain(|one| holds(&format!("{} {} {}", one.row.name, one.row.says.as_deref().unwrap_or_default(), one.row.mark.label()), query));
    // A stable sort keeps the order Fleet read them in within a group, and a removed piece after the rest.
    all.sort_by_cached_key(|one| (one.row.mark.order(), one.gone.is_some()));
    let mut lines = Vec::new();
    let mut last: Option<&Mark> = None;
    for (at, one) in all.iter().enumerate() {
        if last != Some(&one.row.mark) {
            lines.push(Line::Head(one.row.mark.clone()));
            last = Some(&one.row.mark);
        }
        lines.push(Line::Row(at));
    }
    Listed { shown: all, lines }
}

/// The skills each repository's Jobs start without, as the setting holds them.
pub fn parse_off(value: Option<&SettingValue>) -> BTreeMap<String, Vec<String>> {
    let Some(SettingValue::Map(map)) = value else { return BTreeMap::new() };
    map.iter()
        .map(|(id, names)| {
            let names = match names {
                SettingValue::List(names) => names.iter().filter_map(|name| if let SettingValue::Text(text) = name { Some(text.clone()) } else { None }).collect(),
                _ => Vec::new(),
            };
            (id.clone(), names)
        })
        .collect()
}

/// The skills each repository's Jobs start without, as settings.json holds them.
pub fn skills_off(list: Option<&SettingsList>) -> BTreeMap<String, Vec<String>> {
    parse_off(list.and_then(|list| list.settings.iter().find(|one| one.key == SKILLS_OFF)).map(|one| &one.value))
}

/// Whether `skill` is on in every one of `repositories`.
pub fn on_in(off: &BTreeMap<String, Vec<String>>, repositories: &[String], skill: &str) -> bool {
    repositories.iter().all(|id| !off.get(id).is_some_and(|names| names.iter().any(|name| name == skill)))
}

/// What to save to switch `skill` on or off in each of `repositories`: the whole map, or none where nothing is
/// left off (`null` takes the key away).
pub fn switched(off: &BTreeMap<String, Vec<String>>, repositories: &[String], skill: &str, on: bool) -> BTreeMap<String, Option<SettingValue>> {
    let mut next = off.clone();
    for id in repositories {
        let mut names: Vec<String> = next.remove(id).unwrap_or_default();
        names.retain(|name| name != skill);
        if !on {
            names.push(skill.to_string());
        }
        if !names.is_empty() {
            next.insert(id.clone(), names);
        }
    }
    let value = (!next.is_empty()).then(|| {
        SettingValue::Map(next.into_iter().map(|(id, names)| (id, SettingValue::List(names.into_iter().map(SettingValue::Text).collect()))).collect())
    });
    BTreeMap::from([(SKILLS_OFF.to_string(), value)])
}

/// The project Kit is scoped to: its name and the Manifest ids of the repositories it draws on.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Scope {
    pub name: String,
    pub repositories: Vec<String>,
}

impl Row {
    /// Whether the project's switch can move for this piece.
    pub fn switchable(&self) -> bool {
        self.jobs.is_some() || self.server.as_ref().is_some()
    }

    /// Whether it is on in the scoped project, for the lamp and the switch. None where the kind has no lamp.
    pub fn lamp(&self, off: &BTreeMap<String, Vec<String>>, scope: &Scope) -> Option<bool> {
        // Only on a linked Mac: nothing here to be on or off.
        if self.elsewhere.is_some() {
            return None;
        }
        match (self.kind, &self.jobs, &self.server) {
            (Kind::Skills, Some(name), _) => Some(on_in(off, &scope.repositories, name)),
            // A skill a Jobs starts without ever being asked: the repository's own and Armada's.
            (Kind::Skills, None, _) => Some(true),
            (Kind::Servers, _, Some(server)) => Some(server.resolves),
            // Connected outside Armada: it reaches no Drone.
            (Kind::Servers, _, None) => Some(false),
            (Kind::OutputStyles, ..) => self.in_force,
            _ => None,
        }
    }

    pub fn removal(&self) -> Removal {
        if let Some(piece) = &self.piece {
            Removal::Piece(piece.clone())
        } else if let Some(plugin) = &self.comes_with {
            Removal::Locked(plugin.clone())
        } else if let Some(command) = &self.command {
            Removal::Command(command.clone())
        } else if self.server.is_some() {
            Removal::Server(self.name.clone())
        } else {
            Removal::None
        }
    }
}

/// What the Remove place holds for a piece.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Removal {
    Piece(Piece),
    /// It comes with this plugin, and the plugin is what goes.
    Locked(String),
    /// An allowed command, taken back from the allowlist.
    Command(String),
    /// A server Kit holds, forgotten.
    Server(String),
    None,
}

impl Row {
    /// The one file Edit and Open work on: a skill's `SKILL.md`, or the item's own file. None for a plugin, which is
    /// a folder of several.
    pub fn opens(&self) -> Option<String> {
        let path = self.path.as_ref()?;
        match self.kind {
            Kind::Plugins | Kind::Servers | Kind::Allowed => None,
            Kind::Skills => Some(format!("{path}/SKILL.md")),
            _ => Some(path.clone()),
        }
    }
}

/// A path as a person writes it: under their own folder, `~`.
pub fn tilded(path: &str, home: Option<&str>) -> String {
    match home.filter(|home| !home.is_empty()).and_then(|home| path.strip_prefix(home)).filter(|rest| rest.is_empty() || rest.starts_with('/')) {
        Some(rest) => format!("~{rest}"),
        None => path.to_string(),
    }
}
