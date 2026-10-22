//! Kit's Add and Remove for a plugin or a skill: what a pasted source is, the commands that install
//! it, taking back a half-done install, and a skill's folder moved to the Trash and back.
//! `docs/practices/protocol/kit-add-and-remove-a-plugin-or-a-skill.md`.
//!
//! **This is the only file that knows how the agent CLI and the skills CLI spell an install**, and
//! where a GitHub repository keeps a marketplace or a skill. Fleet holds what [`look`] answers and
//! passes it back; it never builds a command.
//!
//! **A look runs nothing on the person's machine.** It reads the repository through `gh` and a
//! website through one GET. [`install`] takes an [`Asked`], as the mod's own install does.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use serde::Deserialize;

pub use crate::agent_mod::Asked;

/// Where a repository keeps its marketplace, which makes it a plugin source.
const MARKETPLACE: &str = ".claude-plugin/marketplace.json";
const SKILL_FILE: &str = "SKILL.md";
/// The skills CLI's name for this harness.
const SKILLS_AGENT: &str = "claude-code";
/// Under the person's home: their own skills, and the macOS Trash.
const SKILLS: &str = ".claude/skills";
const TRASH: &str = ".Trash";
/// The most of a website one look reads. An install line sits near the top of a page.
const MOST_PAGE: usize = 2 * 1024 * 1024;
/// GitHub paths that are not an owner, so a link to them names no repository.
const NOT_OWNERS: &[&str] = &[
    "about", "apps", "collections", "contact", "enterprise", "explore", "features", "login", "marketplace",
    "notifications", "orgs", "pricing", "security", "settings", "site", "sponsors", "topics",
];

/// One command, as it runs and as a person reads it. **Run without a shell**, so nothing in an
/// argument is read as syntax.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandLine {
    pub program: String,
    pub args: Vec<String>,
}

impl CommandLine {
    fn of(program: &str, args: &[&str]) -> CommandLine {
        CommandLine {
            program: program.to_string(),
            args: args.iter().map(|arg| arg.to_string()).collect(),
        }
    }

    /// As Add shows it.
    pub fn shown(&self) -> String {
        std::iter::once(self.program.as_str())
            .chain(self.args.iter().map(String::as_str))
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// Something Add can install.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Piece {
    /// A plugin through its marketplace. `skills` are the ones it brings.
    Plugin {
        name: String,
        marketplace: String,
        repository: String,
        skills: Vec<String>,
        commands: Vec<CommandLine>,
    },
    /// A skill on its own.
    Skill {
        name: String,
        repository: String,
        commands: Vec<CommandLine>,
    },
}

impl Piece {
    pub fn repository(&self) -> &str {
        match self {
            Piece::Plugin { repository, .. } | Piece::Skill { repository, .. } => repository,
        }
    }

    pub fn commands(&self) -> &[CommandLine] {
        match self {
            Piece::Plugin { commands, .. } | Piece::Skill { commands, .. } => commands,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Piece::Plugin { name, .. } | Piece::Skill { name, .. } => name,
        }
    }
}

/// One of several pieces a source holds, and its own words for itself where it gives any.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Choice {
    pub says: Option<String>,
    pub piece: Piece,
}

/// What a source was found to be.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Found {
    Piece(Piece),
    /// Several plugins in one marketplace, or several skills and no marketplace. Every choice is
    /// of one kind, and there are always at least two.
    Several { repository: String, choices: Vec<Choice> },
    /// Neither, in words a person reads.
    Unknown { why: String },
}

fn unknown(why: String) -> Found {
    Found::Unknown { why }
}

/// Where a look reads from. The real one is [`ForgeReader`]; a test plants a repository.
pub trait SourceReader {
    /// Every file in the repository's default branch, by path. `Err` says why it would not list.
    fn paths(&self, repository: &str) -> Result<Vec<String>, String>;
    /// One file's bytes.
    fn file(&self, repository: &str, path: &str) -> Result<Vec<u8>, String>;
    /// A web page's text.
    fn page(&self, url: &str) -> Result<String, String>;
}

/// Read a source: `owner/repository`, a GitHub address, or a website that points at one.
pub fn look(source: &str, reader: &impl SourceReader) -> Found {
    match source_of(source) {
        Ok(Source::Repository(repository)) => look_at_repository(&repository, reader),
        Ok(Source::Website(url)) => match reader.page(&url) {
            Ok(page) => match repository_on(&page) {
                Some(repository) => look_at_repository(&repository, reader),
                None => unknown(format!(
                    "{url} names no GitHub repository: no install line and no link to one was found on it."
                )),
            },
            Err(why) => unknown(format!("{url} could not be read: {why}")),
        },
        Err(why) => unknown(why),
    }
}

