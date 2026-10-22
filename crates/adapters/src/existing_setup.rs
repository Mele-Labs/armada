//! What the agent CLI's own user configuration holds, read to be shown.
//!
//! **This is the only file in the workspace that knows where that CLI keeps a
//! person's skills, plugins, sub agents, commands, global agent file and
//! connected servers** — [`harness`](mod@crate::harness) is the same guarantee
//! for how it spells an argument, and this is it for how it spells a home.
//! Nothing above this crate learns either.

//! **It reads and it cannot grant.** A server is carried as its name, the
//! program's own file name, or the host it is at — enough to tell two apart
//! and to see one pointing somewhere wrong. What comes after any of those is
//! where a key sits, and each has a type here that keeps nothing: [`Program`],
//! [`Origin`] and two [`IgnoredAny`] fields. So what survives could not start
//! the server it names, and this adapter holds no credential rather than being
//! careful not to draw one. Grep `IgnoredAny` and `Kept` here; grep this
//! module's name in [`mcp`](mod@crate::mcp) — where a Drone's server document
//! is written — and find nothing.

//! **It reads and it cannot write.** [`SetupFiles`] has no write method, so
//! nothing here can repair a file it fails to parse. A file that will not read
//! is named in the answer and left exactly as it is.

use std::collections::BTreeMap;
use std::fmt;

use adapter_traits::{
    FileRead, HarnessSetup, Inventory, ItemAsked, ItemRead, KindRead, SetupFiles, SetupItem, SetupKind,
    StyleApplied, StyleFacts, StyleFrom, Unreadable, WhatWasRead,
};
use serde::de::IgnoredAny;
use serde::Deserialize;

/// The filesystem half: a home directory on this machine, read and never
/// written. Apart from this file because what it knows is macOS, where
/// everything above knows one harness.
mod home;
/// One Loadout row's file, read whole: `read_loadout_item`.
mod item;

pub use home::Home;

/// The harness, in its own name. Crosses the wire as data, so a surface draws
/// it rather than holding it.
const HARNESS: &str = "Claude Code";

/// Where it keeps everything but the servers, under a person's home.
const HOME: &str = ".claude";
/// The servers a person connected. **Beside the folder, not inside it**, which
/// is why [`SetupFiles`] is rooted at the home rather than at `HOME`.
const CONNECTED: &str = ".claude.json";
const SKILLS: &str = ".claude/skills";
const SKILL_FILE: &str = "SKILL.md";
const SUB_AGENTS: &str = ".claude/agents";
const COMMANDS: &str = ".claude/commands";
const AGENT_FILE: &str = ".claude/CLAUDE.md";
const PLUGINS: &str = ".claude/plugins/installed_plugins.json";
const OUTPUT_STYLES: &str = ".claude/output-styles";
/// A plugin's own folder of styles, under where it is installed.
const PLUGIN_STYLES: &str = "output-styles";
/// A plugin's own skills, sub agents and commands, under where it is installed.
const PLUGIN_SKILLS: &str = "skills";
const PLUGIN_AGENTS: &str = "agents";
const PLUGIN_COMMANDS: &str = "commands";
/// The settings files that can name the active style, **user first**. A
/// repository has the other two under its own `.claude`, and the order of
/// precedence is the harness's own: local over project over user.
const SETTINGS: &str = ".claude/settings.json";
const LOCAL_SETTINGS: &str = ".claude/settings.local.json";

/// The styles the harness carries with no file behind them, and the line each
/// says for itself.
const BUILT_IN_STYLES: [(&str, &str); 3] = [
    (
        "default",
        "Completes coding tasks efficiently and answers concisely.",
    ),
    (
        "Explanatory",
        "Explains its implementation choices and the patterns it finds in the codebase.",
    ),
    (
        "Learning",
        "Pauses and asks the person to write small pieces of the code themselves.",
    ),
];

/// Opens one repository's files, rooted at it. Supplied by whoever builds the
/// reader, because where a checkout is is no business of this module.
type Opener = Box<dyn Fn(&str) -> Box<dyn SetupFiles + Send + Sync> + Send + Sync>;

