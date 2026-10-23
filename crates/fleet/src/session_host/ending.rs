//! A terminal session ending: End session held for its mod, and the slot
//! parked once the terminal's process has gone, however it ended. `docs/concepts/session.md`,
//! *Ending a terminal session*.

use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{AgentHarness, Delivery, SlotParkRefused, Vcs, WorkProduct};
use api::Refusal;
use ipc::{SessionRecord, WireError};
use store::{KeptSession, SessionState};

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
    /// is dropped, and the sign-in held for its window. Its slot's processes and
    /// servers are ended by the park (`Vcs::park_hosted_slot`), and a run's
    /// process is among them, since Fleet holds no pid for a run its CLI makes.
    pub(crate) async fn session_wound_up(&self, session: &str) {
        self.logbook_distils().queue(session);
        self.activity_ended(session);
        self.window_sign_ins().forget(session);
        self.session_checks_ended(session).await;
        if let Some(url) = self.walk_open(session).await {
            self.walk_ended(session, &url).await;
        }
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
