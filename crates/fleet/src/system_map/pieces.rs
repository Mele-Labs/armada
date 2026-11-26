//! The pieces of a part, seen when it is zoomed into: what stands directly under its source folder, and the libraries it
//! reaches. **Read from the tracked files**, with each piece's one line from the first comment its file opens with.

use std::collections::{BTreeMap, BTreeSet};

use ipc::SystemPartKind;

use super::detect::{slug, Detected, Role};
use super::model::{Piece, Part};
use super::repo::{extension_of, Repo};
use super::tables::known;

/// The most pieces one part shows, the largest first.
const MOST_PIECES: usize = 40;
/// A folder that holds this many entries is the part's pieces itself.
const MANY_BELOW: usize = 3;

/// The libraries each part reaches through other libraries only, each given to the part that is a service where one reaches it.
fn attribute_libs(repo: &Repo, det: &mut Detected) {
    let uses = &repo.layout.uses;
    let mut reached: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for (at, part) in det.parts.iter().enumerate() {
        let mut seen: BTreeSet<usize> = BTreeSet::new();
        let mut queue: Vec<usize> = part.modules.clone();
        while let Some(from) = queue.pop() {
            for (_, to) in uses.iter().filter(|(one, _)| *one == from) {
                if det.roles[*to] == Role::Lib && seen.insert(*to) {
                    queue.push(*to);
                }
            }
        }
        for lib in seen {
            reached.entry(lib).or_default().push(at);
        }
    }
    for (lib, parts) in reached {
        let owner = parts.iter().copied().find(|at| det.parts[*at].wire.kind == SystemPartKind::Service).unwrap_or(parts[0]);
        det.parts[owner].libs.push(lib);
    }
}

/// The first sentence of the comment a file opens with, or of a folder's own file.
fn what_it_does(repo: &Repo, path: &str) -> String {
    let files: Vec<String> = if path.ends_with('/') {
        ["mod.rs", "lib.rs", "__init__.py", "index.ts", "index.js", "README.md"].iter().map(|name| format!("{path}{name}")).collect()
    } else {
        vec![path.to_string()]
    };
    for file in files {
        let Some(text) = repo.text(&file) else { continue };
        let lines: Vec<&str> = text.lines().take(20).collect();
        let found: Vec<String> = match extension_of(&file) {
            "rs" => lines.iter().filter_map(|line| line.trim().strip_prefix("//!")).map(|line| line.trim().to_string()).collect(),
            "py" => lines
                .iter()
                .find_map(|line| line.trim().strip_prefix("\"\"\"").or_else(|| line.trim().strip_prefix("'''")))
                .map(|first| vec![first.trim().trim_end_matches("\"\"\"").trim().to_string()])
                .unwrap_or_default(),
            "md" => lines.iter().filter(|line| !line.trim().is_empty() && !line.starts_with('#')).take(1).map(|line| line.trim().to_string()).collect(),
            _ => lines.iter().filter_map(|line| line.trim().strip_prefix("//").or_else(|| line.trim().strip_prefix("/**")).or_else(|| line.trim().strip_prefix("* "))).map(|line| line.trim().to_string()).collect(),
        };
        let first = found.into_iter().find(|line| !line.is_empty() && !line.starts_with(['#', '!', '/', '`', '*']) && line.len() > 8);
        if let Some(line) = first {
            let sentence = line.split(". ").next().unwrap_or(&line).trim_end_matches('.').to_string();
            return sentence.chars().take(110).collect();
        }
    }
    String::new()
}

/// The folder the part's code starts at below `root`: its `src` or `app` where it has one.
fn source_root(repo: &Repo, root: &str) -> String {
    let root = root.trim_end_matches('/');
    for folder in ["src", "app"] {
        let candidate = if root.is_empty() || root == "." { folder.to_string() } else { format!("{root}/{folder}") };
        if repo.under(&candidate).next().is_some() {
            return candidate;
        }
    }
    if root == "." { String::new() } else { root.to_string() }
}