/// How far under `skills/` a skill may sit. One level is a skill a person
/// wrote. **Three is measured, not guessed**: a synced set lands under a folder
/// and then under the id of the set it came from, and at two the owner's eight
/// synced skills read as nothing at all.
const SKILL_DEPTH: usize = 3;
/// The same, for commands: a namespaced command is a file in a subfolder.
const COMMAND_DEPTH: usize = 2;

/// How much of an item's own description is carried. Enough for a row, and
/// short of carrying the file.
const SAYS: usize = 240;

/// Reads one person's setup. `F` is where the files come from, so the parsing
/// below is exercised against a planted tree rather than against a home
/// directory a test would have to own.
pub struct ExistingSetup<F> {
    files: F,
    /// As a person would type it, for what the answer says about where each
    /// item came from.
    home: String,
    /// How a repository is opened, where the output styles of repositories are
    /// to be read. `None` reads the home alone.
    repositories: Option<Opener>,
}

impl<F: SetupFiles> ExistingSetup<F> {
    /// `home` is the person's own home directory, absolute, and is what every
    /// `source` in the answer is written under.
    pub fn over(files: F, home: &str) -> ExistingSetup<F> {
        ExistingSetup {
            files,
            home: home.trim_end_matches('/').to_string(),
            repositories: None,
        }
    }

    /// Read each repository's own output styles and settings too, through
    /// `open`, which roots the files it returns at the repository.
    pub fn in_repositories(
        mut self,
        open: impl Fn(&str) -> Box<dyn SetupFiles + Send + Sync> + Send + Sync + 'static,
    ) -> ExistingSetup<F> {
        self.repositories = Some(Box::new(open));
        self
    }

    /// A path under the home, as a person reads it.
    fn at(&self, path: &str) -> String {
        format!("{}/{path}", self.home)
    }

    /// Each installed plugin that sits under the home, as its name without the
    /// marketplace and the folder it is installed in, relative to the home.
    fn plugin_folders(&self) -> Vec<(String, String)> {
        let FileRead::Bytes(bytes) = self.files.read(PLUGINS) else {
            return Vec::new();
        };
        let Ok(installed) = ipc::decode::<Installed>("installed plugins", &bytes) else {
            return Vec::new();
        };
        let under = format!("{}/", self.home);
        installed
            .plugins
            .into_iter()
            .filter_map(|(name, versions)| {
                let path = versions.first().and_then(|held| held.install_path.as_ref())?;
                // Only a plugin installed under the home is reachable here.
                let relative = path.strip_prefix(&under)?.to_string();
                let plugin = name.split('@').next().unwrap_or(&name).to_string();
                Some((plugin, relative))
            })
            .collect()
    }

    /// The person's skills, then each installed plugin's.
    fn skills(&self) -> WhatWasRead {
        let (mut items, mut unreadable) = (Vec::new(), Vec::new());
        self.skills_under(SKILLS, SKILL_DEPTH, None, &mut items, &mut unreadable);
        for (plugin, folder) in self.plugin_folders() {
            let dir = format!("{folder}/{PLUGIN_SKILLS}");
            self.skills_under(&dir, SKILL_DEPTH, Some(&plugin), &mut items, &mut unreadable);
        }
        WhatWasRead::Read { items, unreadable }
    }

    /// Every directory holding a `SKILL.md`, and a directory that holds none
    /// looked into once more rather than reported.
    fn skills_under(
        &self,
        dir: &str,
        depth: usize,
        plugin: Option<&str>,
        items: &mut Vec<SetupItem>,
        unreadable: &mut Vec<Unreadable>,
    ) {
        let Ok(entries) = self.files.entries(dir) else {
            return;
        };
        // A hidden folder is a tool's own keeping, `.trash` among them, and not a skill in use.
        for entry in entries.iter().filter(|entry| entry.is_dir && !entry.name.starts_with('.')) {
            let here = format!("{dir}/{}", entry.name);
            let file = format!("{here}/{SKILL_FILE}");
            match self.files.read(&file) {
                FileRead::Bytes(bytes) => match front_matter(&bytes) {
                    Ok(front) => items.push(SetupItem {
                        name: front.name.unwrap_or_else(|| entry.name.clone()),
                        says: front.description.map(|said| shortened(&said)),
                        source: self.at(&here),
                        style: None,
                        plugin: plugin.map(str::to_string),
                        uninstall: None,
                    }),
                    Err(why) => unreadable.push(Unreadable {
                        source: self.at(&file),
                        why,
                    }),
                },
                FileRead::Unreadable(why) => unreadable.push(Unreadable {
                    source: self.at(&file),
                    why,
                }),
                FileRead::Absent if depth > 1 => {
                    self.skills_under(&here, depth - 1, plugin, items, unreadable)
                }
                FileRead::Absent => {}
            }
        }
    }

