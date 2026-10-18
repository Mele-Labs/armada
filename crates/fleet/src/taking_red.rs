//! A Session takes a repository's red main, so no Job on another machine is
//! handed it. `take_main_red`; `docs/capabilities/merge-line.md`, *Fixing a red
//! across machines*.
//!
//! **It writes the same claim a Job's take does**, with the Session's id as
//! holder, and the same local take, which the hub's Fixing row reads. A Session
//! is told who holds the red only when it is refused. Asking again for the red
//! it already holds is not a refusal.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{Caller, Refusal};
use ipc::{MainRedTaken, ManifestId, TakeMainRed};
use store::{TakenBy, TakenHow};

use crate::daemon::Fleet;
use crate::main_fix::{refused, MainNotFixable};
use crate::red_claim::Taker;

const NOT_TAKEABLE: &str = "fleet.main_red_not_takeable";

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
    pub(crate) async fn main_red_taken(
        &self,
        caller: Option<Caller>,
        take: TakeMainRed,
    ) -> Result<MainRedTaken, Refusal> {
        let refuse = |why: &str| self.hosted_refusal(NOT_TAKEABLE, why);
        // Placed by the connection, then by the id a terminal session passes.
        let id = caller
            .as_ref()
            .and_then(|caller| self.session_holding(caller))
            .or_else(|| {
                let named = take.session_id.as_ref()?.as_str().trim().to_string();
                (!named.is_empty()).then_some(named)
            })
            .ok_or_else(|| refuse("a red is taken by a Job, and this call is not from one"))?;
        let manifest = self
            .store()
            .lock()
            .await
            .session(&id)
            .map_err(|why| self.ledger_fault(why))?
            .and_then(|session| session.manifest_id)
            .ok_or_else(|| refuse(&format!("job {id} is in no repository Fleet serves")))?;
        let root = self.served_named(Some(&ManifestId::carried(&manifest)))?.root().to_string();
        let (_, main) = self.red_of(&root).await.map_err(|why| self.refusal(why))?;
        match self.taking(&main).await {
            Some(TakenBy::Session(held)) if held == id => {}
            Some(by) => return Err(self.refusal(refused(&root, MainNotFixable::Taken(by)))),
            None => {}
        }
        let claimed = self
            .claim_red(&main, &Taker::Session(id.clone()), take.take_over)
            .await
            .map_err(|why| self.refusal(refused(&root, why)))?;
        self.recorded_session_take(&main, &id, TakenHow::Took, None).await;
        Ok(MainRedTaken { root, claimed })
    }
}
