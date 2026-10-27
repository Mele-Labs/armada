//! Watches: a command Fleet runs on a cadence for one Job, until what it follows is finished.
//! `docs/design/fleet-watches-ci-and-deploys-for-a-job.md`.
//!
//! **The store holds every watch**, its readings, set-asides, verdicts and what was told of it, so
//! a restart carries on (`store::KeptWatch`). Memory holds only the commands running now.
//! `serving` answers the four operations, `running` takes readings on a timer of its own, and
//! `telling` tells the Job once per change and ends its watches with it, and `needing` puts one that
//! ended not ours on its Needs you list.

pub(crate) mod environment;
pub(crate) mod listening;
mod logs;
mod needing;
mod running;
mod serving;
mod telling;
mod tools;

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

pub(crate) use environment::LoginEnvironment;
pub use running::{keep_running_watches, EVERY};
#[cfg(test)]
pub(crate) use running::interval;
pub(crate) use needing::{fix_it, not_ours_item, FIX_IT, IGNORE};
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
/// `delete_watch` on a watch still live. A 409.
pub(crate) const LIVE: &str = "fleet.watch_live";
/// `ours` on a watch that has not failed. A 409.
pub(crate) const NOT_FAILED: &str = "fleet.watch_not_failed";
/// `not_ours` naming a part no reading of the watch carried. A 422.
pub(crate) const PART_UNREPORTED: &str = "fleet.watch_part_unreported";
/// A terminal watch started where the Job has no terminal open. A 409.
pub(crate) const TERMINAL_CLOSED: &str = "fleet.watch_terminal_closed";
/// `save_watch_recipe` naming a recipe Armada ships. A 409.
pub(crate) const RECIPE_SHIPPED: &str = "fleet.watch_recipe_shipped";
/// `save_watch_recipe` naming a recipe there already is, without `replace`. A 409.
pub(crate) const RECIPE_EXISTS: &str = "fleet.watch_recipe_exists";
/// `save_watch_recipe` could not write the recipe or its script. A 500.
pub(crate) const RECIPE_UNWRITTEN: &str = "fleet.watch_recipe_unwritten";

/// The most a watch may run before giving up: the ceiling of `timeouts.watchGiveUpMinutes`'
/// entry, a day, so lowering the default never lowers the cap.
pub(crate) const GIVE_UP_CAP_SECONDS: u32 = 24 * 60 * 60;

/// Broken readings in a row that end a watch `broken`.
pub(crate) const BROKEN_IN_A_ROW: usize = 3;

/// What runs for one watch now: its command, or a terminal watch's listening, and what stops it.
struct Held {
    stop: tokio::sync::oneshot::Sender<()>,
    /// A command, which counts toward `limits.watchesAtOnce`. A terminal watch runs nothing.
    command: bool,
}

/// The commands running now, each with what stops it, and the environment each live watch was
/// given. **Memory only**: a run a restart cut short left no reading, so its watch is simply due
/// again, `crate::orphans` ends its process, and its environment is read again at that run.
pub(crate) struct Watches {
    running: Mutex<HashMap<String, Held>>,
    /// The environment each watch started with, by watch.
    environments: Mutex<HashMap<String, LoginEnvironment>>,
    /// The login shell read for an environment. `None` reads none, as Fleet's own tests do.
    shell: Mutex<Option<String>>,
    /// When a notice a Job could not take was last offered, so it is offered a minute apart.
    offered: Mutex<HashMap<String, core_model::Timestamp>>,
    /// When a reading that read the same was last published, by watch.
    republished: Mutex<HashMap<String, core_model::Timestamp>>,
}

impl Default for Watches {
    fn default() -> Watches {
        Watches {
            running: Mutex::default(),
            environments: Mutex::default(),
            shell: Mutex::new((!cfg!(test)).then(crate::session_running::shell)),
            offered: Mutex::default(),
            republished: Mutex::default(),
        }
    }
}

impl Watches {
    fn running(&self) -> MutexGuard<'_, HashMap<String, Held>> {
        self.running.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The commands running now. A terminal watch's listening is not one.
    pub(crate) fn running_now(&self) -> usize {
        self.running().values().filter(|held| held.command).count()
    }

    /// The login shell an environment is read from, where one is.
    pub(crate) fn shell(&self) -> Option<String> {
        self.shell.lock().unwrap_or_else(PoisonError::into_inner).clone()
    }

    #[cfg(test)]
    pub(crate) fn read_login_shell(&self, shell: Option<String>) {
        *self.shell.lock().unwrap_or_else(PoisonError::into_inner) = shell;
    }

