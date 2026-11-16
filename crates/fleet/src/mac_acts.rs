//! Acts across his Macs. An act on a Job another Mac holds is sent to that Mac, and one that cannot be sent waits in
//! the queue the edits of a project copy use, in one order, until it speaks again. What another Mac sends here is
//! run only for a Job in a shared project, once for each id it sends. `docs/concepts/pocket.md`, *Another Mac*.
//!
//! **The next stage's Job message to another Mac plugs in here**, as one more kind of send behind
//! `/api/mac/act`, queued and drained by this module's queue.
//!
//! **The Mac that holds a Job runs the act on its own checkout and slots**: nothing is run here for a Job held from
//! another Mac, a merge included. Every refusal it gives is passed back as it said it.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{ActAnswered, ActBegun, ActTurn, Refusal};
use core_model::ScopeId;
use ipc::{Event, JobAct, JobDispatched, MacAct, Queued, QueuedList, SessionId, SessionRecord, TakeBack, WireError};
use store::{AttachmentState, FromMac, HolderKind, KeptActRun, KeptQueuedAct};

use crate::daemon::Fleet;
use crate::project_copies::Flushing;
use crate::projects::NO_SUCH_PROJECT;
use crate::session_host::NO_SUCH_SESSION;

/// A Mac this one has nothing from: not paired, or forgotten. A 404.
pub(crate) const MAC_NOT_LINKED: &str = "fleet.other_mac_not_linked";
/// No act waits under an id. A 404.
const QUEUED_NOT_FOUND: &str = "fleet.queued_not_found";
/// The Mac's queue is being sent, so the act may already be on its way. A 409.
const QUEUED_IN_FLIGHT: &str = "fleet.queued_in_flight";
/// A start from another Mac reaches a Job or a folder of this one. A 422.
const START_REACHES: &str = "fleet.start_from_mac_reaches";
/// An act from another Mac without an id to run it once by. A 422.
const ACT_UNNAMED: &str = "fleet.other_mac_act_unnamed";

/// What the other Mac made of an act sent to it.
pub(crate) enum JobSent {
    /// It answered, `status` and all: a refusal is an answer.
    Answered(ActAnswered),
    /// It could not be reached, or its Fleet was not answering: the act waits.
    Unreachable,
}

/// What the drain did with one queued act.
pub(crate) enum Flushed {
    /// Answered: it leaves the queue.
    Sent,
    /// Refused: it stays, with why, and the rest go on.
    Kept,
    /// The Mac could not be reached: the rest wait.
    Stopped,
}

