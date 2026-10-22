//! What Fleet's test map adds to an agent's `armada check <name> --changed`.
//! `docs/concepts/test-map.md`, *What reads it*.
//!
//! **A sharpening, never a dependency.** No Fleet, a Fleet that does not answer
//! or a map that holds no edge for the repository runs the Check exactly as it ran
//! before the map, and says nothing more. Only an agent's reading asks it: the
//! merge line and CI run what the Manifest says, and a run the map narrowed
//! gates nothing.

use std::collections::BTreeMap;
use std::path::Path;

use checks_runner::touched;
use core_model::{EdgeGrain, Narrowing};
use fleet::runtime;
use ipc::TestsReaching;

use crate::loopback::Loopback;
use crate::manifests::Manifests;

/// How the map is read: the root Manifest's id and the changed paths as the
/// repository spells them, to Fleet's answer or `None`. A parameter so a test
/// reads a map it wrote, and never the machine's Fleet.
pub type ReadMap<'a> = &'a (dyn Fn(&str, &[String]) -> Option<TestsReaching> + Sync);

/// What the map said about one run, for the lines printed around it. Empty
/// where it said nothing or was not asked.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Mapped {
    /// Tests the map named and no changed file touched, in the order run.
    pub added: Vec<String>,
    /// Of `added`, those read only from stale edges.
    pub stale: Vec<String>,
    /// Changed files the Check covers that the map has no edge for and that
    /// are not tests themselves, as the repository spells them.
    pub unreached: Vec<String>,
}

impl Mapped {
    /// The lines said before the run's output: what the map added, and which of
    /// that it read from stale edges.
    pub fn before(&self) -> Vec<String> {
        let mut lines = Vec::new();
        if !self.added.is_empty() {
            lines.push(format!("the test map added {}", self.added.join(" ")));
        }
        if !self.stale.is_empty() {
            lines.push(format!("stale in the test map: {}", self.stale.join(" ")));
        }
        lines
    }

    /// The lines said after the verdict, one per changed file no test reaches.
    pub fn after(&self) -> Vec<String> {
        self.unreached.iter().map(|file| format!("no test reaches {file}")).collect()
    }
}

/// One Check's share of an answer.
#[derive(Debug, Default)]
pub struct ForCheck {
    /// `(member, name)` for each test, as `touched_run_with` takes them.
    pub tests: Vec<(String, String)>,
    /// Each name with whether every edge naming it is stale.
    stale: BTreeMap<String, bool>,
}

impl ForCheck {
    /// The tests the map named for the Check `key`. A file-grain edge names a
    /// test file, which is read for its tests as a changed test file is, from
    /// `root`, the Check's manifest directory, which is `dir` below the
    /// repository.
    pub fn of(answer: &TestsReaching, key: &str, root: &Path, dir: &str, narrowing: &Narrowing) -> ForCheck {
        let mut found = ForCheck::default();
        for group in answer.checks.iter().filter(|group| group.check == key) {
            for test in &group.tests {
                let names = match test.grain.domain() {
                    EdgeGrain::Test => vec![(group.member.clone(), test.test.clone())],
                    EdgeGrain::File => {
                        let file = within(dir, &test.test);
                        let read = touched(root, narrowing, std::slice::from_ref(&file));
                        let member = read.members.first().cloned().unwrap_or_default();
                        read.names.into_iter().map(|name| (member.clone(), name)).collect()
                    }
                };
                for (member, name) in names {
                    let all_stale = found.stale.entry(name.clone()).or_insert(true);
                    *all_stale &= test.stale;
                    if !found.tests.contains(&(member.clone(), name.clone())) {
                        found.tests.push((member, name));
                    }
                }
            }
        }
        found
    }

    /// [`Mapped`] for a run that added `added`.
    pub fn said(&self, added: Vec<String>, unreached: Vec<String>) -> Mapped {
        let stale = added
            .iter()
            .filter(|name| self.stale.get(*name).copied().unwrap_or(false))
            .cloned()
            .collect();
        Mapped { added, stale, unreached }
    }
}

/// The map read for one Check's `--changed` run: its tests for the Check `key`,
/// and the changed files of `paths` it has no edge for, that the Check covers
/// and that today's reading finds no test in. `None` where the Check cannot run
/// a test by name, so nothing was asked, or the map gave no answer worth one.
pub fn read_for(
    read: ReadMap<'_>,
    manifests: &Manifests,
    manifest: &config::Manifest,
    key: &str,
    check: &config::Check,
    root: &Path,
    paths: &[String],
) -> Option<(ForCheck, Vec<String>)> {
    check.one_test()?;
    let narrowing = check.narrow().filter(|narrowing| narrowing.under().is_some())?;
    let answer = read(manifests.root().id().as_str(), paths).filter(|answer| answer.holds_edges)?;
    let found = ForCheck::of(&answer, key, root, manifest.dir(), narrowing);
    let unreached = answer
        .unreached
        .into_iter()
        .filter(|path| {
            let owned = manifests.owned(manifest, std::slice::from_ref(path));
            !owned.is_empty() && check.covers(&owned) && touched(root, narrowing, &owned).is_empty()
        })
        .collect();
    Some((found, unreached))
}

/// A repository path as the manifest in `dir` spells it.
pub fn within(dir: &str, path: &str) -> String {
    let path = path.trim_start_matches('/');
    match dir.is_empty() {
        true => path.to_string(),
        false => path
            .strip_prefix(dir)
            .and_then(|rest| rest.strip_prefix('/'))
            .unwrap_or(path)
            .to_string(),
    }
}

/// The longest `paths=` value one request carries, encoded. Far under what a
/// listener takes in a request line, so a large change goes over several.
const PATHS_PER_REQUEST: usize = 4096;

/// The map, read from the Fleet on this machine. `None` where no Fleet is up,
/// where any request goes unanswered or is refused, or where the answer will
/// not decode: the Check then runs as it did without one.
pub fn from_fleet(manifest_id: &str, paths: &[String]) -> Option<TestsReaching> {
    let at = runtime::machine_path().ok()?;
    let fleet = Loopback::at(crate::mcp::listening(runtime::read(&at), &at).ok()?);
    let mut whole = TestsReaching::default();
    for chunk in requests(paths) {
        let path = format!(
            "/test_map/reaching?manifest_id={}&paths={}",
            ipc::door::encoded(manifest_id),
            ipc::door::encoded(&chunk.join("\n"))
        );
        let answer = fleet.get(&path).ok()?;
        if answer.status != 200 {
            return None;
        }
        let part: TestsReaching = ipc::decode("the tests reaching the changed paths", &answer.body).ok()?;
        whole.checks.extend(part.checks);
        whole.unreached.extend(part.unreached);
    }
    Some(whole)
}

/// `paths` in runs whose encoded value stays under [`PATHS_PER_REQUEST`]. A
/// single path longer than that goes alone.
pub fn requests(paths: &[String]) -> Vec<Vec<String>> {
    let mut runs: Vec<Vec<String>> = Vec::new();
    let mut length = 0;
    for path in paths {
        let cost = ipc::door::encoded(path).len() + 3;
        match runs.last_mut() {
            Some(run) if length + cost <= PATHS_PER_REQUEST => {
                run.push(path.clone());
                length += cost;
            }
            _ => {
                runs.push(vec![path.clone()]);
                length = cost;
            }
        }
    }
    runs
}