    /// Sub agents and commands: a markdown file, named by its own front matter
    /// where it carries one and by its path where it does not. The person's
    /// own, then each installed plugin's from `plugin_dir` in its folder.
    fn markdown_in(&self, dir: &str, plugin_dir: &str, depth: usize) -> WhatWasRead {
        let (mut items, mut unreadable) = (Vec::new(), Vec::new());
        self.markdown_under(dir, dir, depth, None, &mut items, &mut unreadable);
        for (plugin, folder) in self.plugin_folders() {
            let dir = format!("{folder}/{plugin_dir}");
            self.markdown_under(&dir, &dir, depth, Some(&plugin), &mut items, &mut unreadable);
        }
        WhatWasRead::Read { items, unreadable }
    }

    fn markdown_under(
        &self,
        root: &str,
        dir: &str,
        depth: usize,
        plugin: Option<&str>,
        items: &mut Vec<SetupItem>,
        unreadable: &mut Vec<Unreadable>,
    ) {
        let Ok(entries) = self.files.entries(dir) else {
            return;
        };
        for entry in entries {
            let here = format!("{dir}/{}", entry.name);
            if entry.is_dir {
                if depth > 1 {
                    self.markdown_under(root, &here, depth - 1, plugin, items, unreadable);
                }
                continue;
            }
            let Some(stem) = entry.name.strip_suffix(".md") else {
                continue;
            };
            let named = here
                .strip_prefix(&format!("{root}/"))
                .map(|path| path.trim_end_matches(".md").replace('/', ":"))
                .unwrap_or_else(|| stem.to_string());
            match self.files.read(&here) {
                FileRead::Bytes(bytes) => match front_matter(&bytes) {
                    Ok(front) => items.push(SetupItem {
                        name: front.name.unwrap_or(named),
                        says: front.description.map(|said| shortened(&said)),
                        source: self.at(&here),
                        style: None,
                        plugin: plugin.map(str::to_string),
                        uninstall: None,
                    }),
                    Err(why) => unreadable.push(Unreadable {
                        source: self.at(&here),
                        why,
                    }),
                },
                FileRead::Unreadable(why) => unreadable.push(Unreadable {
                    source: self.at(&here),
                    why,
                }),
                FileRead::Absent => {}
            }
        }
    }

    /// The global "how I work" file: that it is there, how long it is, and its
    /// first line. **Never its contents** — it is a person's own writing and a
    /// row is not where they would read it.
    fn agent_file(&self) -> WhatWasRead {
        let (mut items, mut unreadable) = (Vec::new(), Vec::new());
        match self.files.read(AGENT_FILE) {
            FileRead::Bytes(bytes) => {
                let text = String::from_utf8_lossy(&bytes);
                let lines = text.lines().count();
                let first = text.lines().find(|line| !line.trim().is_empty());
                items.push(SetupItem {
                    name: AGENT_FILE.rsplit('/').next().unwrap_or(AGENT_FILE).into(),
                    says: Some(match first {
                        Some(line) => format!("{lines} lines, opening {}", shortened(line)),
                        None => format!("{lines} lines"),
                    }),
                    source: self.at(AGENT_FILE),
                    style: None,
                    plugin: None,
                    uninstall: None,
                });
            }
            FileRead::Unreadable(why) => unreadable.push(Unreadable {
                source: self.at(AGENT_FILE),
                why,
            }),
            FileRead::Absent => {}
        }
        WhatWasRead::Read { items, unreadable }
    }

