//! One repository as detection reads it: its modules, the source files whose text evidence comes from, the compose services
//! and the Kit servers it declares. **Read from the checkout, never run.** A file that does not read says nothing.

use std::path::{Path, PathBuf};

use crate::feature_library::{layout_with_files, ModuleLayout};

/// The most source files read, and the largest one.
const MOST_SOURCES: usize = 2500;
const LARGEST_SOURCE: u64 = 200_000;
/// The most files listed where git lists none.
const MOST_WALKED: usize = 6000;

const SOURCE_EXTS: &[&str] = &["rs", "py", "js", "jsx", "mjs", "ts", "tsx", "go", "java", "kt", "swift", "rb", "cs", "php", "vue", "svelte"];
const NOT_CODE_FOLDERS: &[&str] = &["tests", "test", "__tests__", "e2e", "fixtures", "mocks", "__mocks__", "stories", "spec", "testdata", "snapshots"];

/// A source file and its text.
pub(super) struct Source {
    pub(super) path: String,
    pub(super) text: String,
}

/// A server a Kit manifest declares.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct KitServer {
    pub(super) name: String,
    pub(super) serve: String,
    pub(super) dir: Option<String>,
    pub(super) links: Vec<String>,
}

/// A service a compose file declares.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct Composed {
    pub(super) name: String,
    pub(super) image: Option<String>,
    pub(super) built: bool,
    /// The published ports.
    pub(super) ports: Vec<u16>,
    pub(super) file: String,
}

pub(super) struct Repo {
    pub(super) manifest: String,
    pub(super) root: PathBuf,
    pub(super) layout: ModuleLayout,
    pub(super) sources: Vec<Source>,
    pub(super) kit: Vec<KitServer>,
    pub(super) composed: Vec<Composed>,
}

pub(super) fn extension_of(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path).rsplit_once('.').map_or("", |(_, ext)| ext)
}

/// Whether a path is a test, a fixture or a mock: code that proves, not code that runs.
pub(super) fn is_proof(path: &str) -> bool {
    let name = path.rsplit('/').next().unwrap_or(path);
    path.split('/').any(|segment| NOT_CODE_FOLDERS.contains(&segment))
        || name.starts_with("test_")
        || name.contains(".test.")
        || name.contains(".spec.")
        || name.contains("_test.")
        || name.ends_with("_tests.rs")
        || name == "tests.rs"
        || name == "conftest.py"
        || name.starts_with('_') && name.contains("test")
}

/// Every file under `root` as a tracked listing would give it, for a folder that is no repository.
fn walked(root: &Path) -> Vec<(String, u64)> {
    let mut found = Vec::new();
    let mut queue = vec![(PathBuf::new(), 0usize)];
    while let Some((rel, depth)) = queue.pop() {
        let Ok(listing) = std::fs::read_dir(root.join(&rel)) else { continue };
        for entry in listing.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            let Ok(kind) = entry.file_type() else { continue };
            let below = rel.join(&name);
            if kind.is_dir() {
                if !name.starts_with('.') && depth < 8 && !crate::feature_library::SKIPPED_FOLDERS.contains(&name.as_str()) {
                    queue.push((below, depth + 1));
                }
            } else if kind.is_file() && found.len() < MOST_WALKED {
                found.push((below.to_string_lossy().into_owned(), entry.metadata().map_or(0, |meta| meta.len())));
            }
        }
    }
    found.sort();
    found
}

/// The text of `rel` under `root`, where it reads and is no larger than `limit`.
pub(super) fn text_at(root: &Path, rel: &str, limit: u64) -> Option<String> {
    let abs = root.join(rel);
    std::fs::metadata(&abs).ok().filter(|meta| meta.len() <= limit)?;
    std::fs::read_to_string(abs).ok()
}

