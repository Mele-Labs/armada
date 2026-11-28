//! The person's dispatch, held by the Operator before any Job exists (`dispatch_job`).
//!
//! **With the Operator on and seated the work goes to it first.** The whole body is kept in the store and the
//! Operator rules: start a Job, fold the work into the live one, keep it for later, or ask him on its card.
//!
//! **What he marks it as is a hint, and decides nothing.** The Operator is told it, in a line, and rules as it
//! would for any dispatch: whether a call is made, the ways offered and the workflow a Job follows do not depend
//! on it. It stays on the stored dispatch, so it rides onto the Job or the parked idea. Parked ideas are Fleet's
//! own list (`parked_ideas`), since a Bridge that is closed when one is parked has no list to put it in.
//!
//! **His work is never lost.** A ruling that did not come, a call that failed, an act that could not be carried
//! out, a Skip, and a dispatch still kept when Fleet starts each start the Job as it was asked. The body is
//! taken from the store by whoever acts on it, so the Job is started once.

//! The person's dispatch (`dispatch_job`).
//!
//! **With the Operator's session running, a plain dispatch is told to it**: it starts the Job, folds the work
//! into a live one or asks him on a card, with its own tools. **Otherwise the Job starts as he asked**, which is
//! what the Operator being off does. A dispatch he held, forked, rooted or made to wait, and one carrying `then`,
//! is a plain start, since the notes and messages to follow need the Job they were set up for.
//!
//! **His work is never lost.** A dispatch a Fleet that ruled on dispatches kept, and one whose start failed, is
//! started as it was asked when Fleet starts. The body is taken from the store by whoever acts on it, so the Job is
//! started once. Parked ideas are Fleet's own list (`parked_ideas`).

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{HostedSessions, Refusal};
use ipc::operator_board::{DispatchWay, OperatorDispatched};
use ipc::{DispatchJob, DispatchThen, Event, JobDispatched, ParkedIdea, ParkedIdeas, SendSessionNotes, SessionId, SessionRecord, WireError};

use super::arriving::{Arrival, Arrived, Source};
use super::desk::first_line;
use crate::daemon::Fleet;

