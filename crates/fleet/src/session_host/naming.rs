//! `name_session`: Armada names a hosted Job on request, from its first message, by the question
//! the proposer is asked at the start. `docs/concepts/job-proposer.md`, *Names he gave*.
//!
//! **Not his rename.** Nothing is kept for the proposer to learn from, and the title is listed as
//! Armada's pick, so a rename he makes after it is learned against it as usual.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{NameSession, SessionRecord};

use super::delivering::first_message;
use super::serving::{NO_SUCH_SESSION, SESSION_COMMAND_REFUSED};
use crate::daemon::Fleet;

/// A Job with no message to name it from. A 422.
const SESSION_NO_REQUEST: &str = "fleet.session_no_request";
/// No proposer to ask, a call that failed, or an answer with no title. A 422, and nothing written.
const SESSION_NOT_NAMED: &str = "fleet.session_not_named";

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
    pub(crate) async fn named_by_armada(&self, name: NameSession) -> Result<SessionRecord, Refusal> {
        let id = name.session_id.as_str().to_string();
        if self.terminal_session(&id).await?.is_some() {
            return Err(self.hosted_refusal(
                SESSION_COMMAND_REFUSED,
                &format!("job {id} is a terminal job, and Armada names only a hosted one"),
            ));
        }
        let (_, hosting) = self.session_and_hosting(&id).await?;
        let rows = self.rows_of(&id).await?;
        let Some(request) = first_message(&rows).map(str::to_string) else {
            return Err(self.hosted_refusal(
                SESSION_NO_REQUEST,
                "this job has no message to name it from",
            ));
        };
        let not_named = |why: String| self.hosted_refusal(SESSION_NOT_NAMED, &why);
        let proposing = self.proposing().map_err(|why| {
            not_named(format!("Fleet could not set up the proposer, so this job was not named: {why:?}"))
        })?;
        let answered = self
            .proposer_answered(&proposing, &hosting.manifest_id, &request)
            .await
            .map_err(|why| {
                not_named(format!("the proposer did not answer, so this job was not named: {why}"))
            })?;
        let Some(title) = answered.brief.read_title(&answered.reply) else {
            return Err(not_named(String::from(
                "the proposer's answer had no title, so this job was not named",
            )));
        };
        let mut store = self.store().lock().await;
        let Some(mut session) = store.session(&id).map_err(|why| self.ledger_fault(why))? else {
            return Err(self.hosted_refusal(NO_SUCH_SESSION, &format!("no job Fleet hosts is named {id}")));
        };
        let changed = session.title.as_deref() != Some(title.as_str());
        if changed {
            session.title = Some(title);
            store
                .keep_session(&session)
                .map_err(|why| self.ledger_fault(why))?;
            self.armada_named(&mut store, &id);
        }
        let record = self.ledger_row(&store, &session)?;
        drop(store);
        if changed {
            self.publish(ipc::Event::SessionChanged(record.clone()));
        }
        Ok(record)
    }
}