enum Source {
    Repository(String),
    Website(String),
}

fn source_of(said: &str) -> Result<Source, String> {
    let said = said.trim().trim_end_matches('/');
    let address = said
        .strip_prefix("https://")
        .or_else(|| said.strip_prefix("http://"))
        .map(|rest| (rest, true))
        .unwrap_or((said, false));
    let (rest, is_web) = address;
    let host = rest.split('/').next().unwrap_or("");
    if host == "github.com" || host == "www.github.com" {
        let mut parts = rest.split('/').skip(1);
        let owner = parts.next().unwrap_or("");
        let name = parts.next().unwrap_or("").trim_end_matches(".git");
        let repository = format!("{owner}/{name}");
        return match is_repository(&repository) && !NOT_OWNERS.contains(&owner) {
            true => Ok(Source::Repository(repository)),
            false => Err(format!("{said} is a GitHub address that names no repository.")),
        };
    }
    if is_web {
        return Ok(Source::Website(said.to_string()));
    }
    match is_repository(said) {
        true => Ok(Source::Repository(said.to_string())),
        false => Err(format!(
            "{said} is not a repository, owner/repository, or a web address, so there is nothing to look at."
        )),
    }
}

/// `owner/name`, each part a GitHub name: never empty, never a flag, never a path step.
fn is_repository(said: &str) -> bool {
    let mut parts = said.split('/');
    let (Some(owner), Some(name), None) = (parts.next(), parts.next(), parts.next()) else {
        return false;
    };
    [owner, name].iter().all(|part| is_name(part) && !part.starts_with('.'))
}

/// A name that can be passed as one argument and read as nothing but a name.
fn is_name(said: &str) -> bool {
    !said.is_empty()
        && !said.starts_with('-')
        && said.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
}

/// The repository a page points at: an install line first, then the first link to one.
fn repository_on(page: &str) -> Option<String> {
    for line in ["plugin marketplace add ", "npx skills add "] {
        for (at, _) in page.match_indices(line) {
            if let Ok(Source::Repository(repository)) = source_of(token(&page[at + line.len()..])) {
                return Some(repository);
            }
        }
    }
    page.match_indices("github.com/").find_map(|(at, _)| {
        let mut parts = token(&page[at + "github.com/".len()..]).split('/');
        let candidate = format!("{}/{}", parts.next()?, parts.next()?.trim_end_matches(".git"));
        let owner = candidate.split('/').next()?;
        (is_repository(&candidate) && !NOT_OWNERS.contains(&owner)).then_some(candidate)
    })
}

/// The text up to where markup or prose takes over again.
fn token(from: &str) -> &str {
    let end = from
        .find(|c: char| c.is_whitespace() || matches!(c, '"' | '\'' | '<' | '>' | '`' | '(' | ')' | ',' | ';' | '#' | '?'))
        .unwrap_or(from.len());
    from[..end].trim_end_matches('.')
}

#[derive(Deserialize)]
struct Marketplace {
    name: String,
    #[serde(default)]
    plugins: Vec<Listed>,
}

