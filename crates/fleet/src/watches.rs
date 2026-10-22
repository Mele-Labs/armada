//! Watches: a command Fleet runs on a cadence for one Job, until what it follows is finished.
//! `docs/design/fleet-watches-ci-and-deploys-for-a-job.md`.
//!
//! **Recorded, not yet run.** A start is checked against its recipe and the limits and kept in
//! memory; a weighing and a stop act on what is kept. Nothing runs a command, takes a reading,
//! tells a Job or survives a restart. Each `// next group:` line names where that lands.

mod serving;

use ipc::{Instant, WatchSetAside, WatchSummary};

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

/// One watch Fleet holds, and what the domain type does not: whose it is and where it came from.
#[derive(Clone, Debug)]
pub(crate) struct Held {
    pub(crate) id: String,
    pub(crate) job: String,
    pub(crate) recipe: Option<String>,
    pub(crate) started_at: Instant,
    pub(crate) watch: core_model::Watch,
}

impl Held {
    pub(crate) fn summary(&self) -> WatchSummary {
        WatchSummary {
            id: self.id.clone(),
            session_id: ipc::SessionId::carried(&self.job),
            name: self.watch.name.clone(),
            command: self.watch.command.clone(),
            recipe: self.recipe.clone(),
            every_seconds: self.watch.every_seconds,
            gives_up_after_seconds: self.watch.gives_up_after_seconds,
            follows: self.watch.follows,
            state: self.watch.state.into(),
            live: self.watch.live,
            started_at: self.started_at.clone(),
            // next group: the last reading, with its instant, once readings are taken and kept.
            last: None,
            set_aside: self
                .watch
                .set_aside
                .iter()
                .map(|aside| WatchSetAside { part: aside.part.clone(), sentence: aside.sentence.clone() })
                .collect(),
        }
    }
}

/// Every watch this process holds, live and ended. A `std` lock, never held across an `.await`.
// next group: the store's table, the last 50 readings a watch, so a restart carries on.
#[derive(Default)]
pub(crate) struct Watches {
    held: std::sync::Mutex<Vec<Held>>,
}

impl Watches {
    fn held(&self) -> std::sync::MutexGuard<'_, Vec<Held>> {
        self.held.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    pub(crate) fn live_for(&self, job: &str) -> usize {
        self.held().iter().filter(|one| one.job == job && one.watch.live).count()
    }

    pub(crate) fn keep(&self, held: Held) {
        self.held().push(held);
    }

    /// `job`'s watches, the newest first.
    pub(crate) fn of(&self, job: &str) -> Vec<Held> {
        self.held().iter().rev().filter(|one| one.job == job).cloned().collect()
    }

    /// Change the watch `id` names, where `job` is its Job or no Job is placed.
    pub(crate) fn change<T>(&self, id: &str, job: Option<&str>, act: impl FnOnce(&mut Held) -> T) -> Option<T> {
        let mut held = self.held();
        let one = held.iter_mut().find(|one| one.id == id && job.is_none_or(|job| one.job == job))?;
        Some(act(one))
    }
}
