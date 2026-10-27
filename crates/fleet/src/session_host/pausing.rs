//! Pausing a hosted Session and resuming it, as a Job is paused (`crate::pausing`): the turn is
//! ended as `stop_session` ends it, the slot's work is committed to the branch (a WIP commit,
//! nothing pushed) and the slot goes back to the pool. The Session stays live.
//! `docs/concepts/session.md`, *Pausing a Session*.
//!
//! **Not a state**: the marker is `KeptHosting::paused_at`, read onto `hosted.paused`. A resume
//! seats the branch at its tip through the Session's ordinary lease (`gate::take_lease`), so a
//! full pool puts it in the repository's line and never takes a slot from another holder.
//!
//! **Only a person's Resume lifts a pause.** No message does: the process of a paused Session is
//! never started (`living::process_of` refuses), a Fleet notice is not told (`fleet_word`), and
//! every other turn, a person's, Helm's or another Job's, is on the thread and kept off the agent
//! in `held_later_turns`, where the start card keeps its own. They are the Job's next turns once
//! it holds a slot again, or Fleet tells it that it was resumed ([`Fleet::pause_lifted`]).

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, SlotStanding, Vcs, WorkProduct};
use api::{HostedSessions, Refusal};
use ipc::{ManifestId, PauseSession, ResumeSession, SessionRecord, StopSession, WireError};
use store::Holder;

use super::gate::Taken;
use super::proposing::Held;
use super::start_card::{first_turn_of, FirstTurn};
use crate::daemon::Fleet;
use crate::leasing::pool_of;

/// A Session that cannot be paused or resumed as it stands: ended, a Helm, or holding no slot.
/// A 409.
const SESSION_NOT_PAUSABLE: &str = "fleet.session_not_pausable";
/// Pausing a Session that is paused. A 409.
const SESSION_ALREADY_PAUSED: &str = "fleet.session_already_paused";
/// Resuming a Session that is not paused. A 409.
const SESSION_NOT_PAUSED: &str = "fleet.session_not_paused";
/// The pool or git would not park the work, or seat the branch. A 409.
const SESSION_PAUSE_REFUSED: &str = "fleet.session_pause_refused";