    /// Installed plugins, off the file the CLI keeps them in. A plugin carries
    /// its marketplace in its own name, so the name is left as written.
    fn plugins(&self) -> WhatWasRead {
        let (mut items, mut unreadable) = (Vec::new(), Vec::new());
        match self.files.read(PLUGINS) {
            FileRead::Bytes(bytes) => match ipc::decode::<Installed>("installed plugins", &bytes) {
                Ok(installed) => {
                    for (name, versions) in installed.plugins {
                        let first = versions.first();
                        items.push(SetupItem {
                            says: first
                                .and_then(|held| held.version.as_ref())
                                .map(|version| format!("version {version}")),
                            source: first
                                .and_then(|held| held.install_path.clone())
                                .unwrap_or_else(|| self.at(PLUGINS)),
                            style: None,
                            plugin: None,
                            uninstall: Some(crate::kit_pieces::uninstall_line(&name).shown()),
                            name,
                        });
                    }
                }
                Err(why) => unreadable.push(Unreadable {
                    source: self.at(PLUGINS),
                    why: why.to_string(),
                }),
            },
            FileRead::Unreadable(why) => unreadable.push(Unreadable {
                source: self.at(PLUGINS),
                why,
            }),
            FileRead::Absent => {}
        }
        WhatWasRead::Read { items, unreadable }
    }

    /// Output styles: the person's, each repository's and each installed
    /// plugin's, and which is in force. **Read only, and never a body**: a
    /// style is a person's own writing and a row is not where it is read.
    fn output_styles(&self, repositories: &[String]) -> WhatWasRead {
        let mut found: Vec<Found> = Vec::new();
        let mut unreadable = Vec::new();
        let at = |path: &str| self.at(path);
        let mut chosen: Vec<String> = Vec::new();

        let user = setting(&self.files, SETTINGS, &at, &mut unreadable);
        chosen.extend(user.clone());
        styles_in(
            &self.files,
            OUTPUT_STYLES,
            &at,
            StyleFrom::Person,
            None,
            &mut found,
            &mut unreadable,
        );

        if let Some(open) = &self.repositories {
            for root in repositories {
                let files = open(root);
                let root = root.trim_end_matches('/');
                let there = |path: &str| format!("{root}/{path}");
                let project = setting(&*files, SETTINGS, &there, &mut unreadable);
                let local = setting(&*files, LOCAL_SETTINGS, &there, &mut unreadable);
                chosen.extend(local.or(project).or_else(|| user.clone()));
                styles_in(
                    &*files,
                    OUTPUT_STYLES,
                    &there,
                    StyleFrom::Repository,
                    None,
                    &mut found,
                    &mut unreadable,
                );
            }
        }

        for (plugin, folder) in self.plugin_folders() {
            styles_in(
                &self.files,
                &format!("{folder}/{PLUGIN_STYLES}"),
                &at,
                StyleFrom::Plugin,
                Some(&plugin),
                &mut found,
                &mut unreadable,
            );
        }

        let mut items: Vec<SetupItem> = found
            .iter()
            .map(|style| {
                let on = chosen.iter().any(|pick| style.is_named(pick));
                SetupItem {
                    name: style.name.clone(),
                    says: style.says.clone(),
                    source: style.source.clone(),
                    style: Some(StyleFacts {
                        from: style.from,
                        applied: if on {
                            StyleApplied::Active
                        } else {
                            StyleApplied::Available
                        },
                        plugin: style.plugin.clone(),
                    }),
                    plugin: None,
                    uninstall: None,
                }
            })
            .collect();
        for (name, says) in BUILT_IN_STYLES {
            let named = chosen.iter().any(|pick| pick == name);
            if named && !found.iter().any(|style| style.name == name) {
                items.push(SetupItem {
                    name: name.to_string(),
                    says: Some(says.to_string()),
                    source: format!("built in to {HARNESS}"),
                    style: Some(StyleFacts {
                        from: StyleFrom::BuiltIn,
                        applied: StyleApplied::Active,
                        plugin: None,
                    }),
                    plugin: None,
                    uninstall: None,
                });
            }
        }
        WhatWasRead::Read { items, unreadable }
    }

