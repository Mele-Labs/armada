//! `name_session`: Armada names a hosted Job on request, from its first message, by the question
//! the proposer is asked at the start. `docs/concepts/job-proposer.md`, *Names he gave*.
//!
//! **Not his rename.** Nothing is kept for the proposer to learn from, and the title is listed as
//! Armada's pick, so a rename he makes after it is learned against it as usual.
//!
//! The same call for the title alone is tried once more in the background where a Job's reading failed,
//! and once for a fork an agent titled. Either lands only over the title the Job holds meanwhile.

use std::sync::Arc;

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
            .proposer_answered(&proposing, &hosting.manifest_id, &request, None)
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

    /// One call for the title alone, of a Job holding a stand-in: after its reading failed, or a
    /// fork an agent titled. The answer lands only over the stand-in, and a failure leaves it.
    pub(crate) async fn title_tried_again(self: Arc<Self>, id: String, request: String, hint: Option<String>) {
        match self.title_answered(&id, &request, hint.as_deref()).await {
            Ok(title) => self.stand_in_named(&id, title).await,
            Err(why) => eprintln!("job {id}: the title was not read, so the job keeps its title: {why}"),
        }
    }

    async fn title_answered(&self, id: &str, request: &str, hint: Option<&str>) -> Result<String, String> {
        let proposing = self.proposing().map_err(|why| format!("{why:?}"))?;
        let (_, hosting) = self.session_and_hosting(id).await.map_err(|_| String::from("the job is gone"))?;
        let answered = self.proposer_answered(&proposing, &hosting.manifest_id, request, hint).await?;
        answered
            .brief
            .read_title(&answered.reply)
            .ok_or_else(|| String::from("the answer had no title"))
    }

    /// The title replaces the stand-in, where the Job still holds it: his rename or the wand ended it,
    /// and anything else that titled the Job meanwhile no longer matches it.
    async fn stand_in_named(&self, id: &str, title: String) {
        let record = {
            let mut store = self.store().lock().await;
            let stand_in = self.hosts().of(id).state().proposal.stand_in.take();
            let Ok(Some(mut session)) = store.session(id) else { return };
            if stand_in.is_none() || session.title != stand_in || stand_in.as_deref() == Some(title.as_str()) {
                return;
            }
            session.title = Some(title);
            if let Err(why) = store.keep_session(&session) {
                self.session_warned(id, "a job's title was not kept", format!("{why:?}"));
                return;
            }
            self.armada_named(&mut store, id);
            let Ok(record) = self.ledger_row(&store, &session) else { return };
            record
        };
        self.publish(ipc::Event::SessionChanged(record));
    }

    /// A fork an agent titled is named once, when Start sends its first turn, from that turn's words.
    /// A fork has no reading to do it, and its agent's call returns before anything could.
    pub(crate) fn fork_named(self: &Arc<Self>, id: &str, first: &str) {
        let hint = {
            let runtime = self.hosts().of(id);
            let mut state = runtime.state();
            if state.proposal.asked || state.proposal.stand_in.is_none() || first.trim().is_empty() {
                return;
            }
            state.proposal.asked = true;
            state.proposal.hint.clone()
        };
        let (fleet, id, request) = (Arc::clone(self), id.to_string(), first.trim().to_string());
        tokio::spawn(async move { fleet.title_tried_again(id, request, hint).await });
    }
}