fn queued_of(kept: KeptQueuedAct) -> Option<Queued> {
    let act = ipc::decode("an act queued for another Mac", kept.act.as_bytes()).ok()?;
    Some(Queued { id: kept.id, mac: kept.mac, act, since: kept.at, refused: kept.refused })
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
    fn from_a_mac() -> bool {
        api::via().is_some_and(|via| via.as_wire() == api::MAC)
    }

    fn no_such_job(&self, job: &str) -> Refusal {
        Refusal::NotFound(WireError::raised(NO_SUCH_SESSION, format!("No Job has the id {job}."), self.run_id()))
    }

    // ---- the Mac that holds the Job: what it is sent -------------------------------------------------------

    /// Whether pull request `number` of `repository` is owned by Job `job`, on this Mac.
    async fn pull_owned_by(&self, job: &SessionId, repository: &str, number: u64) -> bool {
        let held = self.store().lock().await.attachments_at("pr", &number.to_string(), Some(repository)).unwrap_or_default();
        held.iter().any(|one| one.holder.kind == HolderKind::Session && one.holder.id == job.as_str() && one.state != AttachmentState::GivenBack)
    }

    /// Whether `manifest` is a repository of shared project `project`.
    fn project_holds(&self, project: &str, manifest: &str) -> bool {
        let held = self.projects_held();
        ScopeId::new(project).ok().and_then(|scope| held.get(&scope).cloned()).is_some_and(|one| {
            one.members().iter().any(|member| member.repository().as_str() == manifest)
        })
    }

    /// `act_from_mac`, before the act runs: the share filter, then whether it ran already. **Anything the filter
    /// refuses is answered as an act on nothing, and nothing is written**, a repeat's answer included.
    pub(crate) async fn act_from_mac_begun(&self, mac: String, sent: &MacAct) -> Result<ActBegun, Refusal> {
        let mac = self.mac_named(&mac)?;
        if sent.id.trim().is_empty() {
            return Err(Refusal::Unacceptable(WireError::raised(ACT_UNNAMED, "An act from another Mac needs an id.", self.run_id())));
        }
        match (&sent.act, sent.act.session_id()) {
            (_, Some(job)) => self.act_from_other_mac(job).await?,
            (JobAct::Arrive(job), None) => self.arrival_checked(job).await?,
            (_, None) => self.start_from_other_mac(sent.act.starts_in())?,
        }
        match &sent.act {
            JobAct::Merge { session_id, repository, number } if !self.pull_owned_by(session_id, repository, *number).await => {
                return Err(self.no_such_job(session_id.as_str()));
            }
            JobAct::Start(start) => self.start_from_mac_checked(start.fork.is_some(), start.root.is_some(), start.after.is_some(), start.project.as_deref(), start.manifest_id.as_str())?,
            JobAct::Dispatch(dispatch) => {
                let start = &dispatch.start;
                self.start_from_mac_checked(start.fork.is_some(), start.root.is_some(), start.after.is_some(), start.project.as_deref(), start.manifest_id.as_str())?;
            }
            _ => {}
        }
        // One at a time, so a send and its retry cannot both run.
        let held = Arc::clone(&self.phone_held().acts).lock_owned().await;
        if let Some(ran) = self.store().lock().await.act_run(&mac, &sent.id).map_err(|why| self.ledger_fault(why))? {
            return Ok(ActBegun::Answered(ActAnswered { status: ran.status, body: ran.body.into_bytes() }));
        }
        Ok(ActBegun::Run(ActTurn { mac, id: sent.id.clone(), what: sent.act.name(), held: Box::new(held) }))
    }

    /// A Job moved here lands in a repository of the project it names: one of this Mac's shared projects, where it
    /// is coming home, or the sending Mac's, which this Mac holds a copy of. Any other is refused as a start is.
    async fn arrival_checked(&self, job: &ipc::ArrivingJob) -> Result<(), Refusal> {
        let project = job.record.hosted.as_ref().and_then(|hosted| hosted.project.as_deref());
        let manifest = job.record.manifest_id.as_ref().map_or("", |one| one.as_str());
        match project {
            Some(id) if (self.project_shared(id) && self.project_holds(id, manifest)) || self.copy_held(id, Some(manifest)).await => Ok(()),
            other => Err(Refusal::NotFound(WireError::raised(NO_SUCH_PROJECT, format!("there is no project {}", other.unwrap_or_default()), self.run_id()))),
        }
    }

    /// A start from another Mac names no Job, folder or wait of this one, and a repository of the shared project it names.
    fn start_from_mac_checked(&self, fork: bool, root: bool, after: bool, project: Option<&str>, manifest: &str) -> Result<(), Refusal> {
        if fork || root || after {
            return Err(Refusal::Unacceptable(WireError::raised(
                START_REACHES,
                "A Job started from another Mac cannot fork a Job, run in a folder or wait on a Job of this Mac.",
                self.run_id(),
            )));
        }
        match project {
            Some(project) if self.project_holds(project, manifest) => Ok(()),
            // The same refusal as an unshared project, so it names nothing of this Mac's.
            other => Err(Refusal::NotFound(WireError::raised(
                NO_SUCH_PROJECT,
                format!("there is no project {}", other.unwrap_or_default()),
                self.run_id(),
            ))),
        }
    }

    /// `act_from_mac`, after it ran: `status` and `body` kept under the sending Mac and its id, and which door it ran as.
    /// A fault is not kept, so the retry of one runs.
    pub(crate) async fn act_from_mac_ran(&self, turn: ActTurn, status: u16, body: &[u8]) {
        if status >= 500 {
            return;
        }
        let ran = KeptActRun {
            mac: turn.mac,
            id: turn.id,
            at: self.now_ms(),
            via: api::via().map_or("none", |via| via.as_wire()).to_string(),
            what: turn.what.to_string(),
            status,
            body: String::from_utf8_lossy(body).into_owned(),
        };
        if let Err(why) = self.store().lock().await.keep_act_run(&ran) {
            self.ledger_fault(why);
        }
    }

    // ---- the Mac that sends --------------------------------------------------------------------------------

    /// The Mac Job `job` is held from, for an act on it. **None for a call that came from another Mac**: it is
    /// run where it landed and never sent on to a third.
    pub(crate) async fn mac_holding_for_act(&self, job: &str) -> Option<String> {
        match Self::from_a_mac() {
            true => None,
            false => self.mac_holding(job).await,
        }
    }

    /// The Mac and Job that own pull request `number` of `repository`, where no Job of this Mac does and a Job held
    /// from another Mac does.
    pub(crate) async fn pull_held_by_other_mac(&self, repository: &str, number: u64) -> Option<(String, SessionId)> {
        if Self::from_a_mac() {
            return None;
        }
        let target = number.to_string();
        let own = self.store().lock().await.attachments_at("pr", &target, Some(repository)).unwrap_or_default();
        if own.iter().any(|one| one.holder.kind == HolderKind::Session && one.state != AttachmentState::GivenBack) {
            return None;
        }
        let held = self.sessions_from_macs().await.ok()?;
        held.into_iter().find_map(|record| {
            let owns = record.attachments.iter().any(|one| {
                one.kind == "pr"
                    && one.target == target
                    && one.state != ipc::AttachmentState::GivenBack
                    && one.manifest_id.as_ref().is_some_and(|id| id.as_str() == repository)
            });
            owns.then(|| record.mac.clone().map(|mac| (mac, record.id.clone()))).flatten()
        })
    }

    /// Whether anything is held from Mac `mac`: a Job it shares or a project, which is all a start needs of it.
    pub(crate) async fn mac_known(&self, mac: &str) -> bool {
        let store = self.store().lock().await;
        if store.paired_macs().is_ok_and(|paired| paired.iter().any(|one| one.id == mac && one.state == store::MacState::Linked)) {
            return true;
        }
        [FromMac::Sessions, FromMac::Projects]
            .into_iter()
            .any(|list| store.held_from_macs(list).is_ok_and(|held| held.iter().any(|one| one.mac == mac)))
    }

    /// `sent` handed to Mac `mac` through the link. **A Mac that cannot be reached, or whose Fleet is not answering,
    /// is told apart from one that refused**: the first is queued, the second is the answer.
    pub(crate) async fn send_job_act(&self, mac: &str, sent: &MacAct) -> Result<JobSent, Refusal> {
        let body = self.encoded(sent)?.into_bytes();
        let Ok(switch) = self.phone_switch() else {
            return Ok(JobSent::Unreachable);
        };
        let Ok((status, bytes)) = switch.send_other_mac(mac, "/other_mac/act", body).await else {
            return Ok(JobSent::Unreachable);
        };
        match status == 401 || status >= 500 {
            true => Ok(JobSent::Unreachable),
            false => Ok(JobSent::Answered(ActAnswered { status, body: bytes })),
        }
    }

    /// The row a start made on Mac `mac` kept at once, so it is drawn before that Mac's stream says it.
    async fn row_held(&self, mac: &str, mut record: SessionRecord) -> SessionRecord {
        record.mac = Some(mac.to_string());
        if let Ok(body) = self.encoded(&record) {
            let kept = self.store().lock().await.keep_from_mac(FromMac::Sessions, mac, record.id.as_str(), &body);
            if kept.is_ok() {
                self.events().relay(Event::SessionChanged(record.clone()));
            }
        }
        record
    }

    /// What a Mac answered `act` with, taken in: **a Job it started is kept as its row and answered with its `mac`**.
    pub(crate) async fn act_answered(&self, mac: &str, act: &JobAct, answered: ActAnswered) -> ActAnswered {
        if !(200..300).contains(&answered.status) {
            return answered;
        }
        let rewritten = match act {
            JobAct::Start(_) => match ipc::decode::<SessionRecord>("a Job's row from another Mac", &answered.body) {
                Ok(record) => self.encoded(&self.row_held(mac, record).await),
                Err(_) => return answered,
            },
            JobAct::Dispatch(_) => match ipc::decode::<JobDispatched>("a dispatch from another Mac", &answered.body) {
                Ok(mut dispatched) => {
                    if let Some(record) = dispatched.session.take() {
                        dispatched.session = Some(self.row_held(mac, record).await);
                    }
                    self.encoded(&dispatched)
                }
                Err(_) => return answered,
            },
            _ => return answered,
        };
        match rewritten {
            Ok(body) => ActAnswered { status: answered.status, body: body.into_bytes() },
            Err(_) => answered,
        }
    }

    /// `act` on a Job Mac `mac` holds, or a start there. Sent at once where it answers and nothing waits ahead of
    /// it, and otherwise **kept in order, answered 202 with the `Queued`**, to go when it next speaks.
    pub(crate) async fn acted_on_other_mac(&self, mac: String, act: JobAct) -> Result<ActAnswered, Refusal> {
        let mac = self.mac_named(&mac)?;
        if !self.mac_known(&mac).await {
            return Err(Refusal::NotFound(WireError::raised(MAC_NOT_LINKED, format!("This Mac is not linked to Mac {mac}."), self.run_id())));
        }
        let (id, at) = (self.minted(), self.now_ms());
        let waiting = !self.queued_unsent(&mac).await?.is_empty();
        if !waiting {
            if let JobSent::Answered(answered) = self.send_job_act(&mac, &MacAct { id: id.clone(), act: act.clone(), at }).await? {
                return Ok(self.act_answered(&mac, &act, answered).await);
            }
        }
        let body = self.encoded(&act)?;
        self.store().lock().await.queue_job_act(&mac, &id, at, &body).map_err(|why| self.ledger_fault(why))?;
        let queued = Queued { id, mac, act, since: at, refused: None };
        self.queued_relayed().await;
        Ok(ActAnswered { status: 202, body: self.encoded(&queued)?.into_bytes() })
    }

    /// One queued act of Mac `mac`'s sent, by the drain. A refusal keeps it, with the Mac's own words.
    pub(crate) async fn job_act_flushed(&self, mac: &str, seq: i64, sent: MacAct) -> Flushed {
        match self.send_job_act(mac, &sent).await {
            Ok(JobSent::Answered(answered)) if (200..300).contains(&answered.status) => {
                self.act_answered(mac, &sent.act, answered).await;
                Flushed::Sent
            }
            Ok(JobSent::Answered(answered)) => {
                let why = self.refusal_passed_on(mac, answered.status, &answered.body).error().message.clone();
                let kept = self.store().lock().await.refuse_queued_act(seq, &why);
                match kept {
                    Ok(()) => Flushed::Kept,
                    Err(_) => Flushed::Stopped,
                }
            }
            Ok(JobSent::Unreachable) | Err(_) => Flushed::Stopped,
        }
    }

    // ---- what waits ----------------------------------------------------------------------------------------

    async fn queued_unsent(&self, mac: &str) -> Result<Vec<KeptQueuedAct>, Refusal> {
        self.store().lock().await.queued_acts(Some(mac), true).map_err(|why| self.ledger_fault(why))
    }

    /// The acts on Jobs waiting for Mac `mac`, or for every Mac, oldest first, those a Mac refused among them.
    pub(crate) async fn queued_listed(&self, mac: Option<&str>) -> Result<QueuedList, Refusal> {
        let kept = self.store().lock().await.queued_acts(mac, false).map_err(|why| self.ledger_fault(why))?;
        Ok(QueuedList { queued: kept.into_iter().filter(|one| one.kind == "job").filter_map(queued_of).collect() })
    }

    /// What waits now, sent to Bridge alone.
    pub(crate) async fn queued_relayed(&self) {
        if let Ok(list) = self.queued_listed(None).await {
            self.events().relay(Event::QueuedChanged(list));
        }
    }

    /// An act that waits, removed. **Refused while its Mac's queue is being sent**: the act may be on its way.
    pub(crate) async fn queued_taken_back(&self, take: TakeBack) -> Result<QueuedList, Refusal> {
        let kept = self.store().lock().await.queued_job_act(&take.id).map_err(|why| self.ledger_fault(why))?;
        let Some(kept) = kept else {
            return Err(Refusal::NotFound(WireError::raised(QUEUED_NOT_FOUND, format!("Nothing waits under {}.", take.id), self.run_id())));
        };
        if !self.phone_held().flushing.lock().unwrap_or_else(std::sync::PoisonError::into_inner).insert(kept.mac.clone()) {
            return Err(Refusal::IllegalMove(WireError::raised(
                QUEUED_IN_FLIGHT,
                "What waits for that Mac is being sent now, so this may already be on its way.",
                self.run_id(),
            )));
        }
        let _unlock = Flushing { held: &self.phone_held().flushing, mac: kept.mac.clone() };
        self.store().lock().await.drop_project_act(kept.seq).map_err(|why| self.ledger_fault(why))?;
        self.queued_relayed().await;
        self.queued_listed(None).await
    }
}
