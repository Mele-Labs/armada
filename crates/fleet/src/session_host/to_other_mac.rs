//! Moving a hosted Job to another of his Macs. `docs/practices/protocol/his-other-macs-fleets-half.md`,
//! *Moving a Job between Macs*.
//!
//! **The Mac it leaves**: the move waits for the turn and the agents it started, as a move into a slot
//! does (`State::mac_move_taken`). Then the Job is paused, so its work is committed to its branch and its
//! slot is freed, the branch is pushed, and the other Mac is sent an `arrive`. A push that fails, or an
//! `arrive` refused or unsent, resumes it here.
//!
//! **The Mac it goes to** answers the `arrive` at once, then reads the conversation in pieces through the
//! link, checks its length and hash, keeps the Job under the same id, paused on the pushed branch, and
//! resumes it, which leases a fresh slot onto that branch. Then it sends a `move_settled`, queued if the
//! link is down. **Only that ends the Job on the Mac it left**: until then it stays paused there, so a
//! read that outlives the link's patience never leaves it running on both.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Pushed, Vcs, Worktree, WorkProduct};
use api::{Refusal, CONVERSATION_PIECE};
use ipc::{
    ArrivingJob, CarriedConversation, CloseSession, JobAct, MacAct, ManifestId, MoveJobToMac, MoveSettled, PauseSession,
    ResumeSession, SendSessionMessage, SendSessionMessageBody, SessionId, SessionRecord, WireError,
};
use sha2::{Digest, Sha256};
use store::{AttachmentState, Holder, KeptAttachment, KeptHosting, KeptSession};

use crate::daemon::Fleet;
use crate::mac_acts::{JobSent, MAC_NOT_LINKED};
use crate::other_macs::MAC_UNREACHABLE;
use crate::session_host::serving::mode_text;
use crate::session_host::start_card::first_turn_of;
use crate::session_host::NO_SUCH_SESSION;

/// A Job that cannot move as it stands: ended, paused, on your words, or moving already. A 409.
const SESSION_NOT_MOVABLE: &str = "fleet.session_not_movable";
/// A move of this Mac's Job that names no Mac to go to. A 422.
const MOVE_UNNAMED: &str = "fleet.move_names_no_mac";
/// A Job arriving that this Mac holds live already. A 409.
const ALREADY_HERE: &str = "fleet.moved_job_already_here";
/// The `end_reason` of a Job that moved to another Mac. **Its row there stands in for it here.**
pub(crate) const MOVED_TO_MAC: &str = "moved_to_mac";

/// What a moved Job is told when it is resumed on the Mac it went to.
fn moved_note(from: &str) -> String {
    format!(
        "You were moved here from Mac {from} and resumed in a fresh slot on your branch, as it was pushed. \
         Servers, terminals, watches and walks you had there are closed. Open again what you still need, and \
         carry on with what you were doing."
    )
}

