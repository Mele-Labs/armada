//! Loading the workspaces' own manifests for a repository, at setup and again
//! when its root manifest is re-read.
//!
//! **A file that will not load is left out and said, never a repository that
//! will not serve.** The others stand, as a workflow that does not fit leaves
//! the rest of the catalogue alone.

use std::path::{Path, PathBuf};

use config::{LoadError, Manifest};
use ipc::{ManifestFault, ManifestReading, ManifestRefused};

use crate::scanning::{manifested, Checkout};

/// What reading every workspace file came to.
#[derive(Debug, Default)]
pub struct Workspaces {
    /// The ones that loaded, in directory order.
    pub manifests: Vec<Manifest>,
    /// The ones that did not, each with the file and why.
    pub refused: Vec<(PathBuf, LoadError)>,
}

/// Every workspace below `root` holding its own `armada.yml`, read against
/// `manifest`, the root's. Depth and package-file rules are Scan's own.
pub fn load(root: &Path, manifest: &Manifest) -> Workspaces {
    let mut read = Workspaces::default();
    for dir in manifested(&Checkout::at(root)) {
        let file = root.join(&dir).join("armada.yml");
        match Manifest::load_workspace(&file, &dir, manifest) {
            Ok(loaded) => read.manifests.push(loaded),
            Err(why) => read.refused.push((file, why)),
        }
    }
    read
}

/// A load failure as the wire carries it.
pub(crate) fn refused(why: &LoadError) -> ManifestRefused {
    ManifestRefused {
        summary: why.to_string(),
        faults: why
            .refusals()
            .iter()
            .map(|one| ManifestFault {
                key: one.key.clone(),
                fault: one.fault.to_string(),
            })
            .collect(),
    }
}

/// The reading that says `file` was left out, in the shape a refused root
/// manifest is said in.
pub(crate) fn reading_of(file: &Path, why: &LoadError, at: ipc::Instant) -> ManifestReading {
    ManifestReading {
        path: file.display().to_string(),
        at,
        moved: Vec::new(),
        at_restart: Vec::new(),
        refused: Some(refused(why)),
    }
}
