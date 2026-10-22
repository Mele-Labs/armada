//! Taking a watch's readings: its command run the way a Check's is, in the Job's folder, on a timer
//! of the watches' own.
//!
//! **Its own timer, not the pull request reading's.** That reading is gated to once a minute and
//! waits on the forge, and `timeouts.watchEverySeconds` lets a recipe ask for every 30 seconds:
//! rounding up to the reading's tick would run such a watch half as often as it asked. [`EVERY`] is
//! how late a due watch can start, not how often a command runs.
//!
//! **A run is a Check's run** (`checks_runner::run_words`): no shell, a process group of its own
//! ended whole, `timeouts.watchRunSeconds`, utility priority, and Fleet's environment with the
//! `ARMADA_SPAWNED_BY` mark, so `crate::orphans` ends what a crash left. A recipe's Requests
//! environment adds its variables, secrets included, and every secret's value is taken back out of
//! what is kept of the reading.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use checks_runner::{Attempt, Priority, Stop, Writing};
use config::settings as keys;
use core_model::{Timestamp, WatchState};
use store::{KeptWatchReading, KeptWatchTaken, KeptWatchWhole};
use tokio::task::JoinHandle;
use verification::{Exit, NeverRan};

use super::telling::unsaid;
use super::{domain, moved, BROKEN_IN_A_ROW};
use crate::clock::elapsed;
use crate::daemon::Fleet;

/// How often due watches are looked for.
pub const EVERY: Duration = Duration::from_secs(5);

/// The most of a broken run's output kept with its reading, from its end.
const OUTPUT_KEPT: usize = 4 * 1024;

/// How often a watch is read: as it asked, then no more than every 2 minutes once nothing has
/// changed for 30, and every 5 once nothing has for an hour.
pub(crate) fn interval(every_seconds: u32, unchanged: Duration) -> Duration {
    let every = Duration::from_secs(u64::from(every_seconds));
    let slowest = match unchanged {
        u if u >= Duration::from_secs(60 * 60) => Duration::from_secs(5 * 60),
        u if u >= Duration::from_secs(30 * 60) => Duration::from_secs(2 * 60),
        _ => Duration::ZERO,
    };
    every.max(slowest)
}

/// Whether `kept` is due a reading at `now`: never read, or read an [`interval`] ago.
pub(crate) fn due(kept: &KeptWatchWhole, now: &Timestamp) -> bool {
    let Some(last) = kept.readings.last() else { return true };
    let unchanged = elapsed(&Timestamp::from_rfc3339(kept.watch.changed_at.clone()), now);
    elapsed(&Timestamp::from_rfc3339(last.at.clone()), now) >= interval(kept.watch.every_seconds, unchanged)
}

/// What a reading comes to, to compare with the one before: a slowing watch is one that reads the
/// same.
fn read_as(taken: &KeptWatchTaken) -> String {
    match taken {
        KeptWatchTaken::Read { state, finished, parts, .. } => {
            let parts: Vec<String> = parts.iter().map(|(name, state)| format!("{name}={state}")).collect();
            format!("{state}/{finished}/{}", parts.join(","))
        }
        KeptWatchTaken::Broken { .. } => String::from("broken"),
    }
}

/// Where a watch's command runs.
enum Folder {
    At(PathBuf),
    /// Its folder is not there: a broken reading saying so.
    Gone(String),
    /// Its Job has ended, or Fleet holds no such Job.
    JobEnded,
}

/// Sweep due watches each `every`, and offer every notice still owed.
pub fn keep_running_watches<H, V, W>(fleet: Arc<Fleet<H, V, W>>, every: Duration) -> JoinHandle<()>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(every);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            ticker.tick().await;
            fleet.watches_run_when_due().await;
            fleet.watches_told_when_due().await;
        }
    })
}

fn tail(text: &str, most: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    chars[chars.len().saturating_sub(most)..].iter().collect()
}

