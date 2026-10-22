//! Watches: a command Fleet runs on a cadence for one Job, until what it follows is finished.
//! `docs/design/fleet-watches-ci-and-deploys-for-a-job.md`.
//!
//! **The store holds every watch**, its readings, set-asides, verdicts and what was told of it, so
//! a restart carries on (`store::KeptWatch`). Memory holds only the commands running now.
//! `serving` answers the four operations, `running` takes readings on a timer of its own, and
//! `telling` tells the Job once per change and ends its watches with it.

mod running;
mod serving;
mod telling;

use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard, PoisonError};

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use core_model::{ReadingState, WatchState};
use ipc::{
    Instant, WatchList, WatchPart, WatchReading, WatchReadingState, WatchSetAside, WatchSummary, WatchVerdictGiven,
    WatchVerdictWord, WireError,
};
use store::{KeptWatch, KeptWatchReading, KeptWatchTaken, KeptWatchWhole};

use crate::daemon::Fleet;

pub use running::{keep_running_watches, EVERY};
#[cfg(test)]
pub(crate) use running::interval;
pub(crate) use telling::owed;
#[cfg(test)]
pub(crate) use telling::notice;

/// Nothing places a Job, by the connection or the body. A 422.
pub(crate) const UNPLACED: &str = "fleet.watches_unplaced";
/// A body that decoded and cannot be a watch: a blank name or item, a span that is not one. A 422.
pub(crate) const REFUSED: &str = "fleet.watch_refused";
/// The Job already holds `limits.watchesPerJob` live watches. A 409.
pub(crate) const LIMIT_REACHED: &str = "fleet.watch_limit_reached";
/// No recipe of that name in the Job's repository or on this machine. A 422.
pub(crate) const RECIPE_UNKNOWN: &str = "fleet.watch_recipe_unknown";
/// A recipe's value missing, not named by its command, blank, or holding a NUL. A 422.
pub(crate) const VALUE_REFUSED: &str = "fleet.watch_value_refused";
/// `every` below `timeouts.watchEverySeconds`. A 422.
pub(crate) const TOO_OFTEN: &str = "fleet.watch_too_often";
/// `gives_up_after` past the day `timeouts.watchGiveUpMinutes` allows at most. A 422.
pub(crate) const GIVES_UP_TOO_LATE: &str = "fleet.watch_gives_up_too_late";
/// No watch by that id, or one another Job holds. A 404.
pub(crate) const NO_SUCH_WATCH: &str = "fleet.no_such_watch";
/// A verdict on a watch that has ended. A 409.
pub(crate) const ENDED: &str = "fleet.watch_ended";
/// `ours` on a watch that has not failed. A 409.
pub(crate) const NOT_FAILED: &str = "fleet.watch_not_failed";
/// `not_ours` naming a part no reading of the watch carried. A 422.
pub(crate) const PART_UNREPORTED: &str = "fleet.watch_part_unreported";

/// The most a watch may run before giving up: the ceiling of `timeouts.watchGiveUpMinutes`'
/// entry, a day, so lowering the default never lowers the cap.
pub(crate) const GIVE_UP_CAP_SECONDS: u32 = 24 * 60 * 60;

/// Broken readings in a row that end a watch `broken`.
pub(crate) const BROKEN_IN_A_ROW: usize = 3;

/// The commands running now, each with what stops it. **Memory only**: a run a restart cut short
/// left no reading, so its watch is simply due again, and `crate::orphans` ends its process.
#[derive(Default)]
pub(crate) struct Watches {
    running: Mutex<HashMap<String, tokio::sync::oneshot::Sender<()>>>,
    /// When a notice a Job could not take was last offered, so it is offered a minute apart.
    offered: Mutex<HashMap<String, core_model::Timestamp>>,
    /// When a reading that read the same was last published, by watch.
    republished: Mutex<HashMap<String, core_model::Timestamp>>,
}

impl Watches {
    fn running(&self) -> MutexGuard<'_, HashMap<String, tokio::sync::oneshot::Sender<()>>> {
        self.running.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub(crate) fn running_now(&self) -> usize {
        self.running().len()
    }

    pub(crate) fn is_running(&self, id: &str) -> bool {
        self.running().contains_key(id)
    }

    /// Hold `id`'s place among the commands running; what completes when it is stopped. `None`
    /// where it already runs.
    pub(crate) fn begin(&self, id: &str) -> Option<tokio::sync::oneshot::Receiver<()>> {
        let mut running = self.running();
        if running.contains_key(id) {
            return None;
        }
        let (stop, stopped) = tokio::sync::oneshot::channel();
        running.insert(id.to_string(), stop);
        Some(stopped)
    }

    pub(crate) fn finished(&self, id: &str) {
        self.running().remove(id);
    }

    /// End `id`'s command, if one runs.
    pub(crate) fn stop(&self, id: &str) {
        if let Some(stop) = self.running().remove(id) {
            let _ = stop.send(());
        }
    }

    /// Whether a notice for `id` may be offered at `now`: never before, or a minute since.
    pub(crate) fn may_offer(&self, id: &str, now: &core_model::Timestamp) -> bool {
        self.once_in(&self.offered, id, now, std::time::Duration::from_secs(60))
    }

    /// Whether a reading of `id` that read the same may be published at `now`: five minutes apart.
    pub(crate) fn may_republish(&self, id: &str, now: &core_model::Timestamp) -> bool {
        self.once_in(&self.republished, id, now, std::time::Duration::from_secs(5 * 60))
    }