#[derive(Deserialize)]
struct Listed {
    name: String,
    #[serde(default)]
    description: Option<String>,
    /// A folder in the same repository; any other shape keeps its skills elsewhere.
    #[serde(default)]
    source: Option<Where>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum Where {
    Folder(String),
    Elsewhere(serde::de::IgnoredAny),
}

fn look_at_repository(repository: &str, reader: &impl SourceReader) -> Found {
    let paths = match reader.paths(repository) {
        Ok(paths) => paths,
        Err(why) => return unknown(format!("{repository} could not be read on GitHub: {why}")),
    };
    if paths.iter().any(|path| path == MARKETPLACE) {
        return plugin_in(repository, &paths, reader);
    }
    if paths.iter().any(|path| path == SKILL_FILE) {
        let fallback = repository.split('/').nth(1).unwrap_or(repository);
        return skill_named(repository, &skill_read(repository, SKILL_FILE, fallback, reader).piece);
    }
    let skills: Vec<Choice> = skills_under(&paths, "skills/")
        .iter()
        .map(|one| skill_read(repository, &format!("skills/{one}/{SKILL_FILE}"), one, reader))
        .collect();
    match <[Choice; 1]>::try_from(skills) {
        Ok([one]) => skill_named(repository, &one.piece),
        Err(skills) if skills.is_empty() => unknown(format!(
            "{repository} holds neither a plugin marketplace ({MARKETPLACE}) nor a skill ({SKILL_FILE}), so there is nothing Kit can install from it."
        )),
        Err(several) => match several.iter().find(|one| !is_name(one.piece.name())) {
            Some(bad) => skill_named(repository, &bad.piece),
            None => Found::Several { repository: repository.to_string(), choices: several },
        },
    }
}

fn plugin_in(repository: &str, paths: &[String], reader: &impl SourceReader) -> Found {
    let bytes = match reader.file(repository, MARKETPLACE) {
        Ok(bytes) => bytes,
        Err(why) => return unknown(format!("{repository}'s {MARKETPLACE} could not be read: {why}")),
    };
    let market = match ipc::decode::<Marketplace>("a plugin marketplace", &bytes) {
        Ok(market) => market,
        Err(why) => return unknown(format!("{repository}'s {MARKETPLACE} is not a marketplace Kit can read: {why}")),
    };
    if !is_name(&market.name) {
        return unknown(format!("{repository}'s marketplace is named `{}`, which Kit will not pass to a command.", market.name));
    }
    if market.plugins.is_empty() {
        return unknown(format!("{repository}'s marketplace `{}` lists no plugin.", market.name));
    }
    if let Some(bad) = market.plugins.iter().find(|one| !is_name(&one.name)) {
        return unknown(format!("{repository}'s plugin is named `{}`, which Kit will not pass to a command.", bad.name));
    }
    let mut choices: Vec<Choice> = market
        .plugins
        .iter()
        .map(|plugin| Choice { says: plugin.description.clone(), piece: plugin_piece(repository, &market.name, plugin, paths) })
        .collect();
    match choices.len() {
        1 => Found::Piece(choices.remove(0).piece),
        _ => Found::Several { repository: repository.to_string(), choices },
    }
}

fn plugin_piece(repository: &str, marketplace: &str, plugin: &Listed, paths: &[String]) -> Piece {
    let skills = match &plugin.source {
        Some(Where::Folder(folder)) => {
            let folder = folder.trim_start_matches("./").trim_end_matches('/');
            let under = match folder {
                "" | "." => String::from("skills/"),
                folder => format!("{folder}/skills/"),
            };
            skills_under(paths, &under)
        }
        Some(Where::Elsewhere(_)) | None => Vec::new(),
    };
    let qualified = format!("{}@{marketplace}", plugin.name);
    Piece::Plugin {
        name: plugin.name.clone(),
        marketplace: marketplace.to_string(),
        repository: repository.to_string(),
        skills,
        commands: vec![
            CommandLine::of(PROGRAM, &["plugin", "marketplace", "add", repository]),
            CommandLine::of(PROGRAM, &["plugin", "install", &qualified]),
        ],
    }
}

/// One skill a look found, or why its name is one no command may carry.
fn skill_named(repository: &str, piece: &Piece) -> Found {
    let name = piece.name();
    if !is_name(name) {
        return unknown(format!("{repository}'s skill is named `{name}`, which Kit will not pass to a command."));
    }
    Found::Piece(piece.clone())
}

/// A skill by its own name and words for itself, or `fallback` where its file says no name or will not read.
fn skill_read(repository: &str, file: &str, fallback: &str, reader: &impl SourceReader) -> Choice {
    let front = reader.file(repository, file).ok().and_then(|bytes| crate::existing_setup::front_matter(&bytes).ok());
    let (name, says) = match front {
        Some(front) => (front.name.unwrap_or_else(|| fallback.to_string()), front.description),
        None => (fallback.to_string(), None),
    };
    let commands = vec![CommandLine::of(
        "npx",
        &["skills", "add", repository, "--global", "--agent", SKILLS_AGENT, "--skill", &name, "--copy", "--yes"],
    )];
    Choice { says, piece: Piece::Skill { name, repository: repository.to_string(), commands } }
}

/// Each folder directly under `under` that holds a `SKILL.md`, by name.
fn skills_under(paths: &[String], under: &str) -> Vec<String> {
    let mut names: Vec<String> = paths
        .iter()
        .filter_map(|path| {
            let (folder, file) = path.strip_prefix(under)?.split_once('/')?;
            (file == SKILL_FILE).then(|| folder.to_string())
        })
        .collect();
    names.sort();
    names.dedup();
    names
}

/// The agent CLI's program name.
pub const PROGRAM: &str = crate::agent_mod::PROGRAM;

/// The command that uninstalls a plugin, by its name as the inventory spells it: `name@marketplace`.
pub fn uninstall_line(plugin: &str) -> CommandLine {
    CommandLine::of(PROGRAM, &["plugin", "uninstall", plugin])
}

/// How a command that ran went wrong.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Failed {
    /// `-1` where it did not run to an exit of its own.
    pub exit_code: i32,
    pub said: String,
}

