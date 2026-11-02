//! What a running Autopilot answered, as the person reviews it: the answers not yet kept, a press
//! that keeps one, and a correction, which is sent to the Job as `Re: <asked>` and then the words.
//! The shape is Sleep's review (`crate::sleeping`), over this table.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{HostedSessions, Refusal};
use ipc::{
    AutopilotDecided, AutopilotDecisions, OverrideAutopilotDecision, ReviewAutopilotDecision, SendSessionMessage,
    WireError,
};
use store::KeptDecision;

use super::wire::decision_from_word;
use crate::daemon::Fleet;

const NO_DECISION: &str = "fleet.autopilot_no_decision";
const EMPTY: &str = "fleet.autopilot_decision_empty";

/// An answer on the wire. **One of a kind this build does not know is left off** rather than guessed.
fn decided(row: KeptDecision) -> Option<AutopilotDecided> {
    Some(AutopilotDecided {
        kind: decision_from_word(&row.kind)?,
        id: row.id,
        manifest_id: ipc::ManifestId::carried(row.manifest_id),
        session_id: row.session_id.map(ipc::SessionId::carried),
        who: row.who,
        asked: row.asked,
        chose: row.chose,
        because: row.because,
        at: ipc::Instant::carried(row.at),
    })
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
    /// `get_autopilot_decisions`.
    pub(super) async fn autopilot_decisions_state(&self) -> Result<AutopilotDecisions, Refusal> {
        let rows = self.store().lock().await.autopilot_decisions_to_review().map_err(|why| self.ledger_fault(why))?;
        Ok(AutopilotDecisions { to_review: rows.into_iter().filter_map(decided).collect() })
    }

    /// Publish the answers to review as they stand now.
    pub(super) async fn autopilot_decisions_published(&self) -> Result<AutopilotDecisions, Refusal> {
        let state = self.autopilot_decisions_state().await?;
        self.publish(ipc::Event::AutopilotDecisionsChanged(state.clone()));
        Ok(state)
    }

    fn no_decision(&self) -> Refusal {
        Refusal::IllegalMove(WireError::raised(NO_DECISION, "no answer of Autopilot's holds that id", self.run_id()))
    }

    /// `review_autopilot_decision`: the person kept one, so it leaves the review.
    pub(super) async fn autopilot_decision_reviewed(&self, review: ReviewAutopilotDecision) -> Result<AutopilotDecisions, Refusal> {
        let kept = self
            .store()
            .lock()
            .await
            .review_autopilot_decision(&review.id)
            .map_err(|why| self.ledger_fault(why))?;
        if !kept {
            return Err(self.no_decision());
        }
        self.autopilot_decisions_published().await
    }

    /// `override_autopilot_decision`: the correction goes to the Job first, so a send that fails leaves
    /// the answer in the review to correct again.
    pub(super) async fn autopilot_decision_overridden(
        self: Arc<Self>,
        over: OverrideAutopilotDecision,
    ) -> Result<AutopilotDecisions, Refusal> {
        let words = over.text.trim().to_string();
        if words.is_empty() {
            return Err(self.hosted_refusal(EMPTY, "a correction needs words"));
        }
        let held = self.store().lock().await.autopilot_decision(&over.id).map_err(|why| self.ledger_fault(why))?;
        let Some((held, session)) = held.and_then(|one| {
            let session = one.session_id.clone()?;
            Some((one, session))
        }) else {
            return Err(self.no_decision());
        };
        Arc::clone(&self)
            .send_session_message(SendSessionMessage {
                session_id: ipc::SessionId::carried(session),
                text: format!("Re: {}\n{}", held.asked, words),
                attachments: Vec::new(),
                mentions: Vec::new(),
            })
            .await?;
        self.store()
            .lock()
            .await
            .correct_autopilot_decision(&over.id, &words)
            .map_err(|why| self.ledger_fault(why))?;
        self.autopilot_decisions_published().await
    }
}