    /// The servers a person connected outside Armada, by name and by the
    /// program or host they are at. See this module's header for what does not
    /// survive the read, and [`Connected`] for how.
    fn connected(&self) -> WhatWasRead {
        let (mut items, mut unreadable) = (Vec::new(), Vec::new());
        match self.files.read(CONNECTED) {
            FileRead::Bytes(bytes) => {
                match ipc::decode::<UserConfig>("connected servers", &bytes) {
                    Ok(config) => {
                        for (name, server) in config.mcp_servers {
                            // A server named for the program it runs says it
                            // once: `gitnexus gitnexus` is a row that reads as
                            // a rendering fault, measured on the owner's own.
                            let shown = server.shown();
                            items.push(SetupItem {
                                says: (shown != name).then_some(shown),
                                name,
                                source: self.at(CONNECTED),
                                style: None,
                                plugin: None,
                                uninstall: None,
                            });
                        }
                    }
                    Err(why) => unreadable.push(Unreadable {
                        source: self.at(CONNECTED),
                        why: why.to_string(),
                    }),
                }
            }
            FileRead::Unreadable(why) => unreadable.push(Unreadable {
                source: self.at(CONNECTED),
                why,
            }),
            FileRead::Absent => {}
        }
        WhatWasRead::Read { items, unreadable }
    }
}

impl<F: SetupFiles> HarnessSetup for ExistingSetup<F> {
    fn read(&self) -> Inventory {
        self.read_across(&[])
    }

    fn read_across(&self, repositories: &[String]) -> Inventory {
        let present = self.files.entries(HOME).is_ok();
        let kinds = SetupKind::ALL
            .iter()
            .map(|kind| KindRead {
                kind: *kind,
                what: match kind {
                    SetupKind::Skills => self.skills(),
                    SetupKind::Plugins => self.plugins(),
                    SetupKind::AgentFile => self.agent_file(),
                    SetupKind::OutputStyles => self.output_styles(repositories),
                    SetupKind::SubAgents => self.markdown_in(SUB_AGENTS, PLUGIN_AGENTS, 1),
                    SetupKind::Commands => self.markdown_in(COMMANDS, PLUGIN_COMMANDS, COMMAND_DEPTH),
                    SetupKind::McpServers => self.connected(),
                    SetupKind::Allowlist => WhatWasRead::NotRead {
                        why: NO_ALLOWLIST.to_string(),
                    },
                    SetupKind::Models => WhatWasRead::NotRead {
                        why: NO_MODELS.to_string(),
                    },
                },
            })
            .collect();
        Inventory {
            harness: HARNESS.to_string(),
            home: self.at(HOME),
            present,
            kinds,
        }
    }

    /// **Off the disk under the home, not through `F`**: whether a file stays inside its folder is
    /// a question about links, which only the disk answers.
    fn read_item(&self, asked: &ItemAsked<'_>) -> ItemRead {
        item::read_item(&self.home, asked)
    }
}

/// **Half-reading an allowlist is worse than not reading one**: a rule drawn
/// out of its two tiers reads as a grant, and which tier a row belongs to is
/// `#41`'s question.
const NO_ALLOWLIST: &str =
    "a rule drawn out of its two tiers reads as a grant, and both tiers are #41";

/// Which models a Drone may use is resolved by `config` against a Manifest, and
/// a second list read off a harness would be a second answer to that.
const NO_MODELS: &str =
    "which models a Job may use is resolved against a Manifest, and that is #41";

/// The front matter a skill, a sub agent or a command carries.
#[derive(Deserialize)]
pub(crate) struct Front {
    pub(crate) name: Option<String>,
    pub(crate) description: Option<String>,
}

/// The document between the first two `---` lines, as YAML. `Ok(None)` fields
/// where there is no front matter at all, which is a file a person wrote
/// without one rather than a file that will not read.
pub(crate) fn front_matter(bytes: &[u8]) -> Result<Front, String> {
    let text = String::from_utf8_lossy(bytes);
    let rest = match text.strip_prefix("---\n") {
        Some(rest) => rest,
        None => {
            return Ok(Front {
                name: None,
                description: None,
            })
        }
    };
    let Some(end) = rest.find("\n---") else {
        return Err("front matter opens and never closes".to_string());
    };
    serde_yaml_ng::from_str::<Front>(&rest[..end]).map_err(|why| why.to_string())
}