    fn once_in(
        &self,
        last: &Mutex<HashMap<String, core_model::Timestamp>>,
        id: &str,
        now: &core_model::Timestamp,
        apart: std::time::Duration,
    ) -> bool {
        let mut last = last.lock().unwrap_or_else(PoisonError::into_inner);
        let due = last.get(id).is_none_or(|then| crate::clock::elapsed(then, now) >= apart);
        if due {
            last.insert(id.to_string(), now.clone());
        }
        due
    }
}

fn reading_state(word: &str) -> ReadingState {
    ReadingState::from_wire(word).unwrap_or(ReadingState::Waiting)
}

fn wire_reading_state(word: &str) -> WatchReadingState {
    reading_state(word).into()
}

/// The domain's watch from what was kept: the last reading that read, not the last that broke.
pub(crate) fn domain(kept: &KeptWatchWhole) -> core_model::Watch {
    let held = &kept.watch;
    let mut watch =
        core_model::Watch::new(held.name.clone(), held.command.clone(), held.every_seconds, held.gives_up_after_seconds);
    watch.follows = held.follows;
    watch.set_aside = kept
        .set_aside
        .iter()
        .map(|aside| core_model::SetAside { part: aside.part.clone(), sentence: aside.sentence.clone() })
        .collect();
    watch.reported = held.reported.clone();
    watch.last = kept.readings.iter().rev().find_map(|one| match &one.taken {
        KeptWatchTaken::Read { state, finished, says, link, parts } => Some(core_model::Reading {
            state: reading_state(state),
            finished: *finished,
            says: says.clone(),
            link: link.clone(),
            parts: parts
                .iter()
                .map(|(name, state)| core_model::Part { name: name.clone(), state: reading_state(state) })
                .collect(),
        }),
        KeptWatchTaken::Broken { .. } => None,
    });
    watch.state = WatchState::from_wire(&held.state).unwrap_or(WatchState::Waiting);
    watch.live = held.live;
    watch
}

/// `kept` once `watch` has moved: its state, whether it is live, the parts reported, and when it
/// ended.
pub(crate) fn moved(kept: &KeptWatch, watch: &core_model::Watch, now: &core_model::Timestamp) -> KeptWatch {
    let mut next = kept.clone();
    next.state = watch.state.as_wire().to_string();
    next.live = watch.live;
    next.reported = watch.reported.clone();
    if kept.live && !watch.live {
        next.ended_at = Some(now.as_str().to_string());
    }
    next
}

fn wire_reading(one: &KeptWatchReading) -> WatchReading {
    let at = Instant::carried(one.at.clone());
    match &one.taken {
        KeptWatchTaken::Read { state, finished, says, link, parts } => WatchReading::Read {
            at,
            state: wire_reading_state(state),
            finished: *finished,
            says: says.clone(),
            link: link.clone(),
            parts: parts
                .iter()
                .map(|(name, state)| WatchPart { name: name.clone(), state: wire_reading_state(state) })
                .collect(),
        },
        KeptWatchTaken::Broken { why, output } => WatchReading::Broken { at, why: why.clone(), output: output.clone() },
    }
}

/// What `list_watches` and `watches.changed` say of one watch.
pub(crate) fn summary(kept: &KeptWatchWhole) -> WatchSummary {
    let held = &kept.watch;
    let readings: Vec<WatchReading> = kept.readings.iter().map(wire_reading).collect();
    WatchSummary {
        id: held.id.clone(),
        session_id: ipc::SessionId::carried(&held.session_id),
        name: held.name.clone(),
        command: held.command.clone(),
        recipe: held.recipe.clone(),
        every_seconds: held.every_seconds,
        gives_up_after_seconds: held.gives_up_after_seconds,
        follows: held.follows,
        environment: held.environment.clone(),
        state: WatchState::from_wire(&held.state).unwrap_or(WatchState::Waiting).into(),
        live: held.live,
        started_at: Instant::carried(held.started_at.clone()),
        last: readings.last().cloned(),
        readings,
        set_aside: kept
            .set_aside
            .iter()
            .map(|aside| WatchSetAside { part: aside.part.clone(), sentence: aside.sentence.clone() })
            .collect(),
        verdicts: kept
            .verdicts
            .iter()
            .map(|given| WatchVerdictGiven {
                at: Instant::carried(given.at.clone()),
                verdict: if given.ours { WatchVerdictWord::Ours } else { WatchVerdictWord::NotOurs },
                sentence: given.sentence.clone(),
                parts: given.parts.clone(),
            })
            .collect(),
    }
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
    pub(crate) fn no_such_watch(&self, id: &str) -> Refusal {
        Refusal::NotFound(WireError::raised(NO_SUCH_WATCH, format!("No watch of this Job is {id}."), self.run_id()))
    }

    /// `job`'s watches, the newest first.
    pub(crate) async fn watch_list(&self, job: &str) -> Result<WatchList, Refusal> {
        let kept = self.store().lock().await.watches_of(job).map_err(|why| self.ledger_fault(why))?;
        Ok(WatchList { session_id: ipc::SessionId::carried(job), watches: kept.iter().map(summary).collect() })
    }

    /// Tell Bridge `job`'s watches moved. A list that will not read publishes nothing.
    pub(crate) async fn watches_published(&self, job: &str) {
        if let Ok(list) = self.watch_list(job).await {
            self.publish(ipc::Event::WatchesChanged(list));
        }
    }
}
