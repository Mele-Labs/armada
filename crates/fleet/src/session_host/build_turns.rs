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
fn held_said(leader: Option<&str>, why: &str) -> String {
    let leader = leader.map_or_else(String::new, |job| format!(" while job {job}'s builds run"));
    format!(
        "Your builds are paused because {why}, and run again on their turn{leader}. \
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
    /// Hold every live Job's builds but one while the machine is short by
    /// [`crate::limits::gate_limits`], and let them go when it recovers. Logs each change of turn.
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
        let limits = crate::limits::gate_limits(&self.machine_settings().now());
        let fleet = Arc::clone(self);
        let looked = tokio::task::spawn_blocking(move || {
            let short = limits.short(&checks_runner::machine_reading());
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
            let held_before = turns.jobs_held();
            let now = Instant::now();
            let loaded = turns.loaded(short, now);
            let why = match short {
                Some(checks_runner::Short::Load { load, limit, window }) => format!("the {window} load is {load}, at or over {limit}"),
                _ => "memory is short".to_string(),
            };
            let signals = turns.look(
                loaded,
                &contenders,
                &listing,
                |pid| peers.carries(pid, checks_runner::HELD_ENV, "1"),
                |pid, job| peers.carries(pid, Holder::SESSION_ENV, job),
                now,
                TURN,
            );
            send(&signals);
            let leader = turns.leader().map(str::to_string);
            Some((held_before, turns.jobs_held(), leader, why))
        })
        .await;
        let Ok(Some((before, after, leader, why))) = looked else {
            return;
        };
        // Told so its agent waits on a build that went quiet, rather than starting it again.
        for job in after.difference(&before) {
            let text = held_said(leader.as_deref(), &why);
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
