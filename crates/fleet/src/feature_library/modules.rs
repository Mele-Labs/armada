//! The modules of a repository, read from its build manifests with its top folders as the fallback. Plain file reads, no
//! model and no code graph. `docs/concepts/feature-catalog.md`, *Floor plan*.
//!
//! **A module is the folder of a build manifest** (`Cargo.toml`, `package.json`, `go.mod`, `pyproject.toml`, a Gradle
//! settings file, `pom.xml`, `.csproj`, `Package.swift`), named as the manifest names it. Where a tree holds none, each
//! of its top folders is a module. **The walk is bounded** by folders, by depth and by modules, and skips what other people
//! wrote or a build wrote.

pub(crate) mod json_scan;
mod manifests;

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::Path;

use super::mapping::owns;

/// Folders that hold what other people wrote or what a build wrote.
pub(crate) const SKIPPED: &[&str] = &["target", "node_modules", "dist", "build", "out", "vendor", "venv", "__pycache__", "bin", "obj", "Pods", "coverage"];

/// The most folders looked at, so a very large tree costs a bounded read.
pub(super) const MOST_FOLDERS: usize = 3000;
/// How deep under the root a folder is looked at.
pub(super) const DEEPEST: usize = 8;
/// The most modules kept; the walk stops there, shallowest first.
const MOST_MODULES: usize = 400;
/// The largest manifest read, in bytes.
const LARGEST_MANIFEST: u64 = 512 * 1024;

/// Which manifest named a module.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    Cargo,
    Npm,
    Go,
    Python,
    Gradle,
    Maven,
    Dotnet,
    Swift,
    Folder,
}

impl Kind {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Kind::Cargo => "cargo",
            Kind::Npm => "npm",
            Kind::Go => "go",
            Kind::Python => "python",
            Kind::Gradle => "gradle",
            Kind::Maven => "maven",
            Kind::Dotnet => "dotnet",
            Kind::Swift => "swift",
            Kind::Folder => "folder",
        }
    }
}

/// What a manifest says a module depends on, before it is matched to a module of the repository.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Dep {
    Name(String),
    /// A repository-relative path, in or under a module's folder.
    Path(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Module {
    pub(crate) name: String,
    /// Repository-relative, empty for the repository's own root.
    pub(crate) root: String,
    pub(crate) kind: Kind,
    pub(crate) deps: Vec<Dep>,
    /// A `.proto` or OpenAPI file lies in it.
    pub(crate) spec_files: bool,
}

/// A repository's modules, and which depend on which, as indexes into `modules`.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Layout {
    pub(crate) modules: Vec<Module>,
    pub(crate) uses: BTreeSet<(usize, usize)>,
    /// Its tracked files and their sizes, where the floor plan read them (`floor_files`); empty from [`read`].
    pub(crate) files: Vec<(String, u64)>,
}

impl Layout {
    /// The module a repository-relative path is in: the one with the deepest root holding it.
    pub(crate) fn holding(&self, path: &str) -> Option<usize> {
        self.modules
            .iter()
            .enumerate()
            .filter(|(_, module)| module.root.is_empty() || owns(&module.root, path))
            .max_by_key(|(_, module)| module.root.len())
            .map(|(at, _)| at)
    }
}

/// Whether a file is a wire or API description: a protobuf file or an OpenAPI or Swagger document.
fn describes_an_api(file: &str) -> bool {
    let lower = file.to_ascii_lowercase();
    lower.ends_with(".proto")
        || ["openapi", "swagger"].iter().any(|stem| lower.strip_prefix(stem).is_some_and(|rest| matches!(rest, ".yaml" | ".yml" | ".json")))
}

/// The layout of the repository at `root`. **Blocking**: the caller runs it off the main thread.
pub(crate) fn read(root: &Path) -> Layout {
    let mut found: Vec<Module> = Vec::new();
    let mut spec_dirs: Vec<String> = Vec::new();
    let mut top: Vec<String> = Vec::new();
    let mut queue: VecDeque<(String, usize)> = VecDeque::from([(String::new(), 0)]);
    let mut looked = 0;
    while let Some((rel, depth)) = queue.pop_front() {
        looked += 1;
        if looked > MOST_FOLDERS || found.len() >= MOST_MODULES {
            break;
        }
        let abs = if rel.is_empty() { root.to_path_buf() } else { root.join(&rel) };
        let Ok(listing) = std::fs::read_dir(&abs) else { continue };
        let (mut files, mut folders) = (Vec::new(), Vec::new());
        for entry in listing.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            match entry.file_type() {
                Ok(kind) if kind.is_dir() => {
                    if !name.starts_with('.') && !SKIPPED.contains(&name.as_str()) {
                        folders.push(name);
                    }
                }
                Ok(kind) if kind.is_file() => files.push(name),
                _ => {}
            }
        }
        files.sort();
        folders.sort();
        if files.iter().any(|file| describes_an_api(file)) {
            spec_dirs.push(rel.clone());
        }
        let here = manifests::Here { abs: &abs, rel: &rel, files: &files };
        for file in files.iter().filter(|file| manifests::is_manifest(file)) {
            let readable = std::fs::metadata(abs.join(file)).is_ok_and(|meta| meta.len() <= LARGEST_MANIFEST);
            if let Some(text) = readable.then(|| std::fs::read_to_string(abs.join(file)).ok()).flatten() {
                found.extend(manifests::read(&here, file, &text));
            }
        }
        for name in folders {
            let below = if rel.is_empty() { name.clone() } else { format!("{rel}/{name}") };
            if depth == 0 {
                top.push(below.clone());
            }
            if depth < DEEPEST {
                queue.push_back((below, depth + 1));
            }
        }
    }
    found.truncate(MOST_MODULES);
    if found.is_empty() {
        found = top.into_iter().map(|folder| Module { name: folder.clone(), root: folder, kind: Kind::Folder, deps: Vec::new(), spec_files: false }).collect();
    }
    found.sort_by(|a, b| a.root.cmp(&b.root).then_with(|| a.name.cmp(&b.name)));
    let mut layout = Layout { modules: found, uses: BTreeSet::new(), files: Vec::new() };
    for dir in &spec_dirs {
        if let Some(at) = layout.holding(dir) {
            layout.modules[at].spec_files = true;
        }
    }
    resolve(&mut layout);
    layout
}

/// A name as dependency tools compare it: lower case, `_` and `.` as `-`.
fn plain(name: &str) -> String {
    name.to_ascii_lowercase().replace(['_', '.'], "-")
}

/// Match each module's dependencies to the modules of the repository, then make every name unique.
fn resolve(layout: &mut Layout) {
    let mut by_name: BTreeMap<String, usize> = BTreeMap::new();
    for (at, module) in layout.modules.iter().enumerate() {
        by_name.entry(plain(&module.name)).or_insert(at);
    }
    let mut uses = BTreeSet::new();
    for (from, module) in layout.modules.iter().enumerate() {
        for dep in &module.deps {
            let to = match dep {
                Dep::Name(name) => by_name.get(&plain(name)).copied(),
                Dep::Path(path) => layout.holding(path),
            };
            if let Some(to) = to.filter(|to| *to != from) {
                uses.insert((from, to));
            }
        }
    }
    layout.uses = uses;
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for module in &layout.modules {
        *counts.entry(module.name.clone()).or_default() += 1;
    }
    for module in &mut layout.modules {
        if counts.get(&module.name).is_some_and(|count| *count > 1) {
            module.name = if module.root.is_empty() { ".".to_string() } else { module.root.clone() };
        }
    }
}