/// What a Job with no turn kept is told when it is resumed, so it carries on.
const RESUMED: &str = "You were paused and have been resumed. Carry on with what you were doing.";

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
    fn conflict(&self, code: &'static str, said: String) -> Refusal {
        Refusal::IllegalMove(WireError::raised(code, said, self.run_id()))
    }

    /// `pause_session`. **Refused, nothing changes**, except that a pool that refuses the park
    /// after the turn was ended leaves the turn ended, as a stop does.
    pub(crate) async fn session_paused(&self, asked: PauseSession) -> Result<SessionRecord, Refusal> {
        let id = asked.session_id.as_str().to_string();
        if self.terminal_session(&id).await?.is_some() {
            return Err(self.hosted_refusal(
                super::serving::SESSION_COMMAND_REFUSED,
                &format!("job {id} runs in a terminal, and only a hosted job is paused from here"),
            ));
        }
        let (session, mut hosting) = self.session_and_hosting(&id).await?;
        if session.state == store::SessionState::Ended || hosting.helm {
            return Err(self.conflict(SESSION_NOT_PAUSABLE, format!("job {id} has ended or is a Helm, and is not paused")));
        }
        if hosting.paused_at.is_some() {
            // Paused again while it waits for a slot: the wait is called off.
            if !hosting.paused_resuming {
                return Err(self.conflict(SESSION_ALREADY_PAUSED, format!("job {id} is already paused")));
            }
            hosting.paused_resuming = false;
            self.store().lock().await.keep_hosting(&hosting).map_err(|why| self.ledger_fault(why))?;
            self.line_left(&id).await;
            return self.published_hosted(&id).await;
        }
        let Some(slot) = hosting.lease_slot else {
            return Err(self.conflict(SESSION_NOT_PAUSABLE, format!("job {id} holds no slot, so there is nothing to park")));
        };
        let served = self.served_named(Some(&ManifestId::carried(&hosting.manifest_id)))?;
        let pool = pool_of(&served);
        // Asked before the turn is ended: a slot another holds, or one gone, changes nothing.
        let standing = {
            let (vcs, pool, id) = (Arc::clone(self.vcs()), pool.clone(), id.clone());
            tokio::task::spawn_blocking(move || vcs.slot_standing(&pool, slot, &id))
                .await
                .expect("git panicked reading a slot")
        };
        match standing {
            SlotStanding::Held => {}
            SlotStanding::HeldBy(now) => {
                return Err(self.conflict(SESSION_PAUSE_REFUSED, format!("slot {slot} is held by {now}")))
            }
            SlotStanding::Free | SlotStanding::Gone => {
                return Err(self.conflict(SESSION_PAUSE_REFUSED, format!("slot {slot} is not leased to the job")))
            }
        }
        let runtime = self.hosts().of(&id);
        let _one_at_a_time = runtime.lease.lock().await;
        self.stop_session(StopSession { session_id: asked.session_id.clone() }).await?;
        self.stopped_servers_in(&adapter_traits::slot_path(served.root(), slot)).await;
        let parked = {
            let (vcs, pool, id) = (Arc::clone(self.vcs()), pool.clone(), id.clone());
            tokio::task::spawn_blocking(move || vcs.park_hosted_slot(&pool, slot, &id))
                .await
                .expect("git panicked parking a slot")
        }
        .map_err(|why| self.conflict(SESSION_PAUSE_REFUSED, why.said()))?;
        let now = self.now().as_str().to_string();
        let moved_branch = hosting.lease_branch.as_deref() != Some(parked.branch.as_str());
        hosting.lease_slot = None;
        hosting.lease_branch = Some(parked.branch.clone());
        hosting.paused_at = Some(now.clone());
        hosting.paused_resuming = false;
        {
            let mut store = self.store().lock().await;
            store.keep_hosting(&hosting).map_err(|why| self.ledger_fault(why))?;
            let holder = Holder::session(&id);
            let _ = store.give_back(&holder, Some("slot"), &now);
            // The branch row stays: the Session still owns the branch. A `kept/` branch cut
            // for work done on the base replaces it.
            if moved_branch {
                let _ = store.give_back(&holder, Some("branch"), &now);
                let _ = store.attach(
                    &store::KeptAttachment {
                        holder,
                        kind: "branch".into(),
                        manifest_id: hosting.manifest_id.clone(),
                        target: parked.branch.clone(),
                        state: store::AttachmentState::Standing,
                        detail: Default::default(),
                        since: now.clone(),
                        changed_at: now.clone(),
                    },
                    true,
                );
            }
        }
        self.session_slot(&id, session.title.clone(), Some(hosting.manifest_id.clone()), slot, false);
        self.writes().left(&crate::clashing::Who::Session(id.clone()));
        self.hosts().slot_freed();
        self.published_hosted(&id).await
    }

    /// Keep `turn` for the Resume of the Job, if it is paused. **Read under the store lock the Resume
    /// clears the pause in**, so a turn that arrives as it is pressed is kept before the Resume takes
    /// the kept turns, or finds the Job resumed and goes on as any other. The turn is on the thread
    /// already.
    pub(crate) async fn turn_kept_while_paused(&self, id: &str, turn: &Held) -> Result<bool, Refusal> {
        let body = ipc::encode(&FirstTurn::kept_of(turn))
            .map_err(|why| self.hosted_fault(super::serving::SESSION_UNSTARTED, &why.to_string()))?;
        let mut store = self.store().lock().await;
        let paused = store.hosting(id).map_err(|why| self.ledger_fault(why))?.is_some_and(|one| one.paused_at.is_some());
        if paused {
            store.keep_held_later_turn(id, &body).map_err(|why| self.ledger_fault(why))?;
        }
        Ok(paused)
    }

    /// The Job holds a slot again, at Resume or when the line grants one: the turns kept while it
    /// was paused go as its next turns, in the order they came, and with none kept Fleet tells it
    /// that it was resumed.
    pub(crate) async fn pause_lifted(self: &Arc<Self>, id: &str) {
        let kept = self.store().lock().await.take_held_later_turns(id);
        let kept = match kept {
            Ok(kept) => kept,
            Err(why) => {
                self.session_warned(id, "a resumed job's kept turns were not read", format!("{why:?}"));
                return;
            }
        };
        let mut sent = false;
        for turn in kept.iter().filter_map(|body| first_turn_of(body)) {
            let held = match self.held_again(id, turn).await {
                Ok(held) => held,
                Err(why) => {
                    self.session_warned(id, "a kept turn was not read again", format!("{why:?}"));
                    continue;
                }
            };
            sent |= self.deliver_turn(id, held).await.is_ok();
        }
        if !sent {
            let _ = self.told_of_a_resume(id, RESUMED).await;
        }
    }

    /// `resume_session`. **A slot is taken if one is free and waited for if not**; the line is
    /// first come first served and nobody is moved out of a slot for it.
    pub(crate) async fn session_resumed(self: &Arc<Self>, asked: ResumeSession) -> Result<SessionRecord, Refusal> {
        let id = asked.session_id.as_str().to_string();
        if self.terminal_session(&id).await?.is_some() {
            return Err(self.hosted_refusal(
                super::serving::SESSION_COMMAND_REFUSED,
                &format!("job {id} runs in a terminal, and only a hosted job is resumed from here"),
            ));
        }
        let (session, mut hosting) = self.session_and_hosting(&id).await?;
        if session.state == store::SessionState::Ended {
            return Err(self.conflict(SESSION_NOT_PAUSABLE, format!("job {id} has ended")));
        }
        if hosting.paused_at.is_none() {
            return Err(self.conflict(SESSION_NOT_PAUSED, format!("job {id} is not paused")));
        }
        if hosting.paused_resuming {
            return self.published_hosted(&id).await;
        }
        let served = self.served_named(Some(&ManifestId::carried(&hosting.manifest_id)))?;
        let manifest = served.manifest().id().as_str().to_string();
        let taken = match self.hosts().slot_queue().ahead_of(&manifest, &id) {
            0 => self.take_lease_resuming(&id, &served).await,
            _ => Taken::Full,
        };
        match taken {
            Taken::Resumed => {
                self.line_left(&id).await;
                self.pause_lifted(&id).await;
                self.published_hosted(&id).await
            }
            Taken::Held { .. } | Taken::Moving(_) => {
                self.line_left(&id).await;
                self.published_hosted(&id).await
            }
            Taken::Full => {
                hosting.paused_resuming = true;
                self.store().lock().await.keep_hosting(&hosting).map_err(|why| self.ledger_fault(why))?;
                self.line_joined(&manifest, &id).await;
                self.published_hosted(&id).await
            }
            Taken::Refused(why) => Err(self.conflict(SESSION_PAUSE_REFUSED, why)),
            Taken::Gone => Err(self.conflict(SESSION_NOT_PAUSABLE, format!("job {id} has ended"))),
        }
    }
}
