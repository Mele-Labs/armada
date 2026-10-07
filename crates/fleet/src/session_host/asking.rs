//! A hosted session's permission door: the call is put to the person on the
//! session's own thread, and the answer goes back inside the call. Since 23.49.
//!
//! **Helm's asking machinery, not a second one**: the same table, the same
//! reading of what is destructive, pushes to a shared space or writes off this
//! machine, and the same words back to the model. What differs is where the ask
//! is drawn, which is the thread and never Helm's dock, and which calls reach a
//! person: `auto` asks only about those three classes, `ask` and `acceptEdits`
//! ask about everything the person's own settings do not cover.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{Caller, Refusal};
use ipc::{
    AskingToRun, HelmCallAnswer, ManifestId, RunOrNot, SessionAskState, SessionMode, SessionRow,
};

use super::serving::mode_of;
use crate::daemon::Fleet;
use crate::helm::{answering, because, unanswered};

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
    /// The hosted session whose process holds this call's connection.
    /// **Placed by the connection**, as Helm's is, never by what the caller says.
    pub(crate) fn session_holding(&self, caller: &Caller) -> Option<String> {
        self.hosts().all().into_iter().find_map(|(id, runtime)| {
            let pid = runtime.state().process.as_ref().and_then(|p| p.pid())?;
            crate::peer::held_within(caller, self.host().port, &[pid], self.peers().as_ref())
                .then_some(id)
        })
    }

    pub(crate) async fn session_permission(
        &self,
        id: &str,
        asking: AskingToRun,
    ) -> Result<RunOrNot, Refusal> {
        let (session, hosting) = self.session_and_hosting(id).await?;
        let served = self.served_named(Some(&ManifestId::carried(&hosting.manifest_id)))?;
        let mode = mode_of(&hosting.mode);
        let put_to_a_person = match mode {
            SessionMode::Ask | SessionMode::AcceptEdits => true,
            SessionMode::Auto | SessionMode::Plan => because(&asking).is_some(),
        };
        if !put_to_a_person {
            return Ok(RunOrNot::Allow {
                updated_input: asking.input,
            });
        }
        let asks = self.hosts().asks();
        let (waiting, answer) = asks.minted(
            &ManifestId::carried(&hosting.manifest_id),
            &asking,
            &self.now(),
        );
        let row_id = format!("ask-{}-{id}", waiting.call);
        let put = |state: SessionAskState| SessionRow::Ask {
            id: row_id.clone(),
            at: self.instant(),
            ask: waiting.clone(),
            state,
        };
        self.hosts().of(id).state().asked = Some(waiting.clone());
        self.row_put(id, put(SessionAskState::Waiting)).await;
        let _ = self.published_hosted(&session.id).await;

        let held = asks.hold();
        let (decided, state) = match tokio::time::timeout(held, answer).await {
            Ok(Ok(said)) => {
                let state = match said.answer {
                    HelmCallAnswer::AllowOnce => SessionAskState::AllowedOnce,
                    HelmCallAnswer::Refuse => SessionAskState::Refused,
                    HelmCallAnswer::AllowAndRemember => {
                        let at = adapters::personal_settings(served.root());
                        // A settings file that will not be written is not a
                        // refusal: the person said yes.
                        let _ = adapters::remember_the_rule(&at, &waiting.rule);
                        SessionAskState::AllowedAndRemembered
                    }
                };
                (answering(&asking, &said), state)
            }
            Ok(Err(_)) => (unanswered(held.as_secs()), SessionAskState::SessionGone),
            Err(_) => {
                let _ = asks.withdraw(&waiting.call);
                (unanswered(held.as_secs()), SessionAskState::Unanswered)
            }
        };
        self.hosts().of(id).state().asked = None;
        self.row_put(id, put(state)).await;
        let _ = self.published_hosted(id).await;
        Ok(decided)
    }
}