/// The children of `folder` in the tracked files: each with its path, whether it is a folder, and its bytes.
fn children(repo: &Repo, folder: &str) -> Vec<(String, bool, u64)> {
    let prefix = if folder.is_empty() { String::new() } else { format!("{folder}/") };
    let mut found: BTreeMap<String, (bool, u64, usize)> = BTreeMap::new();
    for (file, bytes) in repo.layout.files.iter().filter(|(file, _)| file.starts_with(&prefix)) {
        let rest = &file[prefix.len()..];
        let (name, is_folder) = match rest.split_once('/') {
            Some((name, _)) => (name.to_string(), true),
            None => (rest.to_string(), false),
        };
        let entry = found.entry(name).or_insert((is_folder, 0, 0));
        entry.1 += bytes;
        entry.2 += 1;
    }
    found.into_iter().map(|(name, (is_folder, bytes, _))| (format!("{prefix}{name}{}", if is_folder { "/" } else { "" }), is_folder, bytes)).collect()
}

fn stem_of(path: &str) -> String {
    let last = path.trim_end_matches('/').rsplit('/').next().unwrap_or(path);
    if path.ends_with('/') { last.to_string() } else { last.rsplit_once('.').map_or(last, |(stem, _)| stem).to_string() }
}

fn is_code(path: &str) -> bool {
    path.ends_with('/') || ["rs", "py", "js", "jsx", "mjs", "ts", "tsx", "go", "java", "kt", "swift", "rb", "cs", "php", "vue", "svelte", "html"].contains(&extension_of(path))
}

fn is_entry_file(path: &str) -> bool {
    matches!(path.rsplit('/').next().unwrap_or(path), "lib.rs" | "main.rs" | "mod.rs" | "__init__.py" | "index.js" | "index.ts" | "setup.py")
}

/// The pieces of every part.
pub(super) fn pieces(repo: &Repo, det: &mut Detected) -> Vec<Piece> {
    attribute_libs(repo, det);
    let rules = &known().pieces;
    let mut out: Vec<Piece> = Vec::new();
    let mut used: BTreeSet<String> = BTreeSet::new();
    for at in 0..det.parts.len() {
        let part: &Part = &det.parts[at];
        if !matches!(part.wire.kind, SystemPartKind::Entry | SystemPartKind::Service) {
            continue;
        }
        let mut found: Vec<(String, bool, u64)> = Vec::new();
        for path in &part.wire.paths {
            if !path.ends_with('/') {
                found.push((path.clone(), false, repo.size_under(path)));
                continue;
            }
            let folder = source_root(repo, path);
            for child in children(repo, &folder) {
                let name = stem_of(&child.0);
                if child.1 && rules.descend.iter().any(|one| *one == name) {
                    let below = children(repo, child.0.trim_end_matches('/'));
                    if below.len() >= MANY_BELOW {
                        found.extend(below);
                        continue;
                    }
                }
                found.push(child);
            }
        }
        found.retain(|(path, _, _)| is_code(path) && !is_entry_file(path) && !rules.skip.iter().any(|skip| *skip == stem_of(path)) && !path.rsplit('/').next().unwrap_or("").starts_with('.'));
        found.sort_by(|a, b| b.2.cmp(&a.2).then_with(|| a.0.cmp(&b.0)));
        found.truncate(MOST_PIECES);
        found.sort_by(|a, b| a.0.cmp(&b.0));
        for (path, _, _) in found {
            let name = stem_of(&path);
            let id = format!("{}/{}", part.wire.id, slug(&name));
            if !used.insert(id.clone()) {
                continue;
            }
            out.push(Piece { id, name, part: at, summary: what_it_does(repo, &path), paths: vec![path] });
        }
        for lib in &part.libs {
            let module = &repo.layout.modules[*lib];
            let id = format!("{}/{}", part.wire.id, slug(&module.name));
            if used.insert(id.clone()) {
                let path = format!("{}/", module.root);
                out.push(Piece { id, name: module.name.clone(), part: at, summary: what_it_does(repo, &path), paths: vec![path] });
            }
        }
    }
    out
}
