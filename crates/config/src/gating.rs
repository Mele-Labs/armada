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

/// The directory of the nearest workspace above `path`, or empty for the root.
fn owner<'a>(workspaces: &'a [Manifest], path: &str) -> &'a str {
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