    pub(crate) fn environment_of(&self, id: &str) -> Option<LoginEnvironment> {
        self.environments.lock().unwrap_or_else(PoisonError::into_inner).get(id).cloned()
    }

    pub(crate) fn keep_environment(&self, id: &str, environment: LoginEnvironment) {
        self.environments.lock().unwrap_or_else(PoisonError::into_inner).insert(id.to_string(), environment);
    }

    pub(crate) fn is_running(&self, id: &str) -> bool {
        self.running().contains_key(id)
    }

    /// Hold `id`'s place among the commands running; what completes when it is stopped. `None`
    /// where it already runs.
    pub(crate) fn begin(&self, id: &str) -> Option<tokio::sync::oneshot::Receiver<()>> {
        self.held(id, true)
    }

    /// [`Watches::begin`] for a terminal watch's listening, which runs no command.
    pub(crate) fn begin_listening(&self, id: &str) -> Option<tokio::sync::oneshot::Receiver<()>> {
        self.held(id, false)
    }

    fn held(&self, id: &str, command: bool) -> Option<tokio::sync::oneshot::Receiver<()>> {
        let mut running = self.running();
        if running.contains_key(id) {
            return None;
        }
        let (stop, stopped) = tokio::sync::oneshot::channel();
        running.insert(id.to_string(), Held { stop, command });
        Some(stopped)
    }

    pub(crate) fn finished(&self, id: &str) {
        self.running().remove(id);
    }

    /// End `id`'s command, if one runs, and forget its environment: only an ended watch is stopped.
    pub(crate) fn stop(&self, id: &str) {
        self.environments.lock().unwrap_or_else(PoisonError::into_inner).remove(id);
        if let Some(held) = self.running().remove(id) {
            let _ = held.stop.send(());
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
                .map(|part| core_model::Part {
                    name: part.name.clone(),
                    state: reading_state(&part.state),
                    link: part.link.clone(),
                    error: part.error.clone(),
                    tail: part.tail.clone(),
                })
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
        KeptWatchTaken::Read { state, finished, says, link, parts } => WatchReading::read(
            at,
            wire_reading_state(state),
            *finished,
            says.clone(),
            link.clone(),
            parts
                .iter()
                .map(|part| WatchPart {
                    name: part.name.clone(),
                    state: wire_reading_state(&part.state),
                    link: part.link.clone(),
                    error: part.error.clone(),
                    tail: part.tail.clone(),
                })
                .collect(),
        ),
        KeptWatchTaken::Broken { why, output } => WatchReading::broken(at, why.clone(), output.clone()),
    }
    .logged(one.log.clone())
}

/// What `list_watches` and `watches.changed` say of one watch, and where its environment came from
/// where Fleet holds that.
pub(crate) fn summary(kept: &KeptWatchWhole, login: Option<&LoginEnvironment>) -> WatchSummary {
    let held = &kept.watch;
    let last = kept.readings.last().map(wire_reading);
    let readings = ipc::collapsed(kept.readings.iter().map(wire_reading));
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
        last,
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
        in_terminal: held.in_terminal.clone(),
        regex: held.regex,
        login_shell: login.filter(|_| held.live).map(LoginEnvironment::wire),
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

    /// [`summary`], with the environment Fleet holds for it.
    pub(crate) fn watch_summary(&self, kept: &KeptWatchWhole) -> WatchSummary {
        summary(kept, self.watches().environment_of(&kept.watch.id).as_ref())
    }

    /// `job`'s watches, the newest first.
    pub(crate) async fn watch_list(&self, job: &str) -> Result<WatchList, Refusal> {
        let kept = self.store().lock().await.watches_of(job).map_err(|why| self.ledger_fault(why))?;
        let watches = kept.iter().map(|one| self.watch_summary(one)).collect();
        Ok(WatchList { session_id: ipc::SessionId::carried(job), watches })
    }

    /// Publish `job`'s row again, as a watch starting or ending changes whether it is held open by one.
    pub(crate) async fn watching_row_published(&self, job: &str) {
        let record = {
            let store = self.store().lock().await;
            let Ok(Some(kept)) = store.session(job) else { return };
            self.ledger_row(&store, &kept).ok()
        };
        if let Some(record) = record {
            self.publish(ipc::Event::SessionChanged(record));
        }
    }

    /// Tell Bridge `job`'s watches moved. A list that will not read publishes nothing.
    pub(crate) async fn watches_published(&self, job: &str) {
        if let Ok(list) = self.watch_list(job).await {
            self.publish(ipc::Event::WatchesChanged(list));
        }
    }
}
