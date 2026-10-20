//! The root Manifest and the workspace Manifests under it, read together for
//! `armada covers`, `check` and `run`, and the merge line.
//!
//! A repository with no workspace `armada.yml` is the root alone, and every
//! answer here is then what it was before workspaces existed.
//! `docs/concepts/manifest.md`, *Workspace gating*.

use std::path::{Path, PathBuf};

use config::Manifest;

use crate::declared::NotDeclared;
use crate::setup::MANIFEST;

pub struct Manifests {
    root: Manifest,
    workspaces: Vec<Manifest>,
}

/// A Check or Command's key: its name at the root, `<dir>:<name>` in a
/// workspace. The same spelling `ResolvedCheck::key` gives a Job's rows.
pub fn key(dir: &str, name: &str) -> String {
    match dir.is_empty() {
        true => name.to_string(),
        false => format!("{dir}:{name}"),
    }
}

/// The workspace directory a key names, empty for the root's.
pub fn dir_of(key: &str) -> &str {
    key.split_once(':').map_or("", |(dir, _)| dir)
}

fn unread(path: PathBuf, why: config::LoadError) -> NotDeclared {
    NotDeclared::NoManifest {
        path,
        why: Box::new(why),
    }
}

impl Manifests {
    /// Read the root's `armada.yml` and every workspace's. A file that will
    /// not load is refused, not skipped: a gate that quietly dropped one would
    /// pass what it never read.
    ///
    /// **A person's own copy in Kit is laid over the root's** (`docs/concepts/manifest.md`,
    /// *Kit's copy*), so a Check a Job runs on this Mac is the one its Manifest was served with.
    /// A machine with no Kit copy, CI among them, reads the repository's alone.
    pub fn load(root: &Path) -> Result<Manifests, NotDeclared> {
        let kit = std::env::var_os("HOME").map(|home| Path::new(&home).join(crate::setup::KIT_HOME));
        Manifests::load_over(root, kit.as_deref())
    }

    /// [`Manifests::load`] with Kit's home named, or none.
    pub fn load_over(root: &Path, kit: Option<&Path>) -> Result<Manifests, NotDeclared> {
        let path = root.join(MANIFEST);
        let top = match kit {
            Some(kit) => Manifest::load_over(&path, &fleet::kit_manifest(kit, &root.to_string_lossy())),
            None => Manifest::load(&path),
        }
        .map_err(|why| unread(path, why))?;
        let mut workspaces = Vec::new();
        for dir in fleet::scanning::manifested(&fleet::scanning::Checkout::at(root)) {
            let path = root.join(&dir).join(MANIFEST);
            let one =
                Manifest::load_workspace(&path, &dir, &top).map_err(|why| unread(path, why))?;
            workspaces.push(one);
        }
        Ok(Manifests {
            root: top,
            workspaces,
        })
    }

    pub fn root(&self) -> &Manifest {
        &self.root
    }

    /// The manifest a key names and the bare name in it. A name that does not
    /// start with a workspace's directory is the root's.
    pub fn named<'a>(&self, key: &'a str) -> (&Manifest, &'a str) {
        if let Some((dir, name)) = key.split_once(':') {
            if let Some(found) = self.workspaces.iter().find(|one| one.dir() == dir) {
                return (found, name);
            }
        }
        (&self.root, key)
    }

    /// What `manifest` owns of `changed`, relative to its own directory. The
    /// root's is every path no workspace claims.
    pub fn owned(&self, manifest: &Manifest, changed: &[String]) -> Vec<String> {
        let dir = manifest.dir();
        changed
            .iter()
            .filter(|path| config::owner(&self.workspaces, path) == dir)
            .map(|path| {
                let path = path.trim_start_matches('/');
                path.strip_prefix(dir)
                    .map_or(path, |rest| rest.trim_start_matches('/'))
                    .to_string()
            })
            .collect()
    }

    /// Every Check `changed` hits, as keys: the root's, then each workspace's
    /// in the order its manifest writes them.
    ///
    /// With no workspace this is every Check whose `when` covers `changed`.
    /// With workspaces it is the Checks of the manifests `config::gating`
    /// names, each asked about what that manifest owns, so a root Check never
    /// sees a workspace's path and an empty diff names nothing.
    pub fn covering(&self, changed: &[String]) -> Vec<String> {
        if self.workspaces.is_empty() {
            return self.checks_of(&self.root, changed);
        }
        config::gating(&self.root, &self.workspaces, changed)
            .into_iter()
            .flat_map(|gate| {
                let manifest = std::iter::once(&self.root)
                    .chain(&self.workspaces)
                    .find(|one| one.dir() == gate.dir)?;
                Some(self.checks_of(manifest, &self.owned(manifest, changed)))
            })
            .flatten()
            .collect()
    }

    fn checks_of(&self, manifest: &Manifest, paths: &[String]) -> Vec<String> {
        manifest
            .checks_as_written()
            .iter()
            .filter(|name| {
                manifest
                    .check(name)
                    .is_some_and(|check| check.covers(paths))
            })
            .map(|name| key(manifest.dir(), name))
            .collect()
    }

    /// The Commands a workspace's `setup.requires` names, as `armada run`
    /// keys: its own Commands in its directory, and a Command it does not
    /// declare, which is the root's, by its bare name and so in the root.
    pub fn required_by(&self, dir: &str) -> Vec<String> {
        let Some(manifest) = self.workspaces.iter().find(|one| one.dir() == dir) else {
            return Vec::new();
        };
        manifest
            .prepared_by()
            .iter()
            .map(|needed| match manifest.command(needed.name()) {
                Some(_) => key(dir, needed.name()),
                None => needed.name().to_string(),
            })
            .collect()
    }
}