/// What runs a command. The real one is [`Cli`]; a test records what it was asked.
pub trait Runner {
    /// What it printed, or how it failed.
    fn run(&self, line: &CommandLine) -> Result<String, Failed>;
}

/// The commands, run on this machine as the person, with Fleet's `PATH` and a time limit.
pub struct Cli {
    pub path: String,
    pub home: PathBuf,
    pub within: Duration,
}

impl Runner for Cli {
    fn run(&self, line: &CommandLine) -> Result<String, Failed> {
        let mut command = Command::new(&line.program);
        command
            .args(&line.args)
            .current_dir(&self.home)
            .env("HOME", &self.home)
            .env("PATH", &self.path);
        crate::agent_mod::run_coded(command, self.within).map_err(|(exit_code, said)| Failed { exit_code, said })
    }
}

/// How an install went.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Installed {
    Done,
    /// `command` is the one that failed, as Add showed it. `taken_back` names, as a person reads
    /// it, what was undone of the commands before it.
    Failed {
        command: String,
        exit_code: i32,
        error: String,
        taken_back: Vec<String>,
    },
}

#[derive(Deserialize)]
struct Known {
    name: String,
}

/// Run the commands a look found. **A plugin's marketplace is taken back where this install added
/// it and the plugin then failed**; one the person already had is never removed, and where whether
/// they had it cannot be read it is treated as theirs.
pub fn install(_asked: Asked, piece: &Piece, runner: &impl Runner) -> Installed {
    let failed = |line: &CommandLine, why: Failed, taken_back: Vec<String>| Installed::Failed {
        command: line.shown(),
        exit_code: why.exit_code,
        error: why.said,
        taken_back,
    };
    match piece {
        Piece::Skill { commands, .. } => {
            for line in commands {
                if let Err(why) = runner.run(line) {
                    return failed(line, why, Vec::new());
                }
            }
            Installed::Done
        }
        Piece::Plugin { marketplace, commands, .. } => {
            let [add, plugin] = commands.as_slice() else {
                return Installed::Failed {
                    command: String::new(),
                    exit_code: -1,
                    error: format!("a plugin installs in two commands, and this look carried {}", commands.len()),
                    taken_back: Vec::new(),
                };
            };
            let listed = CommandLine::of(PROGRAM, &["plugin", "marketplace", "list", "--json"]);
            let held = match runner.run(&listed) {
                Ok(printed) => ipc::decode::<Vec<Known>>("the marketplaces", printed.as_bytes())
                    .map(|known| known.iter().any(|one| &one.name == marketplace))
                    .unwrap_or(true),
                Err(_) => true,
            };
            let added = runner.run(add);
            if let (Err(why), false) = (&added, held) {
                return failed(add, why.clone(), Vec::new());
            }
            match runner.run(plugin) {
                Ok(_) => Installed::Done,
                Err(mut why) if added.is_ok() && !held => {
                    let remove = CommandLine::of(PROGRAM, &["plugin", "marketplace", "remove", marketplace]);
                    match runner.run(&remove) {
                        Ok(_) => failed(plugin, why, vec![format!("the {marketplace} marketplace")]),
                        Err(left) => {
                            why.said = format!(
                                "{}\n\nThe {marketplace} marketplace this install added is still there: `{}` failed with: {}",
                                why.said,
                                remove.shown(),
                                left.said
                            );
                            failed(plugin, why, Vec::new())
                        }
                    }
                }
                Err(why) => failed(plugin, why, Vec::new()),
            }
        }
    }
}

/// Uninstall a plugin, by its name as the inventory spells it.
pub fn uninstall(plugin: &str, runner: &impl Runner) -> Result<(), Failed> {
    runner.run(&uninstall_line(plugin)).map(|_| ())
}

/// Where the person's own skills are, under their home.
pub fn skills_folder(home: &Path) -> PathBuf {
    home.join(SKILLS)
}

/// The macOS Trash, a folder a skill is moved into and back out of by name.
///
/// **A rename, not Finder.** Finder's delete would ask the person to let Fleet control Finder, and
/// the Foundation call that trashes is not reachable without `unsafe`. A rename within the home's
/// volume is what the Trash holds anyway, and the way back is the same rename reversed. Finder's
/// own Put Back does not know where it came from; Undo does.
pub struct Trash {
    folder: PathBuf,
}

