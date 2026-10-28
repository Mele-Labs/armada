//! `resolve_pull_request_note`: a hosted session marks done a pull request note the owner sent it,
//! and nobody else's. `docs/concepts/session.md`.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{PullRequestNote, ResolvePullRequestNote};

use crate::daemon::Fleet;
use crate::pull_request_notes::note_read;

const NOTE_REFUSED: &str = "fleet.pull_request_note_refused";

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
    /// Mark the note done. Refused unless it was sent to this session and is still open.
    pub(crate) async fn pull_request_note_resolved(
        &self,
        caller: Option<api::Caller>,
        resolve: ResolvePullRequestNote,
    ) -> Result<PullRequestNote, Refusal> {
        let id = self.waiting_session(caller, resolve.session_id.as_ref())?;
        let (session, _) = self.session_and_hosting(&id).await?;
        if session.state == store::SessionState::Ended {
            return Err(self.closed(&id));
        }
        let writing = |why| self.refusal(crate::adrift::Adrift::Writing(why));
        let mut store = self.store().lock().await;
        let Some(found) = store.pull_request_note(&resolve.note_id).map_err(writing)? else {
            return Err(self.hosted_refusal(NOTE_REFUSED, &format!("no note {}", resolve.note_id)));
        };
        if found.sent_to.as_deref() != Some(id.as_str()) {
            return Err(self.hosted_refusal(NOTE_REFUSED, "that note was not sent to this job"));
        }
        if found.status == "done" {
            return Err(self.hosted_refusal(NOTE_REFUSED, "that note is already resolved"));
        }
        let done = store.finish_pull_request_note(&found.id).map_err(writing)?.unwrap_or(found);
        Ok(note_read(&done))
    }
}
