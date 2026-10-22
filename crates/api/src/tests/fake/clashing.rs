//! The clash reads, as the fake answers them: nothing is clashing and nothing is touched.

use ipc::{ClashFile, ClashSide, ManifestId, RepositoryAreas, SessionFileChange, SlotTouches, WriteClashes};

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

    async fn read_clash_file(
        &self,
        _session_id: String,
        _other_kind: ClashSide,
        _other_id: String,
        _path: String,
    ) -> Result<ClashFile, Refusal> {
        Ok(ClashFile { ours: String::new(), theirs: String::new() })
    }

    async fn get_session_file_change(
        &self,
        _session_id: String,
        _path: String,
        _seen: Option<String>,
    ) -> Result<SessionFileChange, Refusal> {
        Ok(SessionFileChange { stamp: String::new(), diff: String::new(), cut: false, unchanged: false })
    }

    async fn list_repository_areas(&self) -> Result<RepositoryAreas, Refusal> {
        Ok(RepositoryAreas { repositories: Vec::new() })
    }

    async fn list_slot_touches(&self) -> Result<SlotTouches, Refusal> {
        Ok(SlotTouches { repositories: Vec::new() })
    }
}