/// What a Job whose move failed is told when it is resumed where it was.
fn stayed_note(to: &str, why: &str) -> String {
    format!("You were to move to Mac {to}, and stay here: {why}. Carry on with what you were doing.")
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
    fn not_movable(&self, said: String) -> Refusal {
        Refusal::IllegalMove(WireError::raised(SESSION_NOT_MOVABLE, said, self.run_id()))
    }

    fn not_here(&self, job: &str) -> Refusal {
        Refusal::NotFound(WireError::raised(NO_SUCH_SESSION, format!("No Job has the id {job}."), self.run_id()))
    }

    // ---- the person's press -------------------------------------------------------------------------------

    /// `move_job_to_mac`. A Job held from another Mac is asked of that Mac, to come here.
    pub(crate) async fn job_moved_to_mac(self: &Arc<Self>, asked: MoveJobToMac) -> Result<SessionRecord, Refusal> {
        let id = asked.session_id.as_str().to_string();
        if let Some(holder) = self.mac_holding_for_act(&id).await {
            return self.move_asked_of(holder, asked.session_id).await;
        }
        let Some(mac) = asked.mac else {
            return Err(Refusal::Unacceptable(WireError::raised(MOVE_UNNAMED, "A Job of this Mac moves to a Mac the move names.", self.run_id())));
        };
        self.move_begun(&id, mac).await
    }

    /// Job `job`, held from Mac `holder`, asked to come here. **Sent through the queue**, so acts on it that wait
    /// for that Mac go first, and a Mac that cannot be reached is asked when it next speaks; the row is answered as
    /// held until its own stream says it moves.
    async fn move_asked_of(&self, holder: String, job: SessionId) -> Result<SessionRecord, Refusal> {
        let answered = self.acted_on_other_mac(holder.clone(), JobAct::Move { session_id: job.clone() }).await?;
        if answered.status == 202 {
            let held = self.sessions_from_macs().await?;
            return held.into_iter().find(|one| one.id == job).ok_or_else(|| self.not_here(job.as_str()));
        }
        if !(200..300).contains(&answered.status) {
            return Err(self.refusal_passed_on(&holder, answered.status, &answered.body));
        }
        let mut record: SessionRecord = ipc::decode("a moving Job's row from another Mac", &answered.body)
            .map_err(|why| Refusal::Fault(WireError::raised(MAC_UNREACHABLE, why.to_string(), self.run_id())))?;
        record.mac = Some(holder);
        Ok(record)
    }

    /// `act_from_mac`'s `move`: Mac `mac` asks for Job `job`.
    pub(crate) async fn job_moved_from_mac(self: &Arc<Self>, mac: String, job: SessionId) -> Result<SessionRecord, Refusal> {
        self.move_begun(job.as_str(), mac).await
    }

    /// The move marked on Job `id`, and made at once where its turn is over. Refused with nothing changed.
    async fn move_begun(self: &Arc<Self>, id: &str, mac: String) -> Result<SessionRecord, Refusal> {
        let mac = self.mac_named(&mac)?;
        if self.terminal_session(id).await?.is_some() {
            return Err(self.not_here(id));
        }
        let (session, hosting) = self.session_and_hosting(id).await?;
        // Only a Job the other Mac may see may go there, and one that may not is answered as one that is not here.
        self.act_from_other_mac(&SessionId::carried(id)).await?;
        if !self.mac_known(&mac).await {
            return Err(Refusal::NotFound(WireError::raised(MAC_NOT_LINKED, format!("This Mac is not linked to Mac {mac}."), self.run_id())));
        }
        if session.state == store::SessionState::Ended {
            return Err(self.not_movable(format!("job {id} has ended")));
        }
        if hosting.paused_at.is_some() {
            return Err(self.not_movable(format!("job {id} is paused; resume it first")));
        }
        if hosting.on_your_words {
            return Err(self.not_movable(format!("job {id} acts on your words, which stay on this Mac")));
        }
        self.live_things_let_go(id).await?;
        let runtime = self.hosts().of(id);
        {
            let mut state = runtime.state();
            if state.to_mac.is_some() {
                return Err(self.not_movable(format!("job {id} is moving already")));
            }
            state.to_mac = Some(mac);
            state.carrying_to_mac = false;
        }
        let record = self.published_hosted(id).await?;
        // A Job between turns, or with no process, moves now; one mid-turn moves where the turn ends.
        if !self.work_beyond_the_turn(id).await {
            let taken = runtime.state().mac_move_taken();
            if let Some(mac) = taken {
                self.carried_to_mac_apart(id, mac);
            }
        }
        Ok(record)
    }

    /// What a move does to the Job's live things here: its walks, terminals, servers and watches. **They close**:
    /// the pause stops its servers and holds its watches, and the end gives back the rest, and the Job is told so
    /// where it lands (`moved_note`). A move that must wait for them, or refuse, decides it here.
    async fn live_things_let_go(&self, _id: &str) -> Result<(), Refusal> {
        Ok(())
    }

    // ---- the Mac it leaves --------------------------------------------------------------------------------

    /// The hand-over, on a task of its own: the turn's end and the act that marked it go on.
    pub(crate) fn carried_to_mac_apart(self: &Arc<Self>, id: &str, mac: String) {
        let (fleet, id) = (Arc::clone(self), id.to_string());
        tokio::spawn(async move {
            // Boxed: the resume of a move that failed starts a process, and its future outgrows a worker's stack.
            if let Err(why) = Box::pin(fleet.carried_to_mac(&id, &mac)).await {
                Box::pin(fleet.move_undone(&id, &mac, &why)).await;
            }
        });
    }

    /// Pause, push, and hand over. Any failure is said as a sentence, and the Job resumed here by the caller.
    async fn carried_to_mac(&self, id: &str, mac: &str) -> Result<(), String> {
        let said = |refusal: Refusal| refusal.error().message.clone();
        self.session_paused(PauseSession { session_id: SessionId::carried(id) }).await.map_err(said)?;
        let (_, hosting) = self.session_and_hosting(id).await.map_err(said)?;
        if let Some(branch) = hosting.lease_branch.clone() {
            let served = self.served_named(Some(&ManifestId::carried(&hosting.manifest_id))).map_err(said)?;
            let (vcs, worktree) = (Arc::clone(self.vcs()), Worktree::at(served.root(), &branch));
            let pushed = tokio::task::spawn_blocking(move || vcs.push(&worktree)).await.map_err(|why| why.to_string())?;
            match pushed {
                Ok(Pushed::ToTheRemote { .. }) => {}
                Ok(Pushed::NoRemote) => return Err(format!("the repository has no remote to push {branch} to")),
                Err(why) => return Err(format!("{branch} was not pushed: {}", why.said())),
            }
        }
        let conversation = self.conversation_measured(id).await?;
        let mut record = self.published_hosted(id).await.map_err(said)?;
        record.mac = None;
        let arriving = ArrivingJob { record, branch: hosting.lease_branch, conversation };
        let sent = MacAct { id: self.minted(), act: JobAct::Arrive(arriving), at: self.now_ms() };
        match self.send_job_act(mac, &sent).await.map_err(said)? {
            JobSent::Answered(answered) if (200..300).contains(&answered.status) => Ok(()),
            JobSent::Answered(answered) => Err(self.refusal_passed_on(mac, answered.status, &answered.body).error().message.clone()),
            JobSent::Unreachable => Err(format!("Mac {mac} could not be reached")),
        }
    }

    /// The length and SHA-256 of Job `id`'s conversation, read whole off the thread. None kept is empty.
    async fn conversation_measured(&self, id: &str) -> Result<CarriedConversation, String> {
        let (home, id) = (self.host().home.clone(), id.to_string());
        tokio::task::spawn_blocking(move || {
            let mut hash = Sha256::new();
            let mut from = 0;
            while let Some(piece) = adapters::terminal_thread::conversation_piece(&home, &id, from, CONVERSATION_PIECE).map_err(|why| why.to_string())? {
                if piece.is_empty() {
                    break;
                }
                hash.update(&piece);
                from += piece.len() as u64;
            }
            Ok(CarriedConversation { bytes: from, sha256: hex(&hash.finalize()) })
        })
        .await
        .map_err(|why| why.to_string())?
    }

    /// A move that did not happen: the mark comes off and a Job it paused is resumed here, told why.
    async fn move_undone(self: &Arc<Self>, id: &str, mac: &str, why: &str) {
        self.session_warned(id, "the Job did not move to another Mac", format!("{mac}: {why}"));
        {
            let runtime = self.hosts().of(id);
            let mut state = runtime.state();
            state.to_mac = None;
            state.carrying_to_mac = false;
        }
        let paused = self.session_and_hosting(id).await.is_ok_and(|(_, hosting)| hosting.paused_at.is_some());
        if paused {
            let asked = ResumeSession { session_id: SessionId::carried(id) };
            if let Err(refused) = self.session_resumed_telling(asked, &stayed_note(mac, why)).await {
                self.session_warned(id, "a Job whose move failed was not resumed", refused.error().message.clone());
            }
        }
        let _ = self.published_hosted(id).await;
    }

    /// `act_from_mac`'s `move_settled`. **A Job paused here that the Mac took is ended even when the mark is
    /// gone**, as after a restart: it runs there now.
    pub(crate) async fn move_settled_from(self: &Arc<Self>, mac: String, settled: MoveSettled) -> Result<(), Refusal> {
        let id = settled.session_id.as_str().to_string();
        let marked = self.hosts().of(&id).state().to_mac.clone();
        let (session, hosting) = self.session_and_hosting(&id).await?;
        let ours = marked.as_deref() == Some(mac.as_str()) || (marked.is_none() && hosting.paused_at.is_some());
        if !ours || session.state == store::SessionState::Ended {
            return Err(self.not_here(&id));
        }
        // On a task of its own, as a close is: ending or resuming a Job inside the act's call outgrows a worker's stack.
        let fleet = Arc::clone(self);
        tokio::spawn(async move {
            let ended = match settled.refused.clone() {
                Some(why) => {
                    Box::pin(fleet.move_undone(&id, &mac, &why)).await;
                    Ok(())
                }
                None => Box::pin(fleet.moved_away(&id, &mac, settled)).await,
            };
            if let Err(why) = ended {
                fleet.session_warned(&id, "a Job that moved to another Mac was not ended here", why.error().message.clone());
            }
        });
        Ok(())
    }

    /// The Job taken by Mac `mac`: ended here, and what was kept for it while it was handed over sent on.
    async fn moved_away(self: &Arc<Self>, id: &str, mac: &str, settled: MoveSettled) -> Result<(), Refusal> {
        let (id, mac) = (id.to_string(), mac.to_string());
        // What was sent to it while it was handed over goes on to it, in the order it came.
        let kept = self.store().lock().await.take_held_later_turns(&id).unwrap_or_default();
        self.closed_and_parked(CloseSession { session_id: settled.session_id.clone() }).await?;
        {
            let mut store = self.store().lock().await;
            if let Ok(Some(mut row)) = store.session(&id) {
                row.end_reason = Some(MOVED_TO_MAC.to_string());
                store.keep_session(&row).map_err(|why| self.ledger_fault(why))?;
            }
        }
        {
            let runtime = self.hosts().of(&id);
            let mut state = runtime.state();
            state.to_mac = None;
            state.carrying_to_mac = false;
        }
        // Its row here goes, and the row the other Mac shares stands in its place.
        self.publish(ipc::Event::SessionRemoved(ipc::SessionRemoved { session_id: settled.session_id.clone() }));
        for turn in kept.iter().filter_map(|body| first_turn_of(body)) {
            let message = SendSessionMessage { session_id: settled.session_id.clone(), text: turn.text, attachments: Vec::new(), mentions: turn.tags, sketches: Vec::new() };
            let act = JobAct::Message(SendSessionMessageBody { message, aim: None });
            if let Err(why) = self.acted_on_other_mac(mac.clone(), act).await {
                self.session_warned(&id, "a message kept while the Job moved was not sent on", why.error().message.clone());
            }
        }
        Ok(())
    }

    /// `share_job_conversation`: a piece of Job `job`'s conversation for Mac `mac`, which it is moving to.
    pub(crate) async fn conversation_shared(&self, mac: String, job: SessionId, from: u64) -> Result<Vec<u8>, Refusal> {
        let moving = {
            let runtime = self.hosts().of(job.as_str());
            let state = runtime.state();
            state.carrying_to_mac && state.to_mac.as_deref() == Some(mac.as_str())
        };
        if !moving {
            return Err(self.not_here(job.as_str()));
        }
        let (home, id) = (self.host().home.clone(), job.as_str().to_string());
        let read = tokio::task::spawn_blocking(move || adapters::terminal_thread::conversation_piece(&home, &id, from, CONVERSATION_PIECE))
            .await
            .map_err(|why| self.hosted_fault(SESSION_NOT_MOVABLE, &why.to_string()))?;
        match read {
            Ok(piece) => Ok(piece.unwrap_or_default()),
            Err(why) => Err(self.hosted_fault(SESSION_NOT_MOVABLE, &why.to_string())),
        }
    }

    // ---- the Mac it goes to -------------------------------------------------------------------------------

    /// `act_from_mac`'s `arrive`, checked and answered. **The share filter has passed it** (`act_from_mac_begun`).
    pub(crate) async fn arrived_from(self: &Arc<Self>, mac: String, job: ArrivingJob) -> Result<SessionRecord, Refusal> {
        let id = job.record.id.as_str().to_string();
        let live = self.store().lock().await.session(&id).map_err(|why| self.ledger_fault(why))?;
        if live.is_some_and(|one| one.state != store::SessionState::Ended) {
            return Err(Refusal::IllegalMove(WireError::raised(ALREADY_HERE, format!("job {id} is on this Mac already"), self.run_id())));
        }
        let manifest = job.record.manifest_id.clone().ok_or_else(|| self.not_here(&id))?;
        self.served_named(Some(&manifest))?;
        let record = job.record.clone();
        let fleet = Arc::clone(self);
        tokio::spawn(async move {
            // Boxed, as the hand-over is: the resume starts a process.
            let refused = Box::pin(fleet.arriving(&mac, job)).await.err();
            if let Some(why) = &refused {
                fleet.session_warned(&id, "a Job moving here from another Mac was refused", why);
            }
            let settled = MoveSettled { session_id: SessionId::carried(&id), refused };
            if let Err(why) = fleet.acted_on_other_mac(mac, JobAct::MoveSettled(settled)).await {
                fleet.session_warned(&id, "the Mac a Job moved from was not told how it went", why.error().message.clone());
            }
        });
        Ok(record)
    }

    /// The conversation read and checked, the Job kept under its own id, paused on its branch, and resumed.
    async fn arriving(self: &Arc<Self>, mac: &str, job: ArrivingJob) -> Result<(), String> {
        let id = job.record.id.as_str().to_string();
        let said = |refusal: Refusal| refusal.error().message.clone();
        let conversation = self.conversation_read_from(mac, &id, &job.conversation).await?;
        let manifest = job.record.manifest_id.clone().ok_or_else(|| format!("job {id} names no repository"))?;
        let served = self.served_named(Some(&manifest)).map_err(said)?;
        if !conversation.is_empty() {
            let (home, root, id) = (self.host().home.clone(), served.root().to_string(), id.clone());
            tokio::task::spawn_blocking(move || adapters::terminal_thread::keep_conversation(&home, &id, &root, &conversation))
                .await
                .map_err(|why| why.to_string())?
                .map_err(|why| format!("the conversation was not kept: {why}"))?;
        }
        self.arrived_kept(&job, served.root(), !job.conversation.bytes.eq(&0)).await.map_err(said)?;
        self.session_resumed_telling(ResumeSession { session_id: job.record.id.clone() }, &moved_note(mac)).await.map_err(said)?;
        Ok(())
    }

    /// Job `id`'s conversation, read from Mac `mac` in pieces and checked against what it said it sent.
    async fn conversation_read_from(&self, mac: &str, id: &str, carried: &CarriedConversation) -> Result<Vec<u8>, String> {
        let switch = self.phone_switch().map_err(|why| why.error().message.clone())?;
        let mut bytes = Vec::with_capacity(carried.bytes as usize);
        while (bytes.len() as u64) < carried.bytes {
            let path = format!("/other_mac/conversation?session_id={}&from={}", ipc::door::encoded(id), bytes.len());
            let (status, piece) = switch.read_other_mac(mac, &path).await?;
            if !(200..300).contains(&status) {
                return Err(self.refusal_passed_on(mac, status, &piece).error().message.clone());
            }
            if piece.is_empty() {
                break;
            }
            bytes.extend_from_slice(&piece);
        }
        let whole = bytes.len() as u64 == carried.bytes && hex(&Sha256::digest(&bytes)) == carried.sha256;
        match whole {
            true => Ok(bytes),
            false => Err(format!("the conversation did not arrive whole: {} of {} bytes", bytes.len(), carried.bytes)),
        }
    }

    /// The arriving Job's ledger row and hosting, under its own id, paused on its pushed branch so its Resume
    /// leases a slot onto it. Its row keeps when it started, so it keeps its place in the list; the pull
    /// requests it held are its here, so their notices reach it.
    async fn arrived_kept(&self, job: &ArrivingJob, root: &str, ran: bool) -> Result<(), Refusal> {
        let record = &job.record;
        let id = record.id.as_str().to_string();
        let hosted = record.hosted.clone();
        let now = self.now().as_str().to_string();
        let manifest = record.manifest_id.as_ref().map(|one| one.as_str().to_string()).unwrap_or_default();
        let project = hosted.as_ref().and_then(|one| one.project.clone());
        // A project held as a copy has no layer of values here, so only the repository's are frozen.
        let own = project.as_deref().filter(|id| self.project_own(id));
        let values = self.values_to_freeze(&manifest, own).await;
        {
            let mut store = self.store().lock().await;
            store
                .keep_session(&KeptSession {
                    id: id.clone(),
                    harness: String::from(adapters::HOSTED_HARNESS),
                    origin: String::from("bridge"),
                    manifest_id: Some(manifest.clone()),
                    cwd: root.to_string(),
                    title: record.title.clone(),
                    state: store::SessionState::Live,
                    started_at: record.started_at.as_str().to_string(),
                    last_seen_at: now.clone(),
                    last_turn_at: record.last_turn_at.as_ref().map(|at| at.as_str().to_string()),
                    ended_at: None,
                    end_reason: None,
                    figures: Default::default(),
                    mod_version: None,
                })
                .map_err(|why| self.ledger_fault(why))?;
            store
                .keep_hosting(&KeptHosting {
                    session_id: id.clone(),
                    manifest_id: manifest.clone(),
                    model: hosted.as_ref().and_then(|one| one.model.clone()),
                    effort: hosted.as_ref().and_then(|one| one.effort.clone()),
                    mode: mode_text(hosted.as_ref().map(|one| one.mode).unwrap_or_default()).to_string(),
                    ran,
                    lease_slot: None,
                    lease_branch: job.branch.clone(),
                    fork_of: None,
                    on_your_words: false,
                    setup: false,
                    workflow: None,
                    proposed: String::new(),
                    turn_model: None,
                    turn_effort: None,
                    paused_at: Some(now.clone()),
                    paused_resuming: false,
                    auto_land: false,
                    mod_job: false,
                    start_held: false,
                    workflow_chosen: false,
                    from_job: None,
                    from_tab: None,
                    mod_of_bay: false,
                    project,
                    side_job: false,
                    terminal_dir: None,
                })
                .map_err(|why| self.ledger_fault(why))?;
            let _ = store.mark_lifecycle_job(&id, &now);
            for pull in record.attachments.iter().filter(|one| one.kind == "pr" && one.state != ipc::AttachmentState::GivenBack) {
                let held = KeptAttachment {
                    holder: Holder::session(&id),
                    kind: pull.kind.clone(),
                    manifest_id: pull.manifest_id.as_ref().map(|one| one.as_str().to_string()).unwrap_or_default(),
                    target: pull.target.clone(),
                    state: AttachmentState::Standing,
                    detail: pull.detail.clone(),
                    since: now.clone(),
                    changed_at: now.clone(),
                };
                store.attach(&held, false).map_err(|why| self.ledger_fault(why))?;
            }
        }
        if let Some(values) = &values {
            self.job_values_frozen(&id, values).await;
        }
        let _ = self.published_hosted(&id).await;
        Ok(())
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
