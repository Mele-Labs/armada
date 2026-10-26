//! `propose_draft`: a session puts a draft to the person for approval, kept as a `draft` row that
//! waits as `draft:<row id>` until `answer_waiting` settles it.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{ProposeDraft, SessionRecord};

use crate::daemon::Fleet;

pub(crate) const DRAFT_UNBUILT: &str = "fleet.draft_unbuilt";

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
    // rust-engineer builds this next
    pub(crate) async fn draft_proposed(
        &self,
        _caller: Option<api::Caller>,
        _draft: ProposeDraft,
    ) -> Result<SessionRecord, Refusal> {
        Err(self.hosted_refusal(DRAFT_UNBUILT, "Fleet cannot take a draft yet"))
    }
}
