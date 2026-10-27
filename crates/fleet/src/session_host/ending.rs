//! A terminal session ending: End session held for its mod, and the slot
//! parked once the terminal's process has gone, however it ended. `docs/concepts/session.md`,
//! *Ending a terminal session*.

use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{AgentHarness, Delivery, SlotParkRefused, Vcs, WorkProduct};
use api::Refusal;
use ipc::{CloseSession, SessionRecord, SessionTurn, WireError};
use store::{Holder, KeptHosting, KeptSession, SessionState};

use super::serving::SLOT_KINDS;
use super::terminal::{LISTENING_FOR, TERMINAL_UNREACHABLE};
use crate::daemon::Fleet;

/// The first `armada` mod that runs an end Fleet holds. An older one drops it
/// unread, so End session on its terminal is refused rather than held for
/// nothing; `/reload-plugins` after an update brings it up to this.
const ENDS_FROM: &str = "0.3.10";

/// How long after an ended session's report its slot's process may take to
/// go, and how often the pool is asked meanwhile. The mod reports from inside
/// the engine's exit, so the process is still there when the report lands.
const PARK_WITHIN: Duration = Duration::from_secs(10);
const PARK_EVERY: Duration = Duration::from_millis(250);

/// The end reasons after which the terminal's process goes on, under another
/// session id and in the same directory: a `/clear` and a resume. Its slot is
/// its own still, so it is not parked.
const GOES_ON: [&str; 2] = ["clear", "resume"];

