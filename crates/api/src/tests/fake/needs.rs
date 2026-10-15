//! `armada need`, as the fake answers it: a declaration is echoed as a need
//! with nothing ahead of it, and a read finds nothing. What the ledger keeps and
//! orders is `fleet::needing`'s, against a real store.

use ipc::{ManifestId, NeedAct, NeedAnswer, NeedCall, NeedLine, NeedList, RepositoryAreas, SlotTouches, WriteClashes};

use super::FakeDaemon;
use crate::{Needs, Refusal};

impl Needs for FakeDaemon {
    async fn act_on_need(&self, call: NeedCall) -> Result<NeedAnswer, Refusal> {
        let mine = (call.act != NeedAct::Release).then(|| NeedLine {
            holder: ipc::Holder {
                kind: ipc::HolderKind::Session,
                id: format!("branch:{}", call.branch),
            },
            held_by: call.branch,
            path: call.path,
            what: call.what.unwrap_or_default(),
            took: call.value,
            since: ipc::Instant::carried("2026-10-07T09:00:00.000Z"),
        });
        Ok(NeedAnswer {
            mine,
            already: false,
            ahead: Vec::new(),
            gave_back: call.act == NeedAct::Release,
        })
    }

    async fn list_needs(&self, _manifest_id: Option<ManifestId>) -> Result<NeedList, Refusal> {
        Ok(NeedList { needs: Vec::new() })
    }

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
