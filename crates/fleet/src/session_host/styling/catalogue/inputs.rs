//! Which files a catalogue is read from, and their fingerprint. **Git lists them**, tracked and
//! untracked but not ignored, so a build's output and other people's code stay out without a walk;
//! a folder git does not know is walked instead, within a bound.

use std::path::Path;

use sha2::{Digest, Sha256};

/// Where a repository keeps its house patterns, from its root.
pub(crate) const PATTERNS_FILE: &str = ".armada/catalogue/patterns.html";

/// Folders no listing enters, beside every hidden one.
const SKIPPED: [&str; 14] = [
    "node_modules", "target", "vendor", "third_party", "bower_components", "venv", "__pycache__", "Pods", "dist", "build", "out",
    "coverage", "storybook-static", "fixtures",
];
const SHEET_EXTENSIONS: [&str; 3] = ["css", "scss", "less"];
const COMPONENT_EXTENSIONS: [&str; 4] = ["tsx", "jsx", "vue", "svelte"];
/// What a file that is not a component carries in its name.
const NOT_COMPONENTS: [&str; 5] = [".stories.", ".test.", ".spec.", ".story.", ".fixture."];
/// Most of each kind read, so a huge checkout costs a bounded read.
const SHEETS_AT_MOST: usize = 600;
const COMPONENTS_AT_MOST: usize = 2_000;
/// Most entries a walk visits where git cannot list.
const VISITS_AT_MOST: usize = 20_000;

/// The files a catalogue reads, from the checkout's root, each list sorted.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Inputs {
    pub sheets: Vec<String>,
    pub components: Vec<String>,
    pub patterns: Option<String>,
}

pub(crate) fn listed(root: &Path) -> Inputs {
    let paths = git_listed(root).unwrap_or_else(|| walked(root));
    let mut inputs = Inputs::default();
    for path in paths {
        if path == PATTERNS_FILE {
            inputs.patterns = Some(path);
            continue;
        }
        if path.split('/').any(|part| part.starts_with('.') || SKIPPED.contains(&part)) {
            continue;
        }
        let name = path.rsplit('/').next().unwrap_or_default();
        let ext = name.rsplit_once('.').map(|(_, ext)| ext.to_ascii_lowercase()).unwrap_or_default();
        if SHEET_EXTENSIONS.contains(&ext.as_str()) && !name.contains(".min.") {
            inputs.sheets.push(path);
        } else if COMPONENT_EXTENSIONS.contains(&ext.as_str()) && is_component_name(name) {
            inputs.components.push(path);
        }
    }
    inputs.sheets.sort();
    inputs.sheets.truncate(SHEETS_AT_MOST);
    inputs.components.sort();
    inputs.components.truncate(COMPONENTS_AT_MOST);
    inputs
}

/// `Kbd.tsx` is a component's file; `index.tsx`, `Kbd.stories.tsx` and `use-thing.tsx` are not.
fn is_component_name(name: &str) -> bool {
    name.chars().next().is_some_and(|first| first.is_ascii_uppercase()) && !NOT_COMPONENTS.iter().any(|mark| name.contains(mark))
}

fn git_listed(root: &Path) -> Option<Vec<String>> {
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["--no-optional-locks", "ls-files", "-z", "--cached", "--others", "--exclude-standard"])
        .output()
        .ok()
        .filter(|out| out.status.success())?;
    let text = String::from_utf8_lossy(&out.stdout);
    let mut paths: Vec<String> = text.split('\0').filter(|one| !one.is_empty()).map(str::to_string).collect();
    paths.dedup();
    Some(paths)
}

fn walked(root: &Path) -> Vec<String> {
    let mut found = Vec::new();
    let mut open = vec![root.to_path_buf()];
    let mut visits = 0;
    while let Some(dir) = open.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else { continue };
        for entry in entries.flatten() {
            visits += 1;
            if visits > VISITS_AT_MOST {
                return found;
            }
            let path = entry.path();
            let Ok(rel) = path.strip_prefix(root) else { continue };
            let rel = rel.to_string_lossy().into_owned();
            let Ok(kind) = entry.file_type() else { continue };
            let name = entry.file_name().to_string_lossy().into_owned();
            if kind.is_dir() {
                let pattern_dir = PATTERNS_FILE.starts_with(&format!("{rel}/"));
                if pattern_dir || !(name.starts_with('.') || SKIPPED.contains(&name.as_str())) {
                    open.push(path);
                }
            } else if kind.is_file() {
                found.push(rel);
            }
        }
    }
    found
}

/// What changes when an input does: each path with its size and modified time, digested.
pub(crate) fn fingerprint(root: &Path, inputs: &Inputs) -> String {
    let mut digest = Sha256::new();
    for path in inputs.sheets.iter().chain(inputs.components.iter()).chain(inputs.patterns.iter()) {
        digest.update(path.as_bytes());
        digest.update([0]);
        if let Ok(meta) = std::fs::metadata(root.join(path)) {
            digest.update(meta.len().to_le_bytes());
            let modified = meta.modified().ok().and_then(|at| at.duration_since(std::time::UNIX_EPOCH).ok()).map_or(0, |at| at.as_nanos());
            digest.update(modified.to_le_bytes());
        }
    }
    digest.finalize().iter().take(16).map(|byte| format!("{byte:02x}")).collect()
}
