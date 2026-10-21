//! The loop beside the Sessions that Autopilot's picks, sleep mode, plan limits and the clash
//! watch ride on.

use std::sync::Arc;
use std::time::Duration;

use tokio::task::JoinHandle;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};

use crate::adrift::Adrift;
use crate::daemon::Fleet;

/// Run the passes that wait on time every `tick`; their own intervals still gate them. Starts
/// Autopilot's pass beside it.
pub fn keep_noticing<H, V, W>(
    fleet: Arc<Fleet<H, V, W>>,
    tick: Duration,
    adrift: impl Fn(Adrift) + Send + Sync + 'static,
) -> JoinHandle<()>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    let adrift = Arc::new(adrift);
    crate::autopilot::keep_autopiloting(Arc::clone(&fleet), tick, Arc::clone(&adrift));
    // The week's transcripts, read once and in the background, a while after Fleet is up.
    let backfill = Arc::clone(&fleet);
    tokio::spawn(async move {
        tokio::time::sleep(crate::turn_ingest::BACKFILL_AFTER).await;
        backfill.turns_backfilled(false).await;
    });
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(tick);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        let mut slept = tokio::time::Instant::now();
        let mut followed = tokio::time::Instant::now();
        let mut measured = tokio::time::Instant::now();
        let mut disputes_read = tokio::time::Instant::now();
        loop {
            ticker.tick().await;
            // A subagent its parent sent more work, read as running again from its own transcript.
            if followed.elapsed() >= crate::subagents_resumed::EVERY {
                followed = tokio::time::Instant::now();
                fleet.subagents_followed().await;
            }
            // What the live sessions' transcripts gained, kept as usage a turn at a time.
            if measured.elapsed() >= crate::turn_ingest::EVERY {
                measured = tokio::time::Instant::now();
                fleet.turns_followed_and_watched().await;
            }
            // A Fleet notice the job pushed back on in its replies, without calling dispute_fleet.
            if disputes_read.elapsed() >= crate::session_host::dispute_reading::EVERY {
                disputes_read = tokio::time::Instant::now();
                fleet.notices_read().await;
            }
            // Sleep mode's pass rides this tick, but looks only every `sleeping::EVERY`.
            if slept.elapsed() >= crate::sleeping::EVERY {
                slept = tokio::time::Instant::now();
                if let Err(why) = Arc::clone(&fleet).sleep_pass().await {
                    eprintln!("sleep mode did not pass over the jobs: {why:?}");
                }
            }
            // Sessions the plan's limit stopped, woken once it has reset.
            fleet.limits_lifted().await;
            // A turn that went quiet is stuck by time passing, which no event says: published when it does.
            fleet.standing_checked().await;
            // Who wrote the same file as whom, once the wait after a write is over.
            fleet.clashes_checked().await;
        }
    })
}
