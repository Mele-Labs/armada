//! What each build manifest says: whether its folder is a module, what it is called, and which other modules it
//! depends on. **Read, never run**, and a manifest that does not read says nothing. A manifest that only lists
//! members (a virtual Cargo workspace, an npm `workspaces` root, a Maven aggregator, a Gradle settings file with
//! includes) is a root, not a room.

use std::path::Path;

use super::json_scan::{member, members, strings};
use super::{Dep, Kind, Module};

/// The folder a manifest sits in, and what sits beside it.
pub(super) struct Here<'a> {
    pub(super) abs: &'a Path,
    /// Repository-relative, empty for the repository's own root.
    pub(super) rel: &'a str,
    pub(super) files: &'a [String],
}

impl Here<'_> {
    fn folder_name(&self) -> String {
        self.rel.rsplit('/').next().filter(|last| !last.is_empty()).map(str::to_string).unwrap_or_else(|| {
            self.abs.file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_else(|| ".".to_string())
        })
    }

    fn has(&self, name: &str) -> bool {
        self.files.iter().any(|file| file == name)
    }

    fn module(&self, name: String, kind: Kind, deps: Vec<Dep>) -> Module {
        Module { name, root: self.rel.to_string(), kind, deps, spec_files: false }
    }
}

/// `relative` taken from the folder `from`, as a repository-relative path: `None` where it leaves the repository.
pub(super) fn joined(from: &str, relative: &str) -> Option<String> {
    let mut parts: Vec<&str> = from.split('/').filter(|part| !part.is_empty()).collect();
    for part in relative.split(['/', '\\']) {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            other => parts.push(other),
        }
    }
    Some(parts.join("/"))
}

/// The modules the manifest file `file` in `here` names, by its kind.
pub(super) fn read(here: &Here, file: &str, text: &str) -> Vec<Module> {
    match file {
        "Cargo.toml" => cargo(here, text).into_iter().collect(),
        "package.json" => npm(here, text).into_iter().collect(),
        "go.mod" => vec![go(here, text)].into_iter().flatten().collect(),
        "pyproject.toml" => python(here, text).into_iter().collect(),
        "pom.xml" => maven(here, text).into_iter().collect(),
        "Package.swift" => swift(here, text).into_iter().collect(),
        "settings.gradle" | "settings.gradle.kts" => gradle(here, text),
        other if other.ends_with(".csproj") => dotnet(here, other, text).into_iter().collect(),
        _ => Vec::new(),
    }
}

pub(super) fn is_manifest(file: &str) -> bool {
    matches!(file, "Cargo.toml" | "package.json" | "go.mod" | "pyproject.toml" | "pom.xml" | "Package.swift" | "settings.gradle" | "settings.gradle.kts")
        || file.ends_with(".csproj")
}

fn cargo(here: &Here, text: &str) -> Option<Module> {
    let table: toml::Table = text.parse().ok()?;
    let name = table.get("package")?.as_table()?.get("name")?.as_str()?.to_string();
    let mut deps = Vec::new();
    let mut take = |from: Option<&toml::Value>| {
        for (key, value) in from.and_then(toml::Value::as_table).into_iter().flatten() {
            match value.as_table() {
                Some(spec) => match spec.get("path").and_then(toml::Value::as_str).and_then(|path| joined(here.rel, path)) {
                    Some(path) => deps.push(Dep::Path(path)),
                    None => deps.push(Dep::Name(spec.get("package").and_then(toml::Value::as_str).unwrap_or(key).to_string())),
                },
                None => deps.push(Dep::Name(key.clone())),
            }
        }
    };
    take(table.get("dependencies"));
    take(table.get("build-dependencies"));
    for target in table.get("target").and_then(toml::Value::as_table).into_iter().flat_map(|all| all.values()) {
        take(target.get("dependencies"));
    }
    Some(here.module(name, Kind::Cargo, deps))
}

fn npm(here: &Here, text: &str) -> Option<Module> {
    if here.has("pnpm-workspace.yaml") || here.has("lerna.json") || member(text, "workspaces").is_some() {
        return None;
    }
    let name = member(text, "name").and_then(|value| strings(value).into_iter().next()).unwrap_or_else(|| here.folder_name());
    let mut deps = Vec::new();
    for group in ["dependencies", "peerDependencies", "optionalDependencies"] {
        for (key, _) in member(text, group).map(members).unwrap_or_default() {
            deps.push(Dep::Name(key));
        }
    }
    Some(here.module(name, Kind::Npm, deps))
}

fn go(here: &Here, text: &str) -> Option<Module> {
    let mut name = None;
    let mut deps = Vec::new();
    let mut block = "";
    for line in text.lines() {
        let line = line.split("//").next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        if line == ")" {
            block = "";
            continue;
        }
        if let Some(open) = line.strip_suffix('(') {
            block = open.trim();
            continue;
        }
        let (word, rest) = if block.is_empty() {
            let (word, rest) = line.split_once(char::is_whitespace).unwrap_or((line, ""));
            (word, rest.trim())
        } else {
            (block, line)
        };
        match word {
            "module" => name = Some(rest.trim_matches('"').to_string()),
            "require" => deps.extend(rest.split_whitespace().next().map(|path| Dep::Name(path.trim_matches('"').to_string()))),
            "replace" => {
                if let Some(local) = rest.split("=>").nth(1).and_then(|to| to.split_whitespace().next()) {
                    if local.starts_with('.') {
                        deps.extend(joined(here.rel, local).map(Dep::Path));
                    }
                }
            }
            _ => {}
        }
    }
    Some(here.module(name?, Kind::Go, deps))
}