/// Whether a terminal session that ended for `reason` has its slot parked:
/// the person left (`prompt_input_exit`), the window closed or the process was
/// signalled (`other`), or a logout. A crash reports no end at all, so its slot
/// stays held for a resume. The park still waits for the process to be gone.
pub(crate) fn parks_on_end(reason: &str) -> bool {
    !GOES_ON.contains(&reason)
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
    /// End session on a terminal session: the end is held for its mod, which
    /// runs `/exit` after any message held ahead of it. **Nothing ends here**:
    /// the row ends when the terminal reports it ended, and the slot is parked
    /// only after that (`park_once_gone`). A terminal not listening is refused,
    /// with nothing ended and nothing parked.
    pub(crate) async fn end_terminal(&self, session: &KeptSession) -> Result<SessionRecord, Refusal> {
        if session.state != SessionState::Ended {
            if crate::sessioning::mod_is_older(session.mod_version.as_deref(), ENDS_FROM) {
                return Err(self.mod_cannot_end());
            }
            if !self.hosts().terminals().hold_end(&session.id, LISTENING_FOR) {
                return Err(self.terminal_unreachable());
            }
        }
        let store = self.store().lock().await;
        self.ledger_row(&store, session)
    }

    /// Park the slot a terminal session stood in, once the process holding it
    /// has gone, so nothing can still be writing into it: a park the pool
    /// refuses because its holder still runs is asked again for
    /// [`PARK_WITHIN`], and a holder that outlives that (a person's own
    /// terminal, another session's lease) is left alone.
    pub(crate) fn park_once_gone(&self, session: &str, manifest: &str, slot: u32) {
        let Ok(served) = self.served_named(Some(&ipc::ManifestId::carried(manifest))) else {
            return;
        };
        let pool = crate::leasing::pool_of(&served);
        let vcs = Arc::clone(self.vcs());
        let queue = Arc::clone(self.hosts().slot_queue());
        let servers = self.servers().clone();
        let path = adapter_traits::slot_path(served.root(), slot);
        let session = session.to_string();
        tokio::spawn(async move {
            crate::servers::stopped_in(servers, &path).await;
            let until = tokio::time::Instant::now() + PARK_WITHIN;
            loop {
                match vcs.park_ended_session_slot(&pool, slot, &session) {
                    Err(
                        SlotParkRefused::HeldByAnother(_)
                        | SlotParkRefused::Busy,
                    ) if tokio::time::Instant::now() < until => {
                        tokio::time::sleep(PARK_EVERY).await;
                    }
                    _ => break,
                }
            }
            queue.freed();
        });
    }

    /// What a session that ended leaves in Fleet's hands: the Check runs it
    /// still had out, and the walk it had open, and what it was doing, which
    /// is dropped, the sign-in held for its window, its live watches, its shell and its scratch folder. Its slot's processes and
    /// servers are ended by the park (`Vcs::park_hosted_slot`), and a run's
    /// process is among them, since Fleet holds no pid for a run its CLI makes.
    pub(crate) async fn session_wound_up(&self, session: &str) {
        self.logbook_distils().queue(session);
        self.activity_ended(session);
        self.window_sign_ins().forget(session);
        self.session_checks_ended(session).await;
        self.watches_ended_with(session).await;
        self.job_shell_wound_up(session);
        self.scratch_removed(session);
        self.project_tasks_freed(session).await;
        if let Some(url) = self.walk_open(session).await {
            self.walk_ended(session, &url).await;
        }
    }

    /// The fast half of a close: the row says `Ended`, whatever else the session holds is given
    /// back, and the record is answered. **What is owed** is the hosting of a session whose slots
    /// are still to be parked ([`Self::parked_after_close`]); a terminal session, or one already
    /// ended, owes nothing.
    pub(crate) async fn session_ended(
        &self,
        closed: CloseSession,
    ) -> Result<(SessionRecord, Option<(String, KeptHosting)>), Refusal> {
        let id = closed.session_id.as_str().to_string();
        if let Some(session) = self.terminal_session(&id).await? {
            return Ok((self.end_terminal(&session).await?, None));
        }
        let (session, hosting) = self.session_and_hosting(&id).await?;
        if session.state == SessionState::Ended {
            return Ok((self.published_hosted(&id).await?, None));
        }
        {
            let runtime = self.hosts().of(&id);
            let mut state = runtime.state();
            if let Some(asked) = state.asked.take() {
                self.hosts().asks().withdraw(&asked.call);
            }
            Self::let_go(&mut state);
            state.turn = SessionTurn::Idle;
            state.moving = None;
            state.allowed_folders.clear();
            state.allowed_kinds.clear();
        }
        // Out of the line if it was in it: an ended session is granted nothing.
        self.hosts().slot_queue().leave(&id);
        let now = self.now().as_str().to_string();
        {
            let mut store = self.store().lock().await;
            let mut row = session;
            row.state = SessionState::Ended;
            row.ended_at = Some(now.clone());
            row.end_reason = Some(String::from("closed"));
            row.last_seen_at = now.clone();
            store.keep_session(&row).map_err(|why| self.ledger_fault(why))?;
            // A slot stays on the ledger as held until it is parked, or it is stranded with
            // nothing saying so. A restart before then leaves it for the boot pass
            // (`freed_from_ended_sessions`).
            let holder = Holder::session(&id);
            let held = store.attachments_of(&holder).map_err(|why| self.ledger_fault(why))?;
            for kind in held.iter().map(|one| one.kind.as_str()).filter(|kind| !SLOT_KINDS.contains(kind)) {
                store.give_back(&holder, Some(kind), &now).map_err(|why| self.ledger_fault(why))?;
            }
        }
        self.session_wound_up(&id).await;
        self.writes().left(&crate::clashing::Who::Session(id.clone()));
        Ok((self.published_hosted(&id).await?, Some((id, hosting))))
    }

    /// The slow half of a close: every slot the session holds is parked, and only then does the
    /// line move, so no other Job takes a slot mid-park. A slot the pool still holds for it stays
    /// on the ledger as held, for the boot pass.
    pub(crate) async fn parked_after_close(&self, id: &str, hosting: KeptHosting) {
        let still_held = match self.served_named(Some(&ipc::ManifestId::carried(&hosting.manifest_id))) {
            // Parked: what is in each slot is committed to the branch it is on and nothing is
            // pushed, so closing loses no work.
            Ok(served) => self.park_every_slot_of(id, &served, hosting.lease_slot).await,
            Err(_) => Vec::new(),
        };
        self.hosts().slot_freed();
        if still_held.is_empty() {
            let now = self.now().as_str().to_string();
            let _ = self.store().lock().await.give_back(&Holder::session(id), None, &now);
        }
        let _ = self.published_hosted(id).await;
    }

    /// [`HostedSessions::close_session`] for a caller that has no `Arc` to spawn with, or needs the
    /// slots parked by the time it goes on: the whole close, awaited.
    pub(crate) async fn closed_and_parked(&self, closed: CloseSession) -> Result<SessionRecord, Refusal> {
        let (ended, owed) = self.session_ended(closed).await?;
        let Some((id, hosting)) = owed else {
            return Ok(ended);
        };
        self.parked_after_close(&id, hosting).await;
        self.published_hosted(&id).await
    }

    /// A mod too old to run an end is unreachable for it: the same code, and
    /// the same remedy Bridge names, `/reload-plugins` in that session.
    fn mod_cannot_end(&self) -> Refusal {
        Refusal::IllegalMove(WireError::raised(
            TERMINAL_UNREACHABLE,
            "that job's armada mod is too old to end it from Bridge. Run /reload-plugins in \
             it, or end it there",
            self.run_id(),
        ))
    }
}