impl Repo {
    /// **Blocking**: the caller runs it off the main thread.
    pub(super) fn read(manifest: &str, root: &Path, kit: Vec<KitServer>) -> Repo {
        let mut layout = layout_with_files(root);
        if layout.files.is_empty() {
            layout.files = walked(root);
        }
        let mut sources = Vec::new();
        for (path, bytes) in &layout.files {
            if sources.len() >= MOST_SOURCES {
                break;
            }
            if *bytes <= LARGEST_SOURCE && SOURCE_EXTS.contains(&extension_of(path)) && !is_proof(path) {
                if let Some(text) = text_at(root, path, LARGEST_SOURCE) {
                    sources.push(Source { path: path.clone(), text });
                }
            }
        }
        let composed = layout
            .files
            .iter()
            .filter(|(path, _)| is_compose_file(path))
            .filter_map(|(path, _)| text_at(root, path, 200_000).map(|text| composed_in(path, &text)))
            .flatten()
            .collect();
        Repo { manifest: manifest.to_string(), root: root.to_path_buf(), layout, sources, kit, composed }
    }

    pub(super) fn has(&self, path: &str) -> bool {
        self.layout.files.iter().any(|(file, _)| file == path)
    }

    /// The text of a tracked or untracked file of the repository.
    pub(super) fn text(&self, rel: &str) -> Option<String> {
        text_at(&self.root, rel, 512 * 1024)
    }

    /// The tracked files at or under `prefix`, a folder with or without its `/`.
    pub(super) fn under<'a>(&'a self, prefix: &'a str) -> impl Iterator<Item = &'a str> {
        let prefix = prefix.trim_end_matches('/');
        self.layout.files.iter().map(|(file, _)| file.as_str()).filter(move |file| prefix.is_empty() || crate::feature_library::mapping::owns(prefix, file))
    }

    pub(super) fn size_under(&self, prefix: &str) -> u64 {
        let prefix = prefix.trim_end_matches('/');
        self.layout.files.iter().filter(|(file, _)| prefix.is_empty() || crate::feature_library::mapping::owns(prefix, file)).map(|(_, bytes)| *bytes).sum()
    }
}

fn is_compose_file(path: &str) -> bool {
    let name = path.rsplit('/').next().unwrap_or(path);
    name.contains("compose") && (name.ends_with(".yml") || name.ends_with(".yaml"))
}

/// The services of a compose file, read by indent. A shape this does not follow gives what was read so far.
fn composed_in(file: &str, text: &str) -> Vec<Composed> {
    let lines: Vec<(usize, &str)> = text
        .lines()
        .map(|line| line.split(" #").next().unwrap_or(line))
        .filter(|line| !line.trim().is_empty() && !line.trim_start().starts_with('#'))
        .map(|line| (line.len() - line.trim_start().len(), line.trim()))
        .collect();
    let Some(start) = lines.iter().position(|(indent, line)| *indent == 0 && *line == "services:") else { return Vec::new() };
    let body: Vec<&(usize, &str)> = lines[start + 1..].iter().take_while(|(indent, _)| *indent > 0).collect();
    let Some(service_indent) = body.first().map(|(indent, _)| *indent) else { return Vec::new() };
    let mut out: Vec<Composed> = Vec::new();
    let mut in_ports = false;
    for (indent, line) in body.into_iter().copied() {
        if indent == service_indent {
            out.push(Composed { name: line.trim_end_matches(':').trim().to_string(), file: file.to_string(), ..Composed::default() });
            in_ports = false;
            continue;
        }
        let Some(service) = out.last_mut() else { continue };
        if indent == service_indent + 2 || indent <= service_indent + 4 && !line.starts_with('-') && !in_ports {
            in_ports = false;
        }
        if let Some(image) = line.strip_prefix("image:") {
            service.image = Some(image.trim().trim_matches(['"', '\'']).to_string());
        } else if line.starts_with("build:") {
            service.built = true;
        } else if let Some(rest) = line.strip_prefix("ports:") {
            in_ports = true;
            service.ports.extend(ports_in(rest));
        } else if in_ports && line.starts_with('-') {
            service.ports.extend(ports_in(line));
        } else if !line.starts_with('-') && !line.starts_with("target:") && !line.starts_with("published:") {
            in_ports = false;
        }
    }
    out
}

/// The published port of each entry in `text`: `"8080:3000"` is 8080, `3000` is 3000.
fn ports_in(text: &str) -> Vec<u16> {
    text.split(|c: char| c.is_whitespace() || matches!(c, ',' | '[' | ']' | '"' | '\''))
        .filter(|entry| !entry.is_empty() && *entry != "-")
        .filter_map(|entry| entry.split('/').next().unwrap_or(entry).split(':').find_map(|part| part.parse::<u16>().ok()))
        .collect()
}