/// One output style found in a folder, before it is known whether it is chosen.
struct Found {
    name: String,
    says: Option<String>,
    source: String,
    from: StyleFrom,
    /// `plugin:name`, which is how a setting names a plugin's style.
    qualified: Option<String>,
    /// The plugin that ships it, without its marketplace.
    plugin: Option<String>,
}

impl Found {
    fn is_named(&self, pick: &str) -> bool {
        self.name == pick || self.qualified.as_deref() == Some(pick)
    }
}

/// What a settings file says about the style, and nothing else it holds: the
/// file also carries permissions and environment, none of which is read.
#[derive(Deserialize)]
struct Settings {
    #[serde(default, rename = "outputStyle")]
    output_style: Option<String>,
}

/// The `outputStyle` one settings file names. A file that is there and will
/// not read is named, and it is no pick.
fn setting(
    files: &dyn SetupFiles,
    path: &str,
    shown: &dyn Fn(&str) -> String,
    unreadable: &mut Vec<Unreadable>,
) -> Option<String> {
    match files.read(path) {
        FileRead::Bytes(bytes) => match ipc::decode::<Settings>("settings", &bytes) {
            Ok(settings) => settings.output_style,
            Err(why) => {
                unreadable.push(Unreadable {
                    source: shown(path),
                    why: why.to_string(),
                });
                None
            }
        },
        FileRead::Unreadable(why) => {
            unreadable.push(Unreadable {
                source: shown(path),
                why,
            });
            None
        }
        FileRead::Absent => None,
    }
}

/// Every `.md` directly in `dir`, as a style. The body is never kept.
fn styles_in(
    files: &dyn SetupFiles,
    dir: &str,
    shown: &dyn Fn(&str) -> String,
    from: StyleFrom,
    plugin: Option<&str>,
    found: &mut Vec<Found>,
    unreadable: &mut Vec<Unreadable>,
) {
    let Ok(entries) = files.entries(dir) else {
        return;
    };
    for entry in entries.iter().filter(|entry| !entry.is_dir) {
        let Some(stem) = entry.name.strip_suffix(".md") else {
            continue;
        };
        let path = format!("{dir}/{}", entry.name);
        match files.read(&path) {
            FileRead::Bytes(bytes) => match front_matter(&bytes) {
                Ok(front) => {
                    let name = front.name.unwrap_or_else(|| stem.to_string());
                    found.push(Found {
                        qualified: plugin.map(|plugin| format!("{plugin}:{name}")),
                        plugin: plugin.map(str::to_string),
                        name,
                        says: front.description.map(|said| shortened(&said)),
                        source: shown(&path),
                        from,
                    });
                }
                Err(why) => unreadable.push(Unreadable {
                    source: shown(&path),
                    why,
                }),
            },
            FileRead::Unreadable(why) => unreadable.push(Unreadable {
                source: shown(&path),
                why,
            }),
            FileRead::Absent => {}
        }
    }
}

/// One paragraph of an item's own description, on one line.
fn shortened(said: &str) -> String {
    let one_line = said.split_whitespace().collect::<Vec<_>>().join(" ");
    match one_line.char_indices().nth(SAYS) {
        Some((cut, _)) => format!("{}…", &one_line[..cut]),
        None => one_line,
    }
}

/// The CLI's own record of what a person installed.
#[derive(Deserialize)]
struct Installed {
    #[serde(default)]
    plugins: BTreeMap<String, Vec<Held>>,
}

#[derive(Deserialize)]
struct Held {
    version: Option<String>,
    #[serde(rename = "installPath")]
    install_path: Option<String>,
}

/// The CLI's own user file. **Every key but one is dropped**: it also holds the
/// account a person is signed in as, and an inventory has no use for it.
#[derive(Deserialize)]
struct UserConfig {
    #[serde(default, rename = "mcpServers")]
    mcp_servers: BTreeMap<String, Connected>,
}

