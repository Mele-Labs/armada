//! Each look of the notice loop at whose builds run on a loaded machine. The deciding is
//! [`crate::build_turns`]'s; this reads the Jobs, the load and the process table, and sends.

use std::sync::Arc;
use std::time::Instant;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use adapters::leasing::Holder;

use crate::build_turns::{send, Contender, LISTING, TURN};
use crate::daemon::Fleet;

/// What a Job whose builds run again is told.
const RESUMED: &str = "Your builds run again, from where they stopped.";

/// What a Job whose builds were just held is told: why, for how long, and what not to do.
fn held_said(leader: Option<&str>, load: Option<f64>) -> String {
    let load = load.map_or_else(String::new, |load| format!(" (the fifteen-minute load is {load:.0})"));
    let leader = leader.map_or_else(String::new, |job| format!(" while job {job}'s builds run"));
    format!(
        "Your builds are paused because the machine is loaded{load}, and run again on their turn{leader}. \
         A paused build keeps everything it has done and goes on from where it stopped, so do not start it again. \
         Run a long build in the background, so a pause does not reach your tool's time limit."
    )
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
        let looked = tokio::task::spawn_blocking(move || {
            let load = checks_runner::fifteen_minute_load();
            let loaded = load.is_some_and(|load| load >= f64::from(checks_runner::LOAD_LIMIT));
            let Some(listing) = std::process::Command::new("ps")
                .args(LISTING)
                .output()
                .ok()
                .filter(|said| said.status.success())
                .map(|said| String::from_utf8_lossy(&said.stdout).into_owned())
            else {
                return None;
            };
            let peers = Arc::clone(fleet.peers());
            let mut turns = fleet.build_turns().lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            let was = turns.leader().map(str::to_string);
            let held_before = turns.jobs_held();
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
            Some((held_before, turns.jobs_held(), now, load))
        })
        .await;
        let Ok(Some((before, after, leader, load))) = looked else {
            return;
        };
        // Told so its agent waits on a build that went quiet, rather than starting it again.
        for job in after.difference(&before) {
            let text = held_said(leader.as_deref(), load);
            let _ = self.told_by_fleet(job, &text, ipc::Notice::BuildsHeld).await;
        }
        for job in before.difference(&after) {
            let _ = self.told_by_fleet(job, RESUMED, ipc::Notice::BuildsHeld).await;
        }
    }

    /// Let every build this Fleet holds go: called as Fleet stops, so none is left stopped.
    pub fn builds_released(&self) {
        let mut turns = self.build_turns().lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        send(&turns.released());
    }
}
