//! What differs when an `armada.yml` is a workspace's and not the root's.
//!
//! **The file does not say which it is.** Where a file sits in the repository
//! is the loader's to know, so [`Manifest::parse_workspace`] takes the
//! directory and the root's Manifest beside the text, and every difference
//! below follows from those two arguments.
//!
//! | In a workspace file | Rule |
//! |---|---|
//! | `checks.<name>.when` | relative to the workspace's directory |
//! | `depends_on` | repository-relative, whatever file it is in |
//! | `setup.requires` | may also name a Command the root declares |
//! | `setup.worktrees` | refused: it is read from the root at every lease |
//!
//! The reasoning is in `docs/concepts/manifest.md`, *Workspace gating*.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use super::declared::Command;
use super::{read, Check, Manifest};
use crate::error::{LoadError, Refusal};

/// Where a file sits: the root, or a directory below it with the root's
/// Manifest to resolve against.
pub(super) struct Placed<'a> {
    pub(super) dir: &'a str,
    root: Option<&'a Manifest>,
}

impl<'a> Placed<'a> {
    pub(super) fn at_the_root() -> Placed<'static> {
        Placed {
            dir: "",
            root: None,
        }
    }

    pub(super) fn is_a_workspace(&self) -> bool {
        self.root.is_some()
    }

    /// The Commands and servers a `setup.requires` entry may name: the file's
    /// own, and the root's where the file does not declare the name. **The
    /// file's own wins**, so a workspace's `bootstrap` is never the root's.
    pub(super) fn requirable(
        &self,
        commands: &BTreeMap<String, Command>,
        serves: &BTreeSet<String>,
    ) -> (BTreeMap<String, Command>, BTreeSet<String>) {
        let mut all = commands.clone();
        let mut servers = serves.clone();
        if let Some(root) = self.root {
            for (name, command) in &root.commands {
                all.entry(name.clone()).or_insert_with(|| command.clone());
            }
            for name in root.servers.keys() {
                if !all.contains_key(name) {
                    servers.insert(name.clone());
                }
            }
        }
        (all, servers)
    }
}

impl Manifest {
    /// Validate a workspace's `armada.yml` already in hand. `dir` is the
    /// workspace's directory relative to the repository root, `/`-separated
    /// and never empty.
    pub fn parse_workspace(
        path: &Path,
        dir: &str,
        text: &str,
        root: &Manifest,
    ) -> Result<Manifest, LoadError> {
        Manifest::parse_placed(
            path,
            text,
            &Placed {
                dir,
                root: Some(root),
            },
        )
    }

    /// Read a workspace's `armada.yml`, as [`Manifest::parse_workspace`].
    pub fn load_workspace(path: &Path, dir: &str, root: &Manifest) -> Result<Manifest, LoadError> {
        let text = std::fs::read_to_string(path).map_err(|cause| LoadError::Unreadable {
            path: path.to_path_buf(),
            cause,
        })?;
        Manifest::parse_workspace(path, dir, &text, root)
    }

    /// The directory this Manifest owns paths under, relative to the
    /// repository root. **Empty for the root.**
    pub fn dir(&self) -> &str {
        &self.dir
    }

    /// The globs a change must match for this Manifest to gate it without
    /// touching a path it owns. Repository-relative. Empty where none are
    /// written.
    pub fn depends_on(&self) -> &[core_model::PathPattern] {
        self.depends_on
            .as_ref()
            .map_or(&[], |covers| covers.patterns())
    }

    /// Whether `check` covers `changed`, which are repository-relative.
    ///
    /// **A workspace's `when` is read from its own directory**, so only the
    /// changed paths below it are shown to the patterns, with the directory
    /// taken off. A Check with no `when` covers everything, as in the root.
    pub fn reaches(&self, check: &Check, changed: &[String]) -> bool {
        match self.dir.is_empty() {
            true => check.covers(changed),
            false => check.covers(&self.within(changed)),
        }
    }

    /// The changed paths below this Manifest's directory, relative to it.
    pub(crate) fn within(&self, changed: &[String]) -> Vec<String> {
        let prefix = format!("{}/", self.dir);
        changed
            .iter()
            .filter_map(|path| path.strip_prefix(&prefix).map(str::to_string))
            .collect()
    }

    pub(super) fn parse_placed(
        path: &Path,
        text: &str,
        placed: &Placed,
    ) -> Result<Manifest, LoadError> {
        let root: serde_yaml_ng::Value =
            serde_yaml_ng::from_str(text).map_err(|cause| LoadError::NotYaml {
                path: path.to_path_buf(),
                cause,
            })?;
        let mut out: Vec<Refusal> = Vec::new();
        match read(path, &root, placed, &mut out) {
            Some(manifest) if out.is_empty() => Ok(manifest),
            _ => Err(LoadError::Refused {
                path: path.to_path_buf(),
                refusals: out,
            }),
        }
    }
}
