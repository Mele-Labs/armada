//! The loop beside the Sessions that Autopilot's picks, plan limits and the clash
//! watch ride on.

use std::sync::Arc;
use std::time::Duration;

use tokio::task::JoinHandle;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};

use crate::adrift::Adrift;
use crate::daemon::Fleet;

/// Run the passes that wait on time every `tick`; their own intervals still gate them. Starts
/// Autopilot's passes beside it: the picks, and the decisions it answers.
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
    crate::autopilot::keep_deciding(Arc::clone(&fleet), tick, Arc::clone(&adrift));
    // The Operator rules on what arrives from here on, with one call already started.
    fleet.operator_seated();
    // The moves into a slot live in memory, so a wait a restart cut off ended with it, but for the
    // moves owed again to a process reattached in the checkout. The line is kept, and read back
    // after, so its waits begin again.
    let waits = Arc::clone(&fleet);
    tokio::spawn(async move {
        waits.slot_waits_ended_at_boot().await;
        waits.slot_lines_restored().await;
        waits.operator_orders_read().await;
    });
    // Away for longer than a few missed beats is an outage, which a stop says it was lost to.
    let started = Arc::clone(&fleet);
    tokio::spawn(async move {
        let at = started.now().as_str().to_string();
        let _ = started.store().lock().await.fleet_started(&at, crate::session_host::stopping::OUTAGE_SECONDS);
    });
    // The week's transcripts, read once and in the background, a while after Fleet is up.
    let backfill = Arc::clone(&fleet);
    tokio::spawn(async move {
        tokio::time::sleep(crate::turn_ingest::BACKFILL_AFTER).await;
        backfill.turns_backfilled(false).await;
    });
    // Where each ended Job's time went, kept as they end and, the first time, for those that ended before.
    let timing = Arc::clone(&fleet);
    tokio::spawn(async move {
        tokio::time::sleep(crate::job_timing::AFTER).await;
        loop {
            timing.jobs_timed().await;
            tokio::time::sleep(crate::job_timing::EVERY).await;
        }
    });
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(tick);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        let mut slept = tokio::time::Instant::now();
        let mut followed = tokio::time::Instant::now();
        let mut measured = tokio::time::Instant::now();
        let mut disputes_read = tokio::time::Instant::now();
        let mut pages_read = tokio::time::Instant::now();
        let mut stops_swept: Option<tokio::time::Instant> = None;
        let mut alive = tokio::time::Instant::now();
        loop {
            ticker.tick().await;
            if alive.elapsed() >= crate::session_host::stopping::BEAT {
                alive = tokio::time::Instant::now();
                let at = fleet.now().as_str().to_string();
                let _ = fleet.store().lock().await.keep_fleet_alive(&at);
            }
            // A subagent its parent sent more work, read as running again from its own transcript.
            if followed.elapsed() >= crate::subagents_resumed::EVERY {
                followed = tokio::time::Instant::now();
                fleet.subagents_followed().await;
            }
            // What the live sessions' transcripts gained, kept as usage a turn at a time.
            if measured.elapsed() >= crate::turn_ingest::EVERY {
                measured = tokio::time::Instant::now();
                fleet.turns_followed_and_read().await;
            }
            // A Fleet notice the job pushed back on in its replies, without calling dispute_fleet.
            if disputes_read.elapsed() >= crate::session_host::dispute_reading::EVERY {
                disputes_read = tokio::time::Instant::now();
                fleet.notices_read().await;
            }
            // A page put to him that he answered in the thread rather than on its card.
            if pages_read.elapsed() >= crate::session_host::page_replies::EVERY {
                pages_read = tokio::time::Instant::now();
                fleet.pages_read().await;
            }
            // A free bay, while a repository's Autopilot runs, re-measures what main's moves made stale.
            if slept.elapsed() >= crate::test_map_feed::EVERY {
                slept = tokio::time::Instant::now();
                fleet.test_map_slept_over();
            }
            // Sessions the plan's limit stopped, woken once it has reset.
            fleet.limits_lifted().await;
            // Jobs a terminal Job's answer signed back in, woken with `Continue`.
            fleet.sign_in_wakes_owed().await;
            // A turn that went quiet is stuck by time passing, which no event says: published when it does.
            fleet.standing_checked().await;
            // A stop nothing excuses, told once a minute after it: every live Job read at start and now and then.
            if stops_swept.is_none_or(|at| at.elapsed() >= crate::session_host::stopping::SWEEP) {
                stops_swept = Some(tokio::time::Instant::now());
                fleet.stops_swept().await;
            }
            fleet.stops_told().await;
            // The Operator's call kept started, or stopped, as its switch now reads.
            fleet.operator_kept_in_step().await;
            // A bug report, a new issue or a note left, handed to the Operator once.
            fleet.sources_handed().await;
            // Who wrote the same file as whom, once the wait after a write is over.
            fleet.clashes_checked().await;
            // The clashes of every repository, pushed when the set moved.
            fleet.clashes_published().await;
            // What main's moves made stale in the test map, read again where that costs nothing.
            fleet.test_map_moves_fed();
            // A Session that ended or handed on, distilled into the Logbook one at a time.
            fleet.logbook_distils_run();
            // The open lessons that are repository facts, moved into the Logbook once per store.
            fleet.logbook_backlog_run();
            // Near-duplicate entries folded together, a repository at most once a week.
            fleet.logbook_tidy_run();
        }
    })
}
