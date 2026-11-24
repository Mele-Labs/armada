//! Level one: the parts of a repository, read from its modules, files, compose services and Kit servers. **Table-driven**:
//! what a dependency or a file means is in `known.toml`, and the rules here only ask it.
//!
//! An entry point is something people use, a service is a server or a process behind them, a store is where state is kept
//! and an outside part is something the project reaches and does not hold.

use std::collections::BTreeSet;

use ipc::{PartOrigin, SystemPart, SystemPartKind};

use super::model::{holds, Part};
use super::repo::{extension_of, is_proof, Repo};
use super::tables::{known, plain, words};
use crate::feature_library::{json_scan, Module, ModuleDep, ModuleKind};

/// What a module is to the map.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Role {
    /// The project's own tooling, or a folder with nothing to say.
    Skip,
    /// Code others use: a piece of whichever part reaches it.
    Lib,
    Entry,
    Service,
    /// An HTTP server nobody runs from here: a library that describes the routes.
    Api,
    /// Where state is kept, by the dependency that says so.
    Store,
}

/// A store found: the part, and the modules that keep it or use it.
pub(super) struct StoreFound {
    pub(super) part: usize,
    /// The name of the rule that found it.
    pub(super) rule: String,
    /// The dependency idents (`rusqlite`, `sqlite3`) a file uses it by.
    pub(super) idents: Vec<String>,
    /// The library modules that are the store's own code.
    pub(super) owners: Vec<usize>,
    /// The modules that use the dependency directly.
    pub(super) users: Vec<usize>,
}

/// Something outside found: the part, the rule that named it, and the hosts it is reached at.
pub(super) struct OutsideFound {
    pub(super) part: usize,
    pub(super) rule: Option<usize>,
    pub(super) hosts: Vec<String>,
    /// The modules whose dependencies name it.
    pub(super) users: Vec<usize>,
}

pub(super) struct Detected {
    pub(super) parts: Vec<Part>,
    pub(super) roles: Vec<Role>,
    /// The part each module belongs to as its own.
    pub(super) own: Vec<Option<usize>>,
    pub(super) stores: Vec<StoreFound>,
    pub(super) outside: Vec<OutsideFound>,
}

/// A name a person reads: `bridge-native` is `Bridge native`.
pub(super) fn humanize(name: &str) -> String {
    let spaced = name.replace(['-', '_'], " ");
    let mut chars = spaced.trim().chars();
    chars.next().map(|first| first.to_uppercase().collect::<String>() + chars.as_str()).unwrap_or_default()
}

pub(super) fn slug(name: &str) -> String {
    let mut out = String::new();
    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if !out.ends_with('-') && !out.is_empty() {
            out.push('-');
        }
    }
    out.trim_end_matches('-').to_string()
}

fn folder_name(module: &Module) -> String {
    module.root.rsplit('/').next().filter(|last| !last.is_empty()).unwrap_or(&module.name).to_string()
}

/// The dependency names of a module, plain, with an npm manifest's dev dependencies and a Python requirements file.
fn deps_of(repo: &Repo, module: &Module) -> Vec<String> {
    let mut deps: Vec<String> = module.deps.iter().filter_map(|dep| if let ModuleDep::Name(name) = dep { Some(plain(name)) } else { None }).collect();
    let at = |file: &str| if module.root.is_empty() { file.to_string() } else { format!("{}/{file}", module.root) };
    if module.kind == ModuleKind::Npm {
        if let Some(text) = repo.text(&at("package.json")) {
            for group in ["devDependencies"] {
                deps.extend(json_scan::member(&text, group).map(json_scan::members).unwrap_or_default().into_iter().map(|(key, _)| plain(&key)));
            }
        }
    }
    deps.extend(requirements_of(repo, &module.root));
    deps.sort();
    deps.dedup();
    deps
}

