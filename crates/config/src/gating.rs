//! Which manifests gate a change, and why each does.
//!
//! **Pure, over Manifests already loaded.** Finding which directories hold an
//! `armada.yml` is a walk of the checkout and belongs to the caller; this takes
//! the result and a list of changed paths and answers nothing else.
//!
//! A manifest gates a change when the diff touches a path it **owns**, or a
//! path matching one of its `depends_on` globs. The nearest `armada.yml` up the
//! tree owns a path, and the root owns what no workspace claims. `depends_on`
//! is **not transitive**: what `a` depends on is what `a` wrote, so a manifest's
//! reach can be read from the manifest alone. `docs/concepts/manifest.md`,
//! *Workspace gating*.
//!
//! An empty `changed` gates nothing, where a Check with no `when` run against
//! it still reads as always. Which of the two a Job wants is the caller's.

use core_model::RunsAt;

use crate::manifest::Manifest;

/// One manifest that gates a change.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Gate {
    /// The manifest's directory, relative to the repository root. Empty for
    /// the root.
    pub dir: String,
    pub why: GateWhy,
}

/// Why a manifest gates.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GateWhy {
    /// The diff touches a path it owns.
    Owns,
    /// The diff touches no path it owns, and matches this `depends_on` glob.
    /// Ownership is reported first where both hold.
    DependsOn(String),
}

/// Every manifest `changed` gates, **the root first and then the workspaces in
/// the order given**. Each appears once.
pub fn gating(root: &Manifest, workspaces: &[Manifest], changed: &[String]) -> Vec<Gate> {
    let owned = |dir: &str| changed.iter().any(|path| owner(workspaces, path) == dir);
    std::iter::once(root)
        .chain(workspaces)
        .filter_map(|manifest| {
            let why = match owned(manifest.dir()) {
                true => GateWhy::Owns,
                false => depended_on(manifest, changed)?,
            };
            Some(Gate {
                dir: manifest.dir().to_string(),
                why,
            })
        })
        .collect()
}

/// Every Check `changed` hits, as the manifest that declares it and its name: the root's, then each
/// workspace's in the order its manifest writes them.
///
/// **A manifest is asked only about the paths it owns**, relative to its own directory, so a root
/// Check never sees a workspace's path. With no workspace it is every Check whose `when` covers
/// `changed`. `armada covers` and the pre-PR Checks read this one answer.
pub fn reaching<'a>(
    root: &'a Manifest,
    workspaces: &'a [Manifest],
    changed: &[String],
) -> Vec<(&'a Manifest, &'a str)> {
    let hit = |manifest: &'a Manifest, paths: &[String]| -> Vec<(&'a Manifest, &'a str)> {
        manifest
            .checks_as_written()
            .iter()
            .filter(|name| {
                manifest
                    .check(name)
                    .is_some_and(|check| check.gates() && check.covers(paths))
            })
            .map(|name| (manifest, name.as_str()))
            .collect()
    };
    if workspaces.is_empty() {
        return hit(root, changed);
    }
    gating(root, workspaces, changed)
        .into_iter()
        .filter_map(|gate| {
            std::iter::once(root)
                .chain(workspaces)
                .find(|one| one.dir() == gate.dir)
        })
        .flat_map(|manifest| {
            let dir = manifest.dir();
            let owned: Vec<String> = changed
                .iter()
                .filter(|path| owner(workspaces, path) == dir)
                .map(|path| {
                    let path = path.trim_start_matches('/');
                    path.strip_prefix(dir)
                        .map_or(path, |rest| rest.trim_start_matches('/'))
                        .to_string()
                })
                .collect();
            hit(manifest, &owned)
        })
        .collect()
}

/// The Checks declared `runs_at: pre_pr` that `changed` reaches, which a pull request about to open
/// waits on. **Gated as [`reaching`] gates**: no new rule of when a workspace's Check applies.
pub fn pre_pr_checks<'a>(
    root: &'a Manifest,
    workspaces: &'a [Manifest],
    changed: &[String],
) -> Vec<(&'a Manifest, &'a str)> {
    reaching(root, workspaces, changed)
        .into_iter()
        .filter(|(manifest, name)| {
            manifest
                .check(name)
                .is_some_and(|check| check.runs_at() == RunsAt::PrePr)
        })
        .collect()
}

/// The directory of the nearest workspace above `path`, or empty for the root.
pub fn owner<'a>(workspaces: &'a [Manifest], path: &str) -> &'a str {
    workspaces
        .iter()
        .map(Manifest::dir)
        .filter(|dir| {
            path.trim_start_matches('/')
                .strip_prefix(dir)
                .is_some_and(|rest| rest.starts_with('/'))
        })
        .max_by_key(|dir| dir.len())
        .unwrap_or("")
}

fn depended_on(manifest: &Manifest, changed: &[String]) -> Option<GateWhy> {
    manifest
        .depends_on()
        .iter()
        .find(|glob| changed.iter().any(|path| glob.matches(path)))
        .map(|glob| GateWhy::DependsOn(glob.as_str().to_string()))
}
