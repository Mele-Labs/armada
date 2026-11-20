//! Each look of the notice loop at whose builds run on a loaded machine. The deciding is
//! [`crate::build_turns`]'s; this reads the Jobs, the load and the process table, and sends.

use std::sync::Arc;
use std::time::Instant;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use adapters::leasing::Holder;

use crate::build_turns::{send, Contender, LISTING, TURN};
use crate::daemon::Fleet;

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
    /// Hold every live Job's builds but one while the fifteen-minute load is at or over
    /// [`checks_runner::LOAD_LIMIT`], and let them go when it falls. Logs each change of turn.
    pub(crate) async fn builds_taken_in_turns(self: &Arc<Self>) {
        // A store that will not answer is asked again on the next look.
        let Ok(ids) = self.store().lock().await.live_session_ids_oldest_first() else {
            return;
        };
        let prioritised = self.prioritised_session_id().await;
        // The agent's own pid only: a Job in a terminal has none here, and is found by the
        // `ARMADA_SESSION` its builds carry.
        let contenders: Vec<Contender> = ids
            .into_iter()
            .map(|job| Contender {
                roots: self.hosts().of(&job).state().process.as_ref().and_then(|one| one.pid()).into_iter().collect(),
                prioritised: prioritised.as_deref() == Some(job.as_str()),
                job,
            })
            .collect();
        let fleet = Arc::clone(self);
        let _ = tokio::task::spawn_blocking(move || {
            let load = checks_runner::fifteen_minute_load();
            let loaded = load.is_some_and(|load| load >= f64::from(checks_runner::LOAD_LIMIT));
            let Some(listing) = std::process::Command::new("ps")
                .args(LISTING)
                .output()
                .ok()
                .filter(|said| said.status.success())
                .map(|said| String::from_utf8_lossy(&said.stdout).into_owned())
            else {
                return;
            };
            let peers = Arc::clone(fleet.peers());
            let mut turns = fleet.build_turns().lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            let was = turns.leader().map(str::to_string);
            let signals = turns.look(
                loaded,
                &contenders,
                &listing,
                |pid| peers.carries(pid, checks_runner::HELD_ENV, "1"),
                |pid, job| peers.carries(pid, Holder::SESSION_ENV, job),
                Instant::now(),
                TURN,
            );
            send(&signals);
            let now = turns.leader().map(str::to_string);
            if was != now {
                match (now, load) {
                    (Some(job), Some(load)) => eprintln!(
                        "builds take turns: the fifteen-minute load is {load:.0}, at or over {}, so job {job}'s builds run and the other Jobs' wait",
                        checks_runner::LOAD_LIMIT
                    ),
                    _ => eprintln!("builds take turns no more: every Job's builds run"),
                }
            }
        })
        .await;
    }

    /// Let every build this Fleet holds go: called as Fleet stops, so none is left stopped.
    pub fn builds_released(&self) {
        let mut turns = self.build_turns().lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        send(&turns.released());
    }
}
