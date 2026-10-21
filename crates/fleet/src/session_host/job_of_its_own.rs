//! A Job Fleet opens for one act of its own: a hosted session with a slot leased and its branch
//! cut, whose process never starts unless a message is sent to it. Commit to repository's, so the
//! piece rides in a pull request of its own (the owner, 21 Oct 2026). `crate::committing`.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{HostedSessions, Refusal};
use ipc::{CloseSession, ManifestId, SessionId, StartSession};

use super::gate::Taken;
use crate::daemon::Fleet;
use crate::repositories::Served;
use crate::run_memory::JobTree;

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
    /// A Job titled `title` in `served`, holding a slot and its own branch. The inner `Err` says
    /// why no slot was leased, and the Job is closed again.
    pub(crate) async fn job_of_its_own(
        &self,
        served: &Served,
        title: String,
    ) -> Result<Result<(SessionId, JobTree), String>, Refusal> {
        let start = StartSession {
            manifest_id: ManifestId::carried(served.manifest().id().as_str()),
            title: Some(title),
            model: None,
            effort: None,
            mode: None,
            root: None,
            fork: None,
            request: None,
            after: None,
        };
        let id = self.started_hosted(start, false).await?.id;
        let leased = match self.take_lease(id.as_str(), served).await {
            // No process runs, so there is nothing to move.
            Taken::Moving(moving) => {
                self.hosts().of(id.as_str()).state().moving = None;
                Ok(JobTree { path: moving.directory, branch: moving.branch })
            }
            Taken::Held { directory, .. } => match self.session_and_hosting(id.as_str()).await?.1.lease_branch {
                Some(branch) => Ok(JobTree { path: directory, branch }),
                None => Err(format!("The slot at {directory} has no branch.")),
            },
            Taken::Full => Err(format!("Every slot of {} is leased.", served.root())),
            Taken::Refused(why) => Err(why),
            Taken::Gone => Err(String::from("The Job ended before a slot was leased.")),
        };
        match leased {
            Ok(tree) => Ok(Ok((id, tree))),
            Err(why) => {
                let _ = self.close_session(CloseSession { session_id: id }).await;
                Ok(Err(why))
            }
        }
    }
}
