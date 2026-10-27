//! Each Job's scratch folder: under Fleet's own state and that Job's alone, where it may write,
//! remove and chmod without the slot guard asking, as in its slots. **Made when first named** and
//! **removed when the Job ends**, so nothing in it outlives the Job.
//!
//! The guard reads a held directory as a slot by its path, so the folder's name is
//! [`SCRATCH_DIR`], which [`is_scratch`] knows.

use std::path::{Path, PathBuf};

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};

use crate::daemon::Fleet;

/// The folder every Job's scratch folder is under, beside its uploads.
pub(crate) const SCRATCH_DIR: &str = "job-scratch";

/// The variable a hosted Job's process carries its scratch folder in.
pub(crate) const SCRATCH_ENV: &str = "ARMADA_SCRATCH";

/// `job`'s scratch folder under `attachments`, where Fleet keeps what is a Job's own.
pub(crate) fn scratch_under(attachments: &Path, job: &str) -> PathBuf {
    attachments.join(SCRATCH_DIR).join(job)
}

/// Whether `directory` is a Job's scratch folder or inside one.
pub(crate) fn is_scratch(directory: &str) -> bool {
    directory.contains(&format!("/{SCRATCH_DIR}/"))
}

impl<H, V, W> Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// `job`'s scratch folder, made where it is not there.
    pub(crate) fn scratch_of(&self, job: &str) -> PathBuf {
        let folder = scratch_under(Path::new(&self.host().attachments_dir), job);
        let _ = std::fs::create_dir_all(&folder);
        folder
    }

    /// Remove `job`'s scratch folder and all it holds.
    pub(crate) fn scratch_removed(&self, job: &str) {
        let folder = scratch_under(Path::new(&self.host().attachments_dir), job);
        let _ = std::fs::remove_dir_all(folder);
    }
}
