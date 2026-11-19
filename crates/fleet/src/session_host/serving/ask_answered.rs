//! `answer_session_ask`: a person's answer to a card the agent put, which takes the ask off the table.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{AnswerSessionAsk, CallAnswer, SessionRecord, WireError};

use super::{SESSION_ANSWER_INCOMPLETE, SESSION_ANSWER_NOT_A_FOLDER, SESSION_ANSWER_NOT_ITS_KIND};
use crate::asks::NotAnswerable;
use crate::daemon::Fleet;

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
    /// `answer_session_ask`. **`by_him`** is false where the Operator or Autopilot answers in his place, which is no
    /// decision of his to file.
    pub(crate) async fn session_ask_answered(&self, said: AnswerSessionAsk, by_him: bool) -> Result<SessionRecord, Refusal> {
        let id = said.session_id.as_str().to_string();
        let terminal = self.terminal_session(&id).await?.is_some();
        if terminal {
            self.terminal_ask_standing(&id).await;
        } else {
            self.session_and_hosting(&id).await?;
        }
        let held = self
            .hosts()
            .of(&id)
            .state()
            .asked
            .as_ref()
            .is_some_and(|asked| asked.call == said.call);
        if !held {
            return Err(self.ask_not_waiting());
        }
        let said_answer = said.answer;
        self.hosts().of(&id).state().answered_via = api::via();
        let waiting = self
            .hosts()
            .asks()
            .answer(&said)
            .map_err(|why| match why {
                NotAnswerable::Incomplete => Refusal::Unacceptable(WireError::raised(
                    SESSION_ANSWER_INCOMPLETE,
                    "that ask has questions, and each needs an answer",
                    self.run_id(),
                )),
                NotAnswerable::NotAFolderAbove => Refusal::Unacceptable(WireError::raised(
                    SESSION_ANSWER_NOT_A_FOLDER,
                    "an always answer names the folder it is for: the path asked about or one above it",
                    self.run_id(),
                )),
                NotAnswerable::NotItsKind => Refusal::Unacceptable(WireError::raised(
                    SESSION_ANSWER_NOT_ITS_KIND,
                    "a like this answer names the ask's own program and script, or its program alone",
                    self.run_id(),
                )),
                _ => self.ask_not_waiting(),
            })?;
        if terminal {
            self.terminal_ask_answered(&id, &waiting, &crate::asks::Said::of(&said))
                .await?;
        }
        if by_him && said_answer == CallAnswer::AllowOnce {
            self.questions_decided(&id, &said).await;
        }
        let record = self.published_hosted(&id).await?;
        self.session_answered(&record, crate::timeline::card_answer_says(said_answer));
        Ok(record)
    }
}