/// One connected server: **the program's own name, or the host it is at, and
/// nothing after either**.
///
/// A person has to be able to tell two servers apart and to spot one pointing
/// somewhere wrong — the owner's decision, 18 Sep. What carries a key is what
/// comes after: the argument list, the query string, the userinfo, the
/// environment. Each of those has a type here that keeps nothing, so this
/// adapter cannot hold one rather than being careful not to draw one.
#[derive(Deserialize)]
struct Connected {
    command: Option<Program>,
    url: Option<Origin>,
    /// **Named in order to be dropped.** Serde would skip an undeclared key
    /// anyway; a field whose type keeps nothing is the refusal said out loud,
    /// where the next person looks. `-e API_KEY=…` is an argument like any
    /// other, which is why the two below are one decision.
    #[serde(default, rename = "args")]
    _args: Option<IgnoredAny>,
    #[serde(default, rename = "env")]
    _env: Option<IgnoredAny>,
}

impl Connected {
    /// What the row says: the program, or the host. Never both — a server has
    /// one or the other, and a file with neither says only that it is there.
    fn shown(&self) -> String {
        match (&self.command, &self.url) {
            (Some(program), _) => program.0.clone(),
            (_, Some(origin)) => origin.0.clone(),
            _ => String::from("connected outside Armada"),
        }
    }
}

/// A program, kept as **the file it is** and never as the path it sits at.
///
/// A path can be a secret on its own — a checkout under a client's name, a
/// directory named for a token — and the file name is what tells two servers
/// apart. The visitor keeps the last segment and allocates nothing else, so
/// this type has nowhere to put the rest.
struct Program(String);

/// A host, kept as **the host alone**: no scheme, no userinfo, no port, no
/// path, no query, no fragment.
///
/// `https://user:key@host/mcp?token=…` is one string carrying three places a
/// key hides, and the host is the only one a person needs in order to see
/// where a server points.
struct Origin(String);

impl<'de> Deserialize<'de> for Program {
    fn deserialize<D: serde::Deserializer<'de>>(input: D) -> Result<Program, D::Error> {
        input.deserialize_str(Kept(|whole: &str| {
            let file = whole.rsplit('/').next().unwrap_or(whole);
            Program(String::from(file))
        }))
    }
}

impl<'de> Deserialize<'de> for Origin {
    fn deserialize<D: serde::Deserializer<'de>>(input: D) -> Result<Origin, D::Error> {
        input.deserialize_str(Kept(|whole: &str| Origin(String::from(host_of(whole)))))
    }
}

/// The host of a URL, or the whole of what was given where it is not one.
///
/// **Everything a `@`, a `:`, a `/`, a `?` or a `#` introduces is dropped**, in
/// that order, so userinfo goes with the port, the path, the query and the
/// fragment. A bracketed IPv6 literal keeps its brackets and loses its port.
fn host_of(url: &str) -> &str {
    let after_scheme = url.split_once("://").map(|(_, rest)| rest).unwrap_or(url);
    let authority = after_scheme
        .split(['/', '?', '#'])
        .next()
        .unwrap_or(after_scheme);
    let host_port = authority
        .rsplit_once('@')
        .map(|(_, host)| host)
        .unwrap_or(authority);
    match host_port.strip_prefix('[') {
        Some(inside) => match inside.split_once(']') {
            Some((literal, _)) => &host_port[..literal.len() + 2],
            None => host_port,
        },
        None => host_port.split(':').next().unwrap_or(host_port),
    }
}

/// A visitor that reads a string and keeps what `K` makes of it.
///
/// **The whole string is borrowed from the reader and never owned.** What is
/// allocated is what `K` returns, which is how [`Program`] and [`Origin`] come
/// to have nowhere to put a credential rather than a rule against holding one.
struct Kept<T, K: Fn(&str) -> T>(K);

impl<T, K: Fn(&str) -> T> serde::de::Visitor<'_> for Kept<T, K> {
    type Value = T;

    fn expecting(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(out, "a string")
    }

    fn visit_str<E: serde::de::Error>(self, whole: &str) -> Result<T, E> {
        Ok((self.0)(whole))
    }
}
