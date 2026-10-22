//! A test map collector that reads imports: for each vitest test file in a Check's
//! directory, the files it reaches inside the repository. `docs/concepts/test-map.md`.
//!
//! Nothing runs, so it is [`LapCost::Nothing`] and every lap is [`LapGrain::File`]. It is
//! blind to what a bundler adds: a `vi.mock` path, a Vite alias not in `tsconfig.json`, a
//! CSS or SVG import.
//!
//! `test_map_static.mjs` loads the `typescript` that resolves from the Check's directory,
//! so a `paths` alias resolves as it does in the editor, and prints one JSON line per
//! test, read here through `ipc::decode`. The script drops nothing it resolves; what is
//! relative, in `node_modules` or outside the repository is decided here, once.

use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};

use adapter_traits::{CollectorSource, Lap, LapCost, LapGrain, NotRecorded, TestCollector, TestsToRecord};
use serde::Deserialize;

const SCRIPT: &str = include_str!("test_map_static.mjs");

/// The import closure of each vitest test file in a Check's directory.
pub struct TypeScriptImports {
    root: PathBuf,
}

impl TypeScriptImports {
    /// `root` is the repository's, and a Check's `dir` is read relative to it.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }
}

impl TestCollector for TypeScriptImports {
    fn name(&self) -> &str {
        "typescript-imports"
    }

    fn source(&self) -> CollectorSource {
        CollectorSource::Static
    }

    fn needs_rebuild(&self) -> bool {
        false
    }

    fn lap_cost(&self) -> LapCost {
        LapCost::Nothing
    }

    /// `dir` is the Check's directory under the repository root; `tests` names test files
    /// relative to the root. A name that is not a file is no lap.
    fn record(&self, _check: &str, dir: &str, tests: &TestsToRecord) -> Result<Vec<Lap>, NotRecorded> {
        let root = self
            .root
            .canonicalize()
            .map_err(|why| NotRecorded(format!("the repository {}: {why}", self.root.display())))?;
        let mut node = crate::background::command("node");
        node.current_dir(root.join(dir))
            .args(["--input-type=module", "-e", SCRIPT, "--"])
            .arg(&root);
        if let TestsToRecord::Named(names) = tests {
            if names.is_empty() {
                return Ok(Vec::new());
            }
            node.args(names.iter().map(|name| root.join(name)));
        }
        let ran = node.output().map_err(|why| NotRecorded(format!("node did not start: {why}")))?;
        if !ran.status.success() {
            return Err(NotRecorded(format!(
                "node read no imports in {dir}: {}",
                String::from_utf8_lossy(&ran.stderr).trim()
            )));
        }
        laps_from(&String::from_utf8_lossy(&ran.stdout), &root, dir)
    }
}

#[derive(Deserialize)]
struct Line {
    test: String,
    files: Vec<String>,
}

/// The script's lines as laps. `root` is the repository's real path, which is what
/// the script printed paths under. A test outside the repository or under
/// `node_modules` is no lap; a file there is no file of the lap's.
pub(crate) fn laps_from(printed: &str, root: &Path, dir: &str) -> Result<Vec<Lap>, NotRecorded> {
    let member = match dir.trim_matches('/') {
        "." => "",
        trimmed => trimmed,
    };
    let mut laps = Vec::new();
    for line in printed.lines().filter(|line| !line.trim().is_empty()) {
        let line: Line = ipc::decode("an import closure line", line.as_bytes())
            .map_err(|why| NotRecorded(why.to_string()))?;
        let Some(test) = within(&line.test, root) else {
            continue;
        };
        let mut files: BTreeSet<String> = line.files.iter().filter_map(|file| within(file, root)).collect();
        files.insert(test.clone());
        laps.push(Lap {
            member: member.to_string(),
            test,
            grain: LapGrain::File,
            files: files.into_iter().collect(),
        });
    }
    Ok(laps)
}

/// `path` relative to `root` and `/`-separated, or nothing where it is outside it or
/// under a `node_modules`.
fn within(path: &str, root: &Path) -> Option<String> {
    let relative = Path::new(path).strip_prefix(root).ok()?;
    let mut parts = Vec::new();
    for part in relative.components() {
        match part {
            Component::Normal(name) if name == "node_modules" => return None,
            Component::Normal(name) => parts.push(name.to_string_lossy().into_owned()),
            _ => return None,
        }
    }
    (!parts.is_empty()).then(|| parts.join("/"))
}
