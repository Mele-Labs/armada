//! `armada need`, as the fake answers it: a declaration is echoed as a need
//! with nothing ahead of it, and a read finds nothing. What the ledger keeps and
//! orders is `fleet::needing`'s, against a real store.

use ipc::{ManifestId, NeedAct, NeedAnswer, NeedCall, NeedLine, NeedList};

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
}
