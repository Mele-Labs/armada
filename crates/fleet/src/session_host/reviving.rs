//! The turn a process died in, and the Fleet that starts it again.
//! `docs/concepts/session/a-job-fleet-hosts.md`, *What a restart does to a job*.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};

use crate::daemon::Fleet;

/// What Fleet waits between two Jobs it starts at boot, so the Mac takes no burst.
const BOOT_START_GAP: std::time::Duration = if cfg!(test) {
    std::time::Duration::from_millis(1)
} else {
    std::time::Duration::from_secs(5)
};

/// What a Job is told when Fleet starts its process again.
const STARTED_AGAIN: &str =
    "Your process ended during a turn and Fleet started it again. Check what your last calls did, then carry on.";

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
    /// A turn began: keep it open until it ends or its process is found dead. A store that will
    /// not keep it fails nothing, as with [`Self::kept_restart`].
    pub(crate) async fn turn_opened(&self, id: &str, at: &str) {
        let _ = self.store().lock().await.turn_opened(id, at);
    }

    /// The turn stopped counting as open without a death: it finished, or was stopped, paused or closed.
    pub(crate) async fn turn_closed(&self, id: &str) {
        let _ = self.store().lock().await.turn_closed(id);
    }

    /// The process of a Job ended with a turn open. **Started once, never in a loop**: a Job
    /// already started again for this turn stays on its row, which is what the person is told of.
    /// Whether the process was started.
    pub(crate) async fn died_with_a_turn_open(self: &Arc<Self>, id: &str) -> bool {
        if self.kept_from_starting(id).await {
            return false;
        }
        let at = self.now().as_str().to_string();
        let again = self.store().lock().await.turn_ended_dead(id, &at);
        let _ = self.published_hosted(id).await;
        if !matches!(again, Ok(0)) {
            return false;
        }
        let _ = self.store().lock().await.turn_started_again(id);
        self.kept_restart(id, "started_again", None).await;
        self.told_of_a_death(id, STARTED_AGAIN).await.unwrap_or(false)
    }

    /// Whether nothing may start this Job by itself, in which case its open turn is closed. A limit's
    /// own wake starts a Job held for one, and only the person's Resume lifts a pause.
    async fn kept_from_starting(&self, id: &str) -> bool {
        let Ok((session, hosting)) = self.session_and_hosting(id).await else {
            return true;
        };
        let held_for_a_limit = matches!(self.store().lock().await.session_limit(id), Ok(Some(_)));
        let kept = session.state == store::SessionState::Ended
            || hosting.paused_at.is_some()
            || hosting.start_held
            || held_for_a_limit;
        if kept {
            self.turn_closed(id).await;
        }
        kept
    }

    /// Start again the Jobs whose turn was open when their process was lost with Fleet down.
    /// **Called once at boot**, after the loops that move Jobs are running.
    pub async fn open_turns_started(self: &Arc<Self>) {
        let rows = self.store().lock().await.open_turns().unwrap_or_default();
        let mut started_one = false;
        for row in rows {
            let id = row.session_id;
            // Reattached: its process is there, and its own events say how the turn ended.
            if self.hosts().of(&id).state().process.is_some() {
                continue;
            }
            // Started again already and still dead: it sits in Needs you, and says nothing twice.
            if row.ended_at.is_some() && row.started_again >= 1 {
                continue;
            }
            if self.kept_from_starting(&id).await {
                continue;
            }
            self.kept_restart(&id, "ended", Some("the job's process was gone when Fleet came back"))
                .await;
            self.fleet_note(&id, "The job's process ended while Fleet was down".to_string())
                .await;
            if started_one {
                tokio::time::sleep(BOOT_START_GAP).await;
            }
            started_one = self.died_with_a_turn_open(&id).await;
        }
    }
}