impl Trash {
    /// The person's own Trash.
    pub fn of(home: &Path) -> Trash {
        Trash { folder: home.join(TRASH) }
    }

    /// Move `folder`, one of the person's own skills under `home`, into the Trash. Where it went.
    pub fn put(&self, home: &Path, folder: &Path) -> Result<PathBuf, String> {
        let skills = skills_folder(home);
        let named = folder.file_name().and_then(|name| name.to_str()).unwrap_or("");
        let inside = folder.starts_with(&skills)
            && folder != skills
            && folder.components().all(|part| matches!(part, std::path::Component::Normal(_) | std::path::Component::RootDir));
        if !inside || named.is_empty() || named.starts_with('.') {
            return Err(format!("{} is not one of the skills in {}", folder.display(), skills.display()));
        }
        if std::fs::symlink_metadata(folder).is_err() {
            return Err(format!("{} is not there", folder.display()));
        }
        std::fs::create_dir_all(&self.folder).map_err(|why| format!("{} could not be made: {why}", self.folder.display()))?;
        let to = (1..1000)
            .map(|n| match n {
                1 => self.folder.join(named),
                n => self.folder.join(format!("{named} {n}")),
            })
            .find(|place| std::fs::symlink_metadata(place).is_err())
            .ok_or_else(|| format!("the Trash already holds a thousand folders named {named}"))?;
        std::fs::rename(folder, &to).map_err(|why| format!("{} could not be moved to the Trash: {why}", folder.display()))?;
        Ok(to)
    }

    /// Move what [`Trash::put`] moved back to where it was, one of the skills under `home`.
    /// Refused for anything not directly in the Trash, and where something has since taken its place.
    pub fn take_back(&self, home: &Path, trashed: &Path, to: &Path) -> Result<(), String> {
        let skills = skills_folder(home);
        if !to.starts_with(&skills) || to == skills {
            return Err(format!("{} is not a place in {}", to.display(), skills.display()));
        }
        if trashed.parent() != Some(self.folder.as_path()) {
            return Err(format!("{} is not in the Trash", trashed.display()));
        }
        if std::fs::symlink_metadata(trashed).is_err() {
            return Err(format!("{} is no longer in the Trash", trashed.display()));
        }
        if std::fs::symlink_metadata(to).is_ok() {
            return Err(format!("{} is there again, so the one in the Trash was left where it is", to.display()));
        }
        if let Some(parent) = to.parent() {
            std::fs::create_dir_all(parent).map_err(|why| format!("{} could not be made: {why}", parent.display()))?;
        }
        std::fs::rename(trashed, to).map_err(|why| format!("{} could not be moved back: {why}", trashed.display()))
    }
}

/// The real reader: `gh` for a repository, one GET for a website.
pub struct ForgeReader {
    pub within: Duration,
}

impl ForgeReader {
    fn gh(&self, args: &[&str]) -> Result<String, String> {
        let mut command = crate::background::command("gh");
        command.args(args);
        crate::agent_mod::run_coded(command, self.within).map_err(|(_, said)| said)
    }
}

impl SourceReader for ForgeReader {
    fn paths(&self, repository: &str) -> Result<Vec<String>, String> {
        let tree = format!("repos/{repository}/git/trees/HEAD?recursive=1");
        let printed = self.gh(&["api", &tree, "--jq", ".tree[] | select(.type == \"blob\") | .path"])?;
        Ok(printed.lines().map(str::to_string).collect())
    }

    fn file(&self, repository: &str, path: &str) -> Result<Vec<u8>, String> {
        let contents = format!("repos/{repository}/contents/{path}");
        self.gh(&["api", "-H", "Accept: application/vnd.github.raw", &contents]).map(String::into_bytes)
    }

    fn page(&self, url: &str) -> Result<String, String> {
        let client = reqwest::blocking::Client::builder()
            .timeout(self.within)
            .build()
            .map_err(|why| why.to_string())?;
        let answer = client.get(url).send().map_err(|why| why.to_string())?;
        if !answer.status().is_success() {
            return Err(format!("it answered {}", answer.status()));
        }
        let bytes = answer.bytes().map_err(|why| why.to_string())?;
        let kept = &bytes[..bytes.len().min(MOST_PAGE)];
        Ok(String::from_utf8_lossy(kept).into_owned())
    }
}