fn python(here: &Here, text: &str) -> Option<Module> {
    let table: toml::Table = text.parse().ok()?;
    let project = table.get("project").and_then(toml::Value::as_table);
    let poetry = table.get("tool").and_then(|tool| tool.get("poetry")).and_then(toml::Value::as_table);
    let name = project.or(poetry)?.get("name")?.as_str()?.to_string();
    let mut deps: Vec<Dep> = project
        .and_then(|project| project.get("dependencies"))
        .and_then(toml::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(toml::Value::as_str)
        .map(|spec| Dep::Name(spec.chars().take_while(|c| c.is_alphanumeric() || matches!(c, '-' | '_' | '.')).collect()))
        .collect();
    if let Some(all) = poetry.and_then(|poetry| poetry.get("dependencies")).and_then(toml::Value::as_table) {
        deps.extend(all.keys().filter(|key| key.as_str() != "python").map(|key| Dep::Name(key.clone())));
    }
    Some(here.module(name, Kind::Python, deps))
}

fn maven(here: &Here, text: &str) -> Option<Module> {
    if text.contains("<modules>") {
        return None;
    }
    let own = without(text, "parent");
    let own = without(&own, "dependencyManagement");
    let own = without(&own, "build");
    let name = tag_values(&without(&own, "dependencies"), "artifactId").into_iter().next()?;
    let deps = own
        .split("<dependencies>")
        .skip(1)
        .filter_map(|rest| rest.split("</dependencies>").next())
        .flat_map(|block| tag_values(block, "artifactId"))
        .map(Dep::Name)
        .collect();
    Some(here.module(name, Kind::Maven, deps))
}

fn gradle(here: &Here, text: &str) -> Vec<Module> {
    let mut included = Vec::new();
    let mut lines = text.lines().peekable();
    while let Some(line) = lines.next() {
        if !line.trim_start().starts_with("include") {
            continue;
        }
        let mut statement = line.to_string();
        while statement.trim_end().ends_with(',') || statement.matches('(').count() > statement.matches(')').count() {
            match lines.next() {
                Some(more) => statement.push_str(more),
                None => break,
            }
        }
        included.extend(quoted(&statement));
    }
    let build_deps = |dir: &Path| -> Vec<Dep> {
        let text = ["build.gradle", "build.gradle.kts"].iter().find_map(|file| std::fs::read_to_string(dir.join(file)).ok()).unwrap_or_default();
        text.split("project(").skip(1).filter_map(|rest| quoted(rest.split(')').next().unwrap_or("")).into_iter().find(|one| one.starts_with(':'))).map(|one| Dep::Name(one.trim_start_matches(':').to_string())).collect()
    };
    if included.is_empty() {
        return if here.has("build.gradle") || here.has("build.gradle.kts") {
            vec![here.module(here.folder_name(), Kind::Gradle, build_deps(here.abs))]
        } else {
            Vec::new()
        };
    }
    included
        .into_iter()
        .map(|path| {
            let name = path.trim_start_matches(':').to_string();
            let folder = name.replace(':', "/");
            let root = joined(here.rel, &folder).unwrap_or(folder.clone());
            Module { name, root, kind: Kind::Gradle, deps: build_deps(&here.abs.join(&folder)), spec_files: false }
        })
        .collect()
}

fn dotnet(here: &Here, file: &str, text: &str) -> Option<Module> {
    let name = file.strip_suffix(".csproj")?.to_string();
    let deps = text
        .split("<ProjectReference")
        .skip(1)
        .filter_map(|rest| {
            let attribute = rest.split("Include=\"").nth(1)?.split('"').next()?;
            joined(here.rel, attribute).map(Dep::Path)
        })
        .collect();
    Some(here.module(name, Kind::Dotnet, deps))
}

fn swift(here: &Here, text: &str) -> Option<Module> {
    let name = text.split("Package(").nth(1).and_then(|rest| rest.split("name:").nth(1)).and_then(|rest| quoted(rest).into_iter().next())?;
    let deps = text
        .split(".package(")
        .skip(1)
        .filter_map(|rest| {
            let call = rest.split(')').next().unwrap_or("");
            if let Some(path) = call.split("path:").nth(1).and_then(|after| quoted(after).into_iter().next()) {
                return joined(here.rel, &path).map(Dep::Path);
            }
            let url = call.split("url:").nth(1).and_then(|after| quoted(after).into_iter().next())?;
            Some(Dep::Name(url.trim_end_matches('/').rsplit('/').next()?.trim_end_matches(".git").to_string()))
        })
        .collect();
    Some(here.module(name, Kind::Swift, deps))
}

/// The quoted strings in `text`, single or double.
fn quoted(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut open: Option<(char, String)> = None;
    for c in text.chars() {
        match (&mut open, c) {
            (None, '"' | '\'') => open = Some((c, String::new())),
            (Some((quote, _)), c) if c == *quote => out.extend(open.take().map(|(_, one)| one)),
            (Some((_, one)), c) => one.push(c),
            _ => {}
        }
    }
    out
}

/// `text` without each `<tag>…</tag>` block.
fn without(text: &str, tag: &str) -> String {
    let (open, close) = (format!("<{tag}>"), format!("</{tag}>"));
    let mut out = String::new();
    let mut rest = text;
    while let Some(start) = rest.find(&open) {
        out.push_str(&rest[..start]);
        match rest[start..].find(&close) {
            Some(end) => rest = &rest[start + end + close.len()..],
            None => return out,
        }
    }
    out.push_str(rest);
    out
}

/// The text of each `<tag>…</tag>` in `text`, trimmed.
fn tag_values(text: &str, tag: &str) -> Vec<String> {
    let (open, close) = (format!("<{tag}>"), format!("</{tag}>"));
    text.split(&open).skip(1).filter_map(|rest| rest.split(&close).next()).map(|one| one.trim().to_string()).collect()
}