/// What a finished run reads as. Only a run that exited 0 is read for its line.
fn reading_of(attempt: &Attempt, budget: Duration) -> KeptWatchTaken {
    let output = || {
        let both = [attempt.output.stderr.trim(), attempt.output.stdout.trim()]
            .into_iter()
            .filter(|one| !one.is_empty())
            .collect::<Vec<_>>()
            .join("\n");
        (!both.is_empty()).then(|| tail(&both, OUTPUT_KEPT))
    };
    let broken = |why: String| KeptWatchTaken::Broken { why, output: output() };
    match &attempt.exit {
        Exit::Code(0) => match ipc::read_watch_line(&attempt.output.stdout) {
            Ok(read) => KeptWatchTaken::Read {
                state: read.state.as_wire().to_string(),
                finished: read.finished,
                says: read.says,
                link: read.link,
                parts: read.parts.into_iter().map(|part| (part.name, part.state.as_wire().to_string())).collect(),
            },
            Err(why) => broken(why.why),
        },
        Exit::Code(code) => broken(format!("It exited {code}.")),
        Exit::Signalled { signal } => broken(format!("It was ended by signal {signal}.")),
        Exit::TimedOut { .. } => broken(format!("It ran past {}s and was stopped.", budget.as_secs())),
        Exit::NeverRan(never) => broken(match never {
            NeverRan::NoSuchCommand { program } => format!("{program} is not a command this machine has."),
            NeverRan::WorktreeGone { worktree } => format!("The Job's folder is gone: {worktree}."),
            NeverRan::NotSpawned { program, kind } => format!("{program} would not start: {kind}."),
            _ => String::from("It never started."),
        }),
    }
}

