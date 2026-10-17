//! The clash reads, as the fake answers them: nothing is clashing and nothing is touched.

use ipc::{ManifestId, RepositoryAreas, SlotTouches, WriteClashes};

use super::FakeDaemon;
use crate::{Clashing, Refusal};

impl Clashing for FakeDaemon {
    async fn list_write_clashes(
        &self,
        _manifest_id: Option<ManifestId>,
        _holder: Option<String>,
    ) -> Result<WriteClashes, Refusal> {
        Ok(WriteClashes {
            clashes: Vec::new(),
        })
    }

    async fn list_repository_areas(&self) -> Result<RepositoryAreas, Refusal> {
        Ok(RepositoryAreas { repositories: Vec::new() })
    }

    async fn list_slot_touches(&self) -> Result<SlotTouches, Refusal> {
        Ok(SlotTouches { repositories: Vec::new() })
    }
}
