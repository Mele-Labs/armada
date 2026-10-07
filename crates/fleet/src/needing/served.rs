//! What Fleet serves `armada need`: declare, say what was taken, give back, and
//! read the standing list. `docs/capabilities/needs.md`.
//!
//! **The caller names a branch and Fleet finds who holds it**
//! (`ledger::holder_of_branch`), so a Drone running `armada need` in its Job's
//! worktree declares as that Job and a person's terminal declares as the session
//! on the branch, or as the branch where there is none.

use std::path::Path;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{Needs, Refusal};
use ipc::{ManifestId, NeedAct, NeedAnswer, NeedCall, NeedLine, NeedList, WireError};

use super::ledger::{self, Need};
use crate::adrift::Adrift;
use crate::daemon::Fleet;
use crate::repositories::Served;

/// A call that names nothing it can be kept against. A 422.
const NEED_UNNAMED: &str = "fleet.need_unnamed";

/// A `took` for a need nobody declared. A 422.
const NEED_NOT_DECLARED: &str = "fleet.need_not_declared";

fn line(need: &Need) -> NeedLine {
    NeedLine {
        holder: ipc::Holder {
            kind: match need.holder.kind {
                store::HolderKind::Session => ipc::HolderKind::Session,
                store::HolderKind::Job => ipc::HolderKind::Job,
            },
            id: need.holder.id.clone(),
        },
        held_by: need.held_by.clone(),
        path: need.path.clone(),
        what: need.what.clone(),
        took: need.took.clone(),
        since: ipc::Instant::carried(&need.since),
    }
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
    fn need_unnamed(&self, what: &str) -> Refusal {
        Refusal::Unacceptable(WireError::raised(
            NEED_UNNAMED,
            format!("a need call needs {what}"),
            self.run_id(),
        ))
    }

    /// A need held by a branch alone, whose branch git no longer has, is given
    /// back, so nobody waits behind one that was deleted. **Read where a need is
    /// read**, never on a timer. Git that cannot run says nothing about a branch.
    pub(crate) async fn gone_branches_given_back(&self, served: &Served) {
        let manifest = served.manifest().id().as_str().to_string();
        let root = served.root().to_string();
        let now = self.now().as_str().to_string();
        // The branches first, so git is asked with no lock held.
        let branches: Vec<String> = {
            let store = self.store().lock().await;
            ledger::standing(&store, &manifest, None)
                .unwrap_or_default()
                .iter()
                .filter_map(|need| ledger::branch_of(&need.holder).map(str::to_string))
                .collect()
        };
        if branches.is_empty() {
            return;
        }
        let gone = tokio::task::spawn_blocking(move || {
            branches
                .into_iter()
                .filter(|branch| !adapters::needs::branch_exists(Path::new(&root), branch))
                .collect::<Vec<_>>()
        })
        .await
        .unwrap_or_default();
        let mut store = self.store().lock().await;
        let _ = ledger::gone_branches_given_back(
            &mut store,
            &manifest,
            |branch| !gone.iter().any(|gone| gone == branch),
            &now,
        );
    }
}

impl<H, V, W> Needs for Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    async fn act_on_need(&self, call: NeedCall) -> Result<NeedAnswer, Refusal> {
        let (branch, path) = (call.branch.trim(), call.path.trim());
        if branch.is_empty() {
            return Err(self.need_unnamed("the branch it is declared from"));
        }
        if path.is_empty() {
            return Err(self.need_unnamed("a path"));
        }
        let served = self.served_named(call.manifest_id.as_ref())?;
        let manifest = served.manifest().id().as_str().to_string();
        self.gone_branches_given_back(&served).await;
        let fault = |why| self.refusal(Adrift::Writing(why));
        let now = self.now().as_str().to_string();
        let mut store = self.store().lock().await;
        let holder = ledger::holder_of_branch(&store, &manifest, branch).map_err(fault)?;
        match call.act {
            NeedAct::Declare => {
                let what = call.what.as_deref().map(str::trim).unwrap_or_default();
                if what.is_empty() {
                    return Err(self.need_unnamed("what is needed there"));
                }
                let done =
                    ledger::declare(&mut store, &holder, &manifest, branch, path, what, &now)
                        .map_err(fault)?;
                Ok(NeedAnswer {
                    mine: Some(line(&done.mine)),
                    already: done.already,
                    ahead: done.ahead.iter().map(line).collect(),
                    gave_back: false,
                })
            }
            NeedAct::Took => {
                let value = call.value.as_deref().map(str::trim).unwrap_or_default();
                if value.is_empty() {
                    return Err(self.need_unnamed("what it took"));
                }
                let took = ledger::took(&mut store, &holder, &manifest, path, value, &now)
                    .map_err(fault)?;
                let Some(need) = took else {
                    return Err(Refusal::Unacceptable(WireError::raised(
                        NEED_NOT_DECLARED,
                        format!(
                            "{branch} has no need on {path} — `armada need {path} \"<what>\"` \
                             first, then say what it took"
                        ),
                        self.run_id(),
                    )));
                };
                Ok(NeedAnswer {
                    mine: Some(line(&need)),
                    already: false,
                    ahead: Vec::new(),
                    gave_back: false,
                })
            }
            NeedAct::Release => {
                let gave_back =
                    ledger::release(&mut store, &holder, &manifest, path, &now).map_err(fault)?;
                Ok(NeedAnswer {
                    mine: None,
                    already: false,
                    ahead: Vec::new(),
                    gave_back,
                })
            }
        }
    }

    async fn list_needs(&self, manifest_id: Option<ManifestId>) -> Result<NeedList, Refusal> {
        let served = self.served_named(manifest_id.as_ref())?;
        let manifest = served.manifest().id().as_str().to_string();
        self.gone_branches_given_back(&served).await;
        let store = self.store().lock().await;
        let mut needs = ledger::standing(&store, &manifest, None)
            .map_err(|why| self.refusal(Adrift::Writing(why)))?;
        // Grouped by path; within one the ledger's order stands, so the sort
        // must not disturb it.
        needs.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(NeedList {
            needs: needs.iter().map(line).collect(),
        })
    }
}