/// What a held dispatch's arrival id begins with: `start:dispatch:<ulid>`.
pub(super) const DISPATCH: &str = "start:dispatch:";

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
    /// `dispatch_job`. **A dispatch he held, forked, rooted or made to wait is a plain start**, and so is every
    /// dispatch while the Operator's session is not running.
    pub(crate) async fn job_dispatched(self: &Arc<Self>, dispatch: DispatchJob) -> Result<JobDispatched, Refusal> {
        let start = &dispatch.start;
        let request = start.request.as_deref().map(str::trim).filter(|words| !words.is_empty()).map(str::to_string);
        let plain = start.fork.is_none() && start.root.is_none() && start.after.is_none() && !start.held;
        if let (Some(request), true) = (request, plain) {
            let id = format!("{DISPATCH}{}", self.mint().ulid().as_str().to_lowercase());
            let arrival = dispatch_arrival(&id, &dispatch, &request, crate::share::from_phone_or_other_mac());
            // **A session that cannot be told leaves the dispatch started as he asked**, as the Operator being off does.
            // So does one carrying `then`: the notes and messages to follow need the Job they were set up for.
            if self.operator_on() && dispatch.then.is_empty() && self.arrival_told(&arrival).await {
                return Ok(JobDispatched { session: None, operator: Some(id), parked: None });
            }
        }
        let session = Box::pin(self.dispatch_started(dispatch)).await?;
        Ok(JobDispatched { session: Some(session), operator: None, parked: None })
    }

    /// Every dispatch kept when Fleet stopped and not waiting on his go-ahead, started as it was asked: those an
    /// earlier Fleet held for its ruling.
    pub async fn dispatches_resumed(self: &Arc<Self>) {
        let kept = self.store().lock().await.operator_dispatches().unwrap_or_default();
        for one in kept.into_iter().filter(|one| !one.asked) {
            self.dispatch_released(&one.id).await;
        }
    }

    /// The dispatch `id` started as it was asked, where it is still held. A start that fails keeps it for the next
    /// time Fleet starts.
    pub(super) async fn dispatch_released(self: &Arc<Self>, id: &str) {
        if let Err(why) = self.dispatch_started_as_asked(id).await {
            self.operator_warned("a dispatch the Operator held did not start", why);
        }
    }

    /// The dispatch `id` started as it was asked, sent what followed it, and told.
    pub(super) async fn dispatch_started_as_asked(self: &Arc<Self>, id: &str) -> Result<(), String> {
        let Some(dispatch) = self.dispatch_taken(id).await else { return Ok(()) };
        let manifest = dispatch.start.manifest_id.as_str().to_string();
        match Box::pin(self.dispatch_started(dispatch.clone())).await {
            Ok(record) => {
                self.dispatch_told(id, &manifest, DispatchWay::Started, Some(record.id), None).await;
                Ok(())
            }
            Err(why) => {
                self.dispatch_kept_again(id, &dispatch).await;
                Err(format!("{why:?}"))
            }
        }
    }

    /// The Job `dispatch` makes, and what followed it sent into it.
    pub(crate) async fn dispatch_started(self: &Arc<Self>, dispatch: DispatchJob) -> Result<SessionRecord, Refusal> {
        let DispatchJob { start, then, .. } = dispatch;
        let manifest = start.manifest_id.as_str().to_string();
        let words = start.request.clone().unwrap_or_default();
        let subject = start.title.clone().filter(|title| !title.trim().is_empty()).unwrap_or_else(|| first_line(&words));
        let found = self.found_elsewhere(&manifest, &subject, &words).await;
        let record = Box::pin(Arc::clone(self).started_reading(start)).await?;
        self.dispatch_sent(&record.id, then).await;
        // Told after his own words, so they stay his first message as he wrote them.
        self.elsewhere_told(record.id.as_str(), &manifest, &subject, &found).await;
        Ok(record)
    }

    /// `list_parked_ideas`.
    pub(crate) async fn parked_ideas_wire(&self) -> Result<ParkedIdeas, Refusal> {
        let kept = self.store().lock().await.parked_ideas().map_err(|why| self.ledger_fault(why))?;
        let mut ideas = Vec::new();
        for one in kept {
            match ipc::decode::<DispatchJob>("an idea kept for later", one.body.as_bytes()) {
                Ok(dispatch) => ideas.push(ParkedIdea { id: one.id, dispatch, at: one.created_at }),
                Err(why) => self.operator_warned("an idea kept for later does not read", why.to_string()),
            }
        }
        Ok(ParkedIdeas { ideas })
    }

    /// `remove_parked_idea`.
    pub(crate) async fn parked_idea_removed(&self, id: &str) -> Result<ParkedIdeas, Refusal> {
        let removed = self.store().lock().await.remove_parked_idea(id).map_err(|why| self.ledger_fault(why))?;
        if !removed {
            let wire = WireError::raised("fleet.parked_idea_unknown", format!("no idea is kept for later under {id}"), self.run_id());
            return Err(Refusal::Unacceptable(wire));
        }
        self.parked_ideas_wire().await
    }

    /// `then`, sent into `session` in order. One that is refused is warned of and the rest go on.
    ///
    /// **Each send runs on a task of its own**, so it starts at the top of a worker's stack. Polled inline it ran
    /// beneath every layer of the router and the whole start, and a note's send overflowed the stack and aborted
    /// Fleet (21 Nov).
    async fn dispatch_sent(self: &Arc<Self>, session: &SessionId, then: Vec<DispatchThen>) {
        for send in then {
            let fleet = Arc::clone(self);
            let session = session.clone();
            let sent = tokio::spawn(async move {
                match send {
                    DispatchThen::Notes(notes) => fleet.send_session_notes(SendSessionNotes { session_id: session, notes }).await.map(|_| ()),
                    DispatchThen::Message(message) => fleet.send_session_message_aiming(message.to(session), None).await.map(|_| ()),
                }
            })
            .await;
            match sent {
                Ok(Ok(())) => {}
                Ok(Err(why)) => self.operator_warned("what followed a dispatch was not sent", format!("{why:?}")),
                Err(why) => self.operator_warned("what followed a dispatch was not sent", why.to_string()),
            }
        }
    }

    /// The held dispatch `id`, taken so that one act only starts it.
    async fn dispatch_taken(&self, id: &str) -> Option<DispatchJob> {
        let body = self.store().lock().await.take_operator_dispatch(id).ok().flatten()?;
        match ipc::decode::<DispatchJob>("a dispatch the Operator holds", body.as_bytes()) {
            Ok(dispatch) => Some(dispatch),
            Err(why) => {
                self.operator_warned("a dispatch the Operator held does not read", why.to_string());
                None
            }
        }
    }

    /// A dispatch whose start failed, held again for the next time Fleet starts.
    async fn dispatch_kept_again(&self, id: &str, dispatch: &DispatchJob) {
        let Ok(body) = ipc::encode(dispatch) else { return };
        let at = self.now();
        let _ = self.store().lock().await.keep_operator_dispatch(id, &body, at.as_str());
    }

    /// `operator.dispatched`, naming the Job started or the live Job the work was folded into.
    async fn dispatch_told(&self, id: &str, manifest: &str, way: DispatchWay, job: Option<SessionId>, named: Option<String>) {
        let title = match (&job, named) {
            (_, Some(named)) => Some(named),
            (Some(job), None) => Some(self.title_of(job.as_str()).await),
            (None, None) => None,
        };
        self.events().publish(Event::OperatorDispatched(OperatorDispatched {
            dispatch_id: id.to_string(),
            manifest_id: manifest.to_string(),
            way,
            session_id: job,
            title,
        }));
    }
}

/// The dispatch as the session is told it: work no Job holds. `away` where he made it on his phone or another Mac.
fn dispatch_arrival(id: &str, dispatch: &DispatchJob, request: &str, away: bool) -> Arrival {
    Arrival {
        id: id.to_string(),
        arrived: Arrived::Start,
        source: Source::Dispatch { away },
        session_id: String::new(),
        manifest_id: dispatch.start.manifest_id.as_str().to_string(),
        project: String::new(),
        subject: dispatch.start.title.clone().filter(|title| !title.trim().is_empty()).unwrap_or_else(|| first_line(request)),
        words: request.to_string(),
    }
}