/// The package names a `requirements.txt` in `folder` lists.
fn requirements_of(repo: &Repo, folder: &str) -> Vec<String> {
    let file = if folder.is_empty() { "requirements.txt".to_string() } else { format!("{folder}/requirements.txt") };
    repo.text(&file)
        .map(|text| {
            text.lines()
                .map(str::trim)
                .filter(|line| !line.is_empty() && !line.starts_with(['#', '-']))
                .map(|line| plain(line.split(|c: char| !(c.is_alphanumeric() || matches!(c, '-' | '_' | '.'))).next().unwrap_or("")))
                .filter(|name| !name.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

fn has_main(repo: &Repo, module: &Module) -> bool {
    let at = |file: &str| if module.root.is_empty() { file.to_string() } else { format!("{}/{file}", module.root) };
    match module.kind {
        ModuleKind::Cargo => repo.has(&at("src/main.rs")) || repo.under(&at("src/bin")).next().is_some(),
        ModuleKind::Go => repo.has(&at("main.go")) || repo.under(&at("cmd")).next().is_some(),
        ModuleKind::Python => repo.has(&at("__main__.py")) || repo.has(&at("main.py")),
        ModuleKind::Npm => repo.text(&at("package.json")).is_some_and(|text| json_scan::member(&text, "bin").is_some()),
        _ => false,
    }
}

fn role_of(repo: &Repo, module: &Module, deps: &[String]) -> Role {
    let known = known();
    if known.tooling(&module.name) || known.tooling(&folder_name(module)) || module.kind == ModuleKind::Folder {
        return Role::Skip;
    }
    let at = |file: &str| if module.root.is_empty() { file.to_string() } else { format!("{}/{file}", module.root) };
    let main = has_main(repo, module);
    if known.entry(deps).is_some() {
        return Role::Entry;
    }
    if known.serves(deps) {
        return if main { Role::Service } else { Role::Api };
    }
    if known.deployed(deps, |file| repo.has(&at(file))) {
        return Role::Service;
    }
    if main {
        let own_name = plain(&module.name) == plain(&repo.manifest) || repo.root.file_name().is_some_and(|dir| plain(&dir.to_string_lossy()) == plain(&module.name));
        return if own_name || known.command(deps) { Role::Entry } else { Role::Service };
    }
    if known.store(deps).is_some() {
        return Role::Store;
    }
    Role::Lib
}

/// The paths a module's part holds: its folder, or its `src` where the module is the repository itself.
fn paths_of(repo: &Repo, module: &Module) -> Vec<String> {
    if !module.root.is_empty() {
        return vec![format!("{}/", module.root)];
    }
    ["src/", "app/", "lib/"].iter().find(|folder| repo.under(folder).next().is_some()).map_or_else(|| vec!["./".to_string()], |folder| vec![(*folder).to_string()])
}

fn wire(id: String, name: String, kind: SystemPartKind, summary: String, paths: Vec<String>, from: &str, several: bool, manifest: &str) -> SystemPart {
    SystemPart {
        id,
        name,
        kind,
        summary,
        within: None,
        manifest_id: several.then(|| ipc::ManifestId::carried(manifest)),
        paths,
        zooms: false,
        origin: PartOrigin::Detected { from: from.to_string() },
    }
}

struct Builder<'a> {
    repo: &'a Repo,
    parts: Vec<Part>,
    several: bool,
}

impl Builder<'_> {
    fn unique(&self, id: String) -> String {
        let taken = |one: &str| self.parts.iter().any(|part| part.wire.id == one);
        if !taken(&id) {
            return id;
        }
        (2..).map(|n| format!("{id}-{n}")).find(|one| !taken(one)).unwrap_or(id)
    }

    fn add(&mut self, id: &str, name: String, kind: SystemPartKind, summary: String, paths: Vec<String>, from: &str, what: &str) -> usize {
        let id = self.unique(id.to_string());
        let wire = wire(id, name, kind, summary, paths, from, self.several, &self.repo.manifest);
        self.parts.push(Part { wire, modules: Vec::new(), libs: Vec::new(), screen: false, port: None, api: false, what: what.to_string() });
        self.parts.len() - 1
    }

    fn covered(&self, path: &str) -> bool {
        self.parts.iter().any(|part| part.wire.kind != SystemPartKind::Store && part.wire.paths.iter().any(|prefix| holds(prefix, path) && prefix.trim_matches('/') != "."))
    }
}

/// The text between the first `"key": "` and the closing quote, for a manifest's own name.
fn string_field(text: &str, key: &str) -> Option<String> {
    let at = text.find(&format!("\"{key}\""))?;
    let rest = text[at + key.len() + 2..].trim_start().strip_prefix(':')?.trim_start().strip_prefix('"')?;
    Some(rest.split('"').next()?.to_string())
}

/// The hosts a web extension's manifest matches pages on, without `localhost`.
fn matched_hosts(text: &str) -> Vec<String> {
    let mut hosts: Vec<String> = json_scan::strings(text)
        .into_iter()
        .filter(|one| one.contains("://"))
        .filter_map(|one| one.split("://").nth(1).map(|rest| rest.split('/').next().unwrap_or(rest).trim_start_matches("*.").to_string()))
        .filter(|host| !host.is_empty() && host != "*" && !host.starts_with("localhost") && !host.starts_with("127."))
        .collect();
    hosts.sort();
    hosts.dedup();
    hosts
}

/// A web framework a Python file starts, as it is named in the code.
const PY_SERVERS: &[(&str, &str)] = &[("FastAPI(", "FastAPI"), ("Flask(", "Flask"), ("Starlette(", "Starlette"), ("Sanic(", "Sanic"), ("web.Application(", "aiohttp")];

pub(super) fn detect(repo: &Repo) -> Detected {
    let known = known();
    let layout = &repo.layout;
    let n = layout.modules.len();
    let deps: Vec<Vec<String>> = layout.modules.iter().map(|module| deps_of(repo, module)).collect();
    let mut roles: Vec<Role> = layout.modules.iter().zip(&deps).map(|(module, deps)| role_of(repo, module, deps)).collect();
    // What implements an API nobody runs is the service behind it.
    for (from, to) in &layout.uses {
        if roles[*to] == Role::Api && roles[*from] == Role::Lib {
            roles[*from] = Role::Service;
        }
    }
    let mut own: Vec<Option<usize>> = vec![None; n];
    let mut build = Builder { repo, parts: Vec::new(), several: false };

    // Modules that are entry points, services and APIs.
    for at in 0..n {
        let module = &layout.modules[at];
        let (kind, what, screen, api) = match roles[at] {
            Role::Entry => match known.entry(&deps[at]) {
                Some(rule) => (SystemPartKind::Entry, rule.what.clone(), rule.screen, false),
                None => (SystemPartKind::Entry, "command".to_string(), false, false),
            },
            Role::Service => (SystemPartKind::Service, "server".to_string(), false, false),
            Role::Api => (SystemPartKind::Entry, "HTTP API".to_string(), false, true),
            _ => continue,
        };
        let paths = paths_of(repo, module);
        let base = if module.root.is_empty() { module.name.clone() } else { folder_name(module) };
        let name = if api && !base.to_ascii_lowercase().contains("api") { format!("{} API", humanize(&base)) } else { humanize(&base).replace("Api", "API") };
        let summary = format!("{} in {}", capitalized(&what), paths[0].trim_end_matches('/'));
        let from = module.kind.label();
        let part = build.add(&slug(&base), name, kind, summary, paths, from, &what);
        build.parts[part].modules.push(at);
        build.parts[part].screen = screen;
        build.parts[part].api = api;
        own[at] = Some(part);
    }

    // Web extensions: a `manifest.json` that says it is one.
    let mut hosts_read: BTreeSet<String> = BTreeSet::new();
    for (path, _) in repo.layout.files.iter().filter(|(path, _)| path.rsplit('/').next() == Some("manifest.json")) {
        let Some(text) = repo.text(path) else { continue };
        if !text.contains("\"manifest_version\"") {
            continue;
        }
        let folder = path.trim_end_matches("manifest.json").to_string();
        let name = string_field(&text, "name").map(|name| name.split(['—', '-', ':']).last().unwrap_or(&name).trim().to_string()).filter(|name| !name.is_empty()).unwrap_or_else(|| humanize(folder.trim_end_matches('/').rsplit('/').next().unwrap_or("extension")));
        let part = build.add("extension", name, SystemPartKind::Entry, format!("Browser extension in {}", folder.trim_end_matches('/')), vec![if folder.is_empty() { "./".to_string() } else { folder }], "manifest.json", "browser extension");
        build.parts[part].screen = true;
        hosts_read.extend(matched_hosts(&text));
        if build.parts[part].wire.paths[0] == "./" {
            build.parts[part].wire.paths = vec![path.clone()];
        }
    }

    // Python servers and scripts that no manifest named.
    let mut py_servers: Vec<(String, &str)> = Vec::new();
    for source in repo.sources.iter().filter(|source| extension_of(&source.path) == "py") {
        if build.covered(&source.path) {
            continue;
        }
        if let Some((_, framework)) = PY_SERVERS.iter().find(|(needle, _)| source.text.contains(needle)) {
            let folder = source.path.rsplit_once('/').map_or("", |(folder, _)| folder).to_string();
            if !py_servers.iter().any(|(one, _)| *one == folder) {
                py_servers.push((folder, framework));
            }
        }
    }
    for (folder, framework) in &py_servers {
        let top = folder.split('/').next().filter(|top| !top.is_empty()).unwrap_or("app");
        let part = build.add(&slug(&format!("{framework} {top}")), format!("{framework} {top}"), SystemPartKind::Service, format!("{framework} server in {}", if folder.is_empty() { "." } else { folder }), vec![if folder.is_empty() { "./".to_string() } else { format!("{folder}/") }], "python", framework);
        build.parts[part].port = None;
    }
    for source in repo.sources.iter().filter(|source| extension_of(&source.path) == "py" && source.path.matches('/').count() <= 1) {
        let command = source.text.contains("__main__") && ["argparse", "sys.argv", "click", "typer"].iter().any(|word| source.text.contains(word));
        if command && !build.covered(&source.path) {
            let name = source.path.rsplit('/').next().unwrap_or(&source.path).to_string();
            build.add(&slug(&name), name.clone(), SystemPartKind::Entry, format!("Command, {}", source.path), vec![source.path.clone()], "python", "command");
        }
    }

    // Compose: stores by image, and a service where no server was found.
    let mut stores: Vec<StoreFound> = Vec::new();
    for service in &repo.composed {
        if let Some(rule) = service.image.as_deref().and_then(|image| known.image_store(image)) {
            if build.parts.iter().all(|part| part.wire.name != rule.name) {
                let part = build.add(&format!("store-{}", slug(&rule.name)), rule.name.clone(), SystemPartKind::Store, format!("{} from {}", rule.name, service.file), Vec::new(), "compose", "store");
                build.parts[part].port = service.ports.first().copied();
                stores.push(StoreFound { part, rule: rule.name.clone(), idents: Vec::new(), owners: Vec::new(), users: Vec::new() });
            }
        } else if service.built && !build.parts.iter().any(|part| part.wire.kind == SystemPartKind::Service) {
            let part = build.add(&slug(&service.name), humanize(&service.name), SystemPartKind::Service, format!("Compose service in {}", service.file), Vec::new(), "compose", "server");
            build.parts[part].port = service.ports.first().copied();
        }
    }

    // Stores by dependency.
    for at in 0..n {
        let Some(rule) = known.store(&deps[at]) else { continue };
        if matches!(roles[at], Role::Skip) {
            continue;
        }
        let found = match stores.iter().position(|one| one.rule == rule.name) {
            Some(found) => found,
            None => {
                let file = state_file(repo, &rule.ext).unwrap_or_else(|| rule.name.clone());
                let part = build.add(&format!("store-{}", slug(&file)), file.clone(), SystemPartKind::Store, rule.name.clone(), Vec::new(), "dependencies", "store");
                stores.push(StoreFound { part, rule: rule.name.clone(), idents: Vec::new(), owners: Vec::new(), users: Vec::new() });
                stores.len() - 1
            }
        };
        let ident = deps[at].iter().find(|dep| known.store(std::slice::from_ref(*dep)).is_some_and(|one| one.name == rule.name)).cloned().unwrap_or_default();
        let one = &mut stores[found];
        if !ident.is_empty() && !one.idents.contains(&ident.replace('-', "_")) {
            one.idents.push(ident.replace('-', "_"));
        }
        if roles[at] == Role::Lib || roles[at] == Role::Store {
            roles[at] = Role::Store;
            one.owners.push(at);
            let path = format!("{}/", layout.modules[at].root);
            let wire = &mut build.parts[one.part].wire;
            if !layout.modules[at].root.is_empty() && !wire.paths.contains(&path) {
                wire.paths.push(path);
            }
        } else {
            one.users.push(at);
        }
    }

    // Folders of state files the code writes, only where nothing else keeps state.
    if stores.is_empty() {
        for source in &repo.sources {
            if !(source.text.contains("json.dump") || source.text.contains("write_text") || source.text.contains("writeFile") || source.text.contains("fs::write")) {
                continue;
            }
            let Some(folder) = home_folder(&source.text) else { continue };
            let last = folder.trim_end_matches('/').rsplit('/').next().unwrap_or("state").to_string();
            let singular = last.strip_suffix('s').unwrap_or(&last);
            let name = format!("{} files", humanize(singular));
            if build.parts.iter().any(|part| part.wire.name == name) {
                continue;
            }
            let part = build.add(&format!("store-{}", slug(&name)), name, SystemPartKind::Store, format!("Files in {folder}"), vec![source.path.clone()], "files", "store");
            stores.push(StoreFound { part, rule: String::new(), idents: Vec::new(), owners: Vec::new(), users: Vec::new() });
        }
    }

    // Outside: dependencies, then what files name.
    let mut outside: Vec<OutsideFound> = Vec::new();
    let named = |rule: &super::tables::OutsideRule, source_path: &str, text: &str| {
        let stem_words = words(source_path.rsplit('/').next().unwrap_or(source_path));
        rule.stems.iter().any(|stem| stem_words.contains(stem))
            || rule.hosts.iter().any(|host| text.contains(host.as_str()))
            || rule.quoted.iter().any(|word| text.contains(&format!("\"{word}\"")))
    };
    for (at, rule) in known.outside.iter().enumerate() {
        let users: Vec<usize> = (0..n).filter(|module| !matches!(roles[*module], Role::Skip) && known.outside_of(&deps[*module]).contains(&at)).collect();
        let by_file = repo.sources.iter().any(|source| !matches!(layout.holding(&source.path).map(|module| &roles[module]), Some(Role::Skip)) && named(rule, &source.path, &source.text));
        if users.is_empty() && !by_file {
            continue;
        }
        let hosts = rule.hosts.clone();
        let part = build.add(&format!("outside-{}", slug(&rule.name)), rule.name.clone(), SystemPartKind::Outside, rule.summary.clone(), Vec::new(), if users.is_empty() { "files" } else { "dependencies" }, "outside");
        outside.push(OutsideFound { part, rule: Some(at), hosts, users });
    }
    // Other `api.<name>.<tld>` hosts the code names.
    let mut seen_hosts: BTreeSet<String> = known.outside.iter().flat_map(|rule| rule.hosts.iter().cloned()).collect();
    for source in repo.sources.iter().filter(|source| !is_proof(&source.path)) {
        for host in api_hosts(&source.text) {
            if seen_hosts.insert(host.clone()) {
                let label = host.split('.').rev().nth(1).unwrap_or(&host).to_string();
                let name = format!("{} API", if label.len() <= 4 { label.to_ascii_uppercase() } else { humanize(&label) });
                let part = build.add(&format!("outside-{}", slug(&host)), name, SystemPartKind::Outside, host.clone(), Vec::new(), "files", "outside");
                outside.push(OutsideFound { part, rule: None, hosts: vec![host], users: Vec::new() });
            }
        }
    }
    if !hosts_read.is_empty() {
        let list: Vec<String> = hosts_read.into_iter().collect();
        let part = build.add("outside-pages", "Pages it reads".to_string(), SystemPartKind::Outside, list.join(", "), Vec::new(), "manifest.json", "outside");
        outside.push(OutsideFound { part, rule: None, hosts: list, users: Vec::new() });
    }

    let mut parts = build.parts;
    for part in &mut parts {
        part.wire.zooms = !part.wire.paths.is_empty() && matches!(part.wire.kind, SystemPartKind::Entry | SystemPartKind::Service);
    }
    Detected { parts, roles, own, stores, outside }
}

fn capitalized(text: &str) -> String {
    let mut chars = text.chars();
    chars.next().map(|first| first.to_uppercase().collect::<String>() + chars.as_str()).unwrap_or_default()
}

/// A file name with one of `exts` that the code names in quotes, the most often named first.
fn state_file(repo: &Repo, exts: &[String]) -> Option<String> {
    let mut counts: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    for source in &repo.sources {
        for ext in exts {
            let dotted = format!(".{ext}\"");
            for (at, _) in source.text.match_indices(&dotted) {
                let stem: String = source.text[..at].chars().rev().take_while(|c| c.is_alphanumeric() || matches!(c, '_' | '-')).collect::<Vec<_>>().into_iter().rev().collect();
                if !stem.is_empty() {
                    *counts.entry(format!("{stem}.{ext}")).or_default() += 1;
                }
            }
        }
    }
    counts.into_iter().max_by_key(|(_, count)| *count).map(|(name, _)| name)
}

/// A folder under the home folder that the text names: `"~/.name/sessions"` or `expanduser("~/.name/sessions")`.
fn home_folder(text: &str) -> Option<String> {
    let at = text.find("\"~/").or_else(|| text.find("'~/"))?;
    let rest = &text[at + 1..];
    let end = rest.find(['"', '\'']).unwrap_or(rest.len());
    let path = rest[..end].trim().to_string();
    (path.matches('/').count() >= 2 && !path.contains(' ') && !path.contains('{')).then_some(path)
}

/// The `api.<name>.<tld>` hosts a text names in an address, `localhost` and examples left out.
pub(super) fn api_hosts(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for (at, _) in text.match_indices("://api.") {
        let host: String = text[at + 3..].chars().take_while(|c| c.is_alphanumeric() || matches!(c, '.' | '-')).collect();
        let host = host.trim_end_matches('.').to_string();
        if host.matches('.').count() >= 2 && !host.contains("example") && !host.ends_with(".test") && !host.ends_with(".local") && !out.contains(&host) {
            out.push(host);
        }
    }
    out
}
