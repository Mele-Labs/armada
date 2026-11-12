//! The person's dispatch, held by the Operator before any Job exists (`dispatch_job`).
//!
//! **With the Operator on and seated the work goes to it first.** Where no live Job shares anything with the
//! words, the Job starts at once and no call is made. Where one does, the whole body is kept in the store and
//! the Operator rules: start a Job, fold the work into the live one, or ask him on its card.
//!
//! **His work is never lost.** A ruling that did not come, a call that failed, an act that could not be carried
//! out, a Skip, and a dispatch still kept when Fleet starts each start the Job as it was asked. The body is
//! taken from the store by whoever acts on it, so the Job is started once.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{HostedSessions, Refusal};
use ipc::operator_board::{DispatchWay, OperatorDispatched};
use ipc::{DispatchJob, DispatchThen, Event, JobDispatched, Notice, SendSessionNotes, SessionId, SessionRecord};

use super::arriving::{Arrival, Arrived, Source};
use super::candidates::near;
use super::duplicating::first_line;
use super::thening::paths_in;
use super::{Ruled, Ruling, Way, NEEDS_YOU_WAIT};
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
    /// dispatch while the Operator is off or not seated.
    pub(crate) async fn job_dispatched(self: &Arc<Self>, dispatch: DispatchJob) -> Result<JobDispatched, Refusal> {
        let start = &dispatch.start;
        let request = start.request.as_deref().map(str::trim).filter(|words| !words.is_empty()).map(str::to_string);
        let plain = start.fork.is_none() && start.root.is_none() && start.after.is_none() && !start.held;
        let operator = self.operator_seat().operator().filter(|_| self.operator_on());
        let (Some(request), true, Some(operator)) = (request, plain, operator) else {
            let session = self.dispatch_started(dispatch, true).await?;
            return Ok(JobDispatched { session: Some(session), operator: None });
        };
        let id = format!("{DISPATCH}{}", self.mint().ulid().as_str().to_lowercase());
        let arrival = dispatch_arrival(&id, &dispatch, &request);
        if near(&arrival, &operator.live_jobs().await).is_empty() {
            // Weighed here, against the live Jobs, so the Job is not weighed again once it exists.
            let record = self.dispatch_started(dispatch, false).await?;
            self.dispatch_told(&id, &arrival.manifest_id, DispatchWay::Started, Some(record.id.clone())).await;
            return Ok(JobDispatched { session: Some(record), operator: None });
        }
        let body = ipc::encode(&dispatch).map_err(|why| self.hosted_refusal("fleet.dispatch_unencodable", &why.to_string()))?;
        let at = self.now();
        if let Err(why) = self.store().lock().await.keep_operator_dispatch(&id, &body, at.as_str()) {
            return Err(self.ledger_fault(why));
        }
        let fleet = Arc::clone(self);
        tokio::spawn(async move { fleet.dispatch_ruled(arrival).await });
        Ok(JobDispatched { session: None, operator: Some(id) })
    }

    /// The Operator's ruling on a held dispatch, waited for. Anything but a ruling that was carried out starts it.
    async fn dispatch_ruled(self: Arc<Self>, arrival: Arrival) {
        let id = arrival.id.clone();
        let ruled = match self.operator_seat().operator() {
            Some(operator) => operator.ruled(arrival, NEEDS_YOU_WAIT).await,
            None => Ruled::PassedOn(super::PassedOn::Failed("no Operator is seated".into())),
        };
        if !matches!(ruled, Ruled::Ruled(_)) {
            self.dispatch_released(&id).await;
        }
    }

    /// Every dispatch kept when Fleet stopped and not waiting on his go-ahead, started as it was asked.
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
        match Box::pin(self.dispatch_started(dispatch.clone(), false)).await {
            Ok(record) => {
                self.dispatch_told(id, &manifest, DispatchWay::Started, Some(record.id)).await;
                Ok(())
            }
            Err(why) => {
                self.dispatch_kept_again(id, &dispatch).await;
                Err(format!("{why:?}"))
            }
        }
    }

    /// The dispatch `id` handed to live Job `into`: its words told to it, and what followed sent into it.
    /// **A Job that does not take it leaves the dispatch started as it was asked**, so it is not lost.
    pub(super) async fn dispatch_handed(self: &Arc<Self>, id: &str, into: &str) -> Result<(), String> {
        let Some(body) = self.store().lock().await.operator_dispatch(id).ok().flatten() else { return Ok(()) };
        let Ok(dispatch) = ipc::decode::<DispatchJob>("a dispatch the Operator holds", body.as_bytes()) else {
            return Err("the dispatch does not read".to_string());
        };
        let words = dispatch.start.request.clone().unwrap_or_default();
        let text = format!("The Operator handed you work the person dispatched, as yours is already on it:\n\n{words}\n\nDo it as part of your work, or say why it is not yours.");
        if !matches!(self.told_by_fleet(into, &text, Notice::Operator).await, Ok(true)) {
            Box::pin(self.dispatch_released(id)).await;
            return Err("the Job did not take it".to_string());
        }
        if self.dispatch_taken(id).await.is_none() {
            return Ok(());
        }
        self.dispatch_sent(&SessionId::carried(into), dispatch.then).await;
        self.dispatch_told(id, dispatch.start.manifest_id.as_str(), DispatchWay::Handed, Some(SessionId::carried(into))).await;
        Ok(())
    }

    /// The dispatch the Operator would hand to a live Job but is not sure of: kept as a go-ahead, `Hand to <Job>` or
    /// `Keep both`, and waiting for it. **A repeat of one answered before, or no Job near, starts it.**
    pub(super) async fn dispatch_asked(self: &Arc<Self>, arrival: &Arrival, ruling: &Ruling) -> Result<(), String> {
        let live = self.live_now().await;
        let Some(first) = near(arrival, &live).into_iter().next() else { return self.dispatch_started_as_asked(&arrival.id).await };
        let offer = Ruling { other: Some(first.job.session_id.clone()), shared: first.paths.clone(), ..Ruling::alone(Way::ExistingJob, ruling.because.clone()) };
        if self.operator_suggested(arrival, &offer).await?.is_some() {
            return self.dispatch_started_as_asked(&arrival.id).await;
        }
        let marked = self.store().lock().await.operator_dispatch_asked(&arrival.id);
        if marked.is_err() {
            return Err("the dispatch was not marked as waiting on him".to_string());
        }
        self.dispatch_told(&arrival.id, &arrival.manifest_id, DispatchWay::Asked, None).await;
        Ok(())
    }

    /// What a ruling on a held dispatch did: start it, hand it over, or ask him.
    pub(super) async fn dispatch_sourced(self: &Arc<Self>, arrival: &Arrival, ruling: &Ruling) -> Result<(), String> {
        match (ruling.way, ruling.other.as_deref()) {
            (Way::NewJob, _) => Box::pin(self.dispatch_started_as_asked(&arrival.id)).await,
            (Way::ExistingJob, Some(into)) => Box::pin(self.dispatch_handed(&arrival.id, into)).await,
            (Way::Raise, _) => Box::pin(self.dispatch_asked(arrival, ruling)).await,
            _ => Err("the dispatch was ruled a way it is not offered".to_string()),
        }
    }

    /// The Job `dispatch` makes, and what followed it sent into it. `overlap` is whether the Operator weighs the
    /// Job against the live ones once it exists, which a dispatch it has ruled on already is not.
    async fn dispatch_started(self: &Arc<Self>, dispatch: DispatchJob, overlap: bool) -> Result<SessionRecord, Refusal> {
        let DispatchJob { start, then } = dispatch;
        let record = Arc::clone(self).started_reading_with(start, overlap).await?;
        self.dispatch_sent(&record.id, then).await;
        Ok(record)
    }

    /// `then`, sent into `session` in order. One that is refused is warned of and the rest go on.
    async fn dispatch_sent(self: &Arc<Self>, session: &SessionId, then: Vec<DispatchThen>) {
        for send in then {
            let sent = match send {
                DispatchThen::Notes(notes) => Arc::clone(self).send_session_notes(SendSessionNotes { session_id: session.clone(), notes }).await.map(|_| ()),
                DispatchThen::Message(message) => Arc::clone(self).send_session_message_aiming(message.to(session.clone()), None).await.map(|_| ()),
            };
            if let Err(why) = sent {
                self.operator_warned("what followed a dispatch was not sent", format!("{why:?}"));
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
    async fn dispatch_told(&self, id: &str, manifest: &str, way: DispatchWay, job: Option<SessionId>) {
        let title = match &job {
            Some(job) => Some(self.title_of(job.as_str()).await),
            None => None,
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

/// The dispatch as the Operator reads it: work no Job holds, which it may start, fold into a live Job or ask about.
fn dispatch_arrival(id: &str, dispatch: &DispatchJob, request: &str) -> Arrival {
    Arrival {
        id: id.to_string(),
        arrived: Arrived::Start,
        source: Source::Dispatch,
        session_id: String::new(),
        manifest_id: dispatch.start.manifest_id.as_str().to_string(),
        subject: dispatch.start.title.clone().filter(|title| !title.trim().is_empty()).unwrap_or_else(|| first_line(request)),
        words: request.to_string(),
        paths: paths_in(request),
        issues: Vec::new(),
        ways: vec![Way::NewJob, Way::ExistingJob, Way::Raise],
        open: Vec::new(),
    }
}