/// `taken` with every secret's value put back as its `{{name}}`.
fn hidden(taken: KeptWatchTaken, secrets: &[(String, String)]) -> KeptWatchTaken {
    let hide = |text: String| crate::requesting::hidden(&text, secrets);
    match taken {
        KeptWatchTaken::Read { state, finished, says, link, parts } => KeptWatchTaken::Read {
            state,
            finished,
            says: hide(says),
            link: link.map(hide),
            parts: parts.into_iter().map(|(name, state)| (hide(name), state)).collect(),
        },
        KeptWatchTaken::Broken { why, output } => KeptWatchTaken::Broken { why: hide(why), output: output.map(hide) },
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
    /// End each live watch past its give-up time, and start each due one while fewer than
    /// `limits.watchesAtOnce` commands run, the oldest watch first.
    pub(crate) async fn watches_run_when_due(self: &Arc<Self>) {
        let now = self.now();
        let at_once = usize::try_from(self.machine_settings().now().get(keys::WATCHES_AT_ONCE)).unwrap_or(0);
        let Ok(live) = self.store().lock().await.live_watches() else { return };
        for kept in live {
            let id = kept.watch.id.clone();
            if self.watches().is_running(&id) {
                continue;
            }
            let started = Timestamp::from_rfc3339(kept.watch.started_at.clone());
            if elapsed(&started, &now) >= Duration::from_secs(u64::from(kept.watch.gives_up_after_seconds)) {
                self.watch_ended(&id, WatchState::GaveUp).await;
                continue;
            }
            if !due(&kept, &now) {
                continue;
            }
            if self.watches().running_now() >= at_once {
                break;
            }
            let Some(stopped) = self.watches().begin(&id) else { continue };
            let fleet = Arc::clone(self);
            tokio::spawn(async move {
                fleet.watch_run(kept, stopped).await;
                fleet.watches().finished(&id);
            });
        }
    }

    /// Where `job`'s watch runs: the slot its hosted Job works in, else the folder a terminal Job
    /// was started in.
    async fn watch_folder(&self, job: &str) -> Folder {
        let (session, hosting) = {
            let store = self.store().lock().await;
            let Ok(Some(session)) = store.session(job) else { return Folder::JobEnded };
            (session, store.hosting(job).ok().flatten())
        };
        if session.state == store::SessionState::Ended {
            return Folder::JobEnded;
        }
        let path = match hosting {
            Some(hosting) => match self.served_named(Some(&ipc::ManifestId::carried(&hosting.manifest_id))) {
                Ok(served) => Self::directory_of(&served, &hosting),
                Err(_) => return Folder::Gone(String::from("Fleet no longer serves the Job's repository.")),
            },
            None => session.cwd,
        };
        match std::path::Path::new(&path).is_dir() {
            true => Folder::At(PathBuf::from(path)),
            false => Folder::Gone(format!("The Job's folder is gone: {path}.")),
        }
    }

    /// Run `kept`'s command once and keep what it read, unless `stopped` ends it first.
    async fn watch_run(self: &Arc<Self>, kept: KeptWatchWhole, stopped: tokio::sync::oneshot::Receiver<()>) {
        let id = kept.watch.id.clone();
        let taken = match self.watch_folder(&kept.watch.session_id).await {
            Folder::JobEnded => return self.watches_ended_with(&kept.watch.session_id).await,
            Folder::Gone(why) => KeptWatchTaken::Broken { why, output: None },
            Folder::At(folder) => {
                let mut env = Vec::new();
                if let Some(mark) = crate::orphans::own_mark() {
                    env.push((crate::orphans::MARK.to_string(), mark));
                }
                let given = match &kept.watch.environment {
                    Some(named) => self.watch_values(&kept.watch.session_id, named).await,
                    None => Ok(None),
                };
                match given {
                    Err(why) => KeptWatchTaken::Broken { why, output: None },
                    Ok(given) => {
                        let secrets = given.as_ref().map(|values| values.secrets().to_vec()).unwrap_or_default();
                        env.extend(given.iter().flat_map(|values| values.variables()));
                        let budget = self.machine_settings().now().get(keys::WATCH_RUN_SECONDS);
                        let Some((program, args)) = kept.watch.command.split_first() else { return };
                        let stop = async move {
                            match stopped.await {
                                Ok(()) => Stop::Graceful,
                                Err(_) => std::future::pending().await,
                            }
                        };
                        let attempt = checks_runner::run_words(
                            program.clone(),
                            args.to_vec(),
                            &folder,
                            budget,
                            Writing::Nowhere,
                            &env,
                            stop,
                            Priority::Low,
                            checks_runner::GRACE,
                        )
                        .await;
                        hidden(reading_of(&attempt, budget), &secrets)
                    }
                }
            }
        };
        self.watch_read(&id, taken).await;
    }

    /// Keep one reading of `id`, and what it comes to: its state, the end it reaches, and the
    /// notice it owes. **An ended watch keeps none**, so a run that finishes after a Stop changes
    /// nothing.
    pub(crate) async fn watch_read(self: &Arc<Self>, id: &str, taken: KeptWatchTaken) {
        let now = self.now();
        let job = {
            let mut store = self.store().lock().await;
            let Ok(Some(kept)) = store.watch_whole(id) else { return };
            if !kept.watch.live {
                return;
            }
            let mut watch = domain(&kept);
            if let KeptWatchTaken::Read { .. } = &taken {
                if let Some(reading) = domain(&KeptWatchWhole {
                    readings: vec![KeptWatchReading { at: now.as_str().to_string(), taken: taken.clone() }],
                    ..kept.clone()
                })
                .last
                {
                    watch.read(reading);
                }
            }
            let broken_run = 1 + kept
                .readings
                .iter()
                .rev()
                .take_while(|one| matches!(one.taken, KeptWatchTaken::Broken { .. }))
                .count();
            if matches!(taken, KeptWatchTaken::Broken { .. }) && broken_run >= BROKEN_IN_A_ROW {
                watch.end(WatchState::Broken);
            }
            let reads_new = kept.readings.last().is_none_or(|last| read_as(&last.taken) != read_as(&taken));
            let mut next = moved(&kept.watch, &watch, &now);
            if reads_new {
                next.changed_at = now.as_str().to_string();
            }
            next.unsaid = unsaid(&store, id, &watch);
            let reading = KeptWatchReading { at: now.as_str().to_string(), taken };
            if store.keep_watch_reading(id, &reading).is_err() || store.keep_watch(&next).is_err() {
                return;
            }
            let moved = reads_new || next.live != kept.watch.live;
            (next.session_id, moved, next.live != kept.watch.live)
        };
        let (job, moved, ended) = job;
        // Recorded on a change too, so the five minutes run from the last thing Bridge was sent.
        if self.watches().may_republish(id, &now) | moved {
            self.watches_published(&job).await;
        }
        if ended {
            self.watching_row_published(&job).await;
        }
        self.watch_told(id).await;
    }

    /// End `id` `how`, where it is live, and tell what it owes.
    pub(crate) async fn watch_ended(self: &Arc<Self>, id: &str, how: WatchState) {
        let now = self.now();
        let job = {
            let mut store = self.store().lock().await;
            let Ok(Some(kept)) = store.watch_whole(id) else { return };
            if !kept.watch.live {
                return;
            }
            let mut watch = domain(&kept);
            watch.end(how);
            let mut next = moved(&kept.watch, &watch, &now);
            next.unsaid = unsaid(&store, id, &watch);
            if store.keep_watch(&next).is_err() {
                return;
            }
            next.session_id
        };
        self.watches().stop(id);
        self.watches_published(&job).await;
        self.watching_row_published(&job).await;
        self.watch_told(id).await;
    }
}
