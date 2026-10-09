//! The build Fleet runs on, read off the files `scripts/restart` leaves, and the
//! detached restart onto another.
//!
//! **The files are the record, because the Fleet that starts a restart is not the
//! Fleet that reads it back.** `Armada/restart-source` says which tree the
//! running build came from, `Armada/restart-commit` which commit, and
//! `Armada/restart-build.status` what `scripts/restart-build` is doing or why it
//! stopped. Nothing here is held in memory but the time of the last fetch.
//!
//! **The restart outlives Fleet.** `scripts/restart-build` is started with
//! [`Detached`], in a session of its own, and unmarked so the next Fleet's
//! orphan sweep does not end it: it stops this Fleet, installs the next and
//! reopens Bridge, all after the process that asked for it is gone.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant, SystemTime};

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{
    BuildPosition, BuildSource, BuildStage, ChangeFleetBuild, FleetBuildChanging, FleetBuildReport, WireError,
};

use crate::daemon::Fleet;
use crate::Detached;

/// No served repository can be restarted from, or the running build came from a tree this does not offer. A 409.
const NOT_OFFERED: &str = "fleet.build_not_offered";
/// A restart is already under way. A 409.
const RESTARTING: &str = "fleet.build_restarting";
/// `scripts/restart-build` would not start. A 500.
const NOT_STARTED: &str = "fleet.build_not_started";

/// The script that wraps `scripts/restart` and `scripts/preview --restart`.
const WRAPPER: &str = "scripts/restart-build";
/// Which tree the running build came from. Written by `scripts/restart --from`
/// and removed by a plain restart.
const SOURCE: &str = "restart-source";
/// The commit `scripts/restart` last installed from.
const COMMIT: &str = "restart-commit";
/// What the wrapper is doing, or why it stopped.
const STATUS: &str = "restart-build.status";
/// The preview tree's tail, as `scripts/restart` records it.
const PREVIEW_TREE: &str = ".armada/preview";

/// How often `origin main` is fetched. A read never waits for one.
const FETCH_EVERY: Duration = Duration::from_secs(10 * 60);
/// A fetch still running after this is stopped.
const FETCH_WITHIN: Duration = Duration::from_secs(60);
/// A `running` status older than this is a wrapper that died, not one working.
const RUNNING_FOR_AT_MOST: Duration = Duration::from_secs(20 * 60);

/// Why a Fleet has no build to offer.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum NotOffered {
    /// No served repository holds `scripts/restart-build`.
    NoWrapper,
    /// `restart-source` names a tree other than the preview.
    OtherTree(String),
}

/// Why a restart was not started.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum NotChanged {
    NotOffered(NotOffered),
    /// One is under way.
    Restarting(BuildSource),
    /// The wrapper would not start.
    NotStarted(String),
}

/// What the status file says.
#[derive(Debug, PartialEq, Eq)]
enum Status {
    Running(BuildSource, Option<BuildStage>),
    /// Stopped, having failed with the build at `commit`.
    Failed { commit: String, message: String },
}

#[derive(Default)]
struct Fetched {
    at: Option<Instant>,
}

/// What Fleet holds of its build: when it last fetched, and nothing else.
#[derive(Clone, Default)]
pub(crate) struct Building {
    fetched: Arc<Mutex<Fetched>>,
}

/// The first served root that holds the wrapper: the checkout a restart runs from.
pub(crate) fn checkout_of<'a>(roots: impl IntoIterator<Item = &'a str>) -> Option<PathBuf> {
    roots
        .into_iter()
        .map(PathBuf::from)
        .find(|root| root.join(WRAPPER).is_file())
}

fn source_in(support: &Path) -> Result<BuildSource, NotOffered> {
    let recorded = std::fs::read_to_string(support.join(SOURCE)).unwrap_or_default();
    let tree = recorded.trim();
    if tree.is_empty() {
        Ok(BuildSource::Main)
    } else if tree.trim_end_matches('/').ends_with(PREVIEW_TREE) {
        Ok(BuildSource::Preview)
    } else {
        Err(NotOffered::OtherTree(tree.to_string()))
    }
}

/// The recorded commit, if it is a full object name.
fn commit_in(support: &Path) -> Option<String> {
    let recorded = std::fs::read_to_string(support.join(COMMIT)).ok()?;
    let commit = recorded.trim();
    (commit.len() == 40 && commit.bytes().all(|b| b.is_ascii_hexdigit())).then(|| commit.to_string())
}

fn build_named(word: &str) -> Option<BuildSource> {
    match word.trim() {
        "main" => Some(BuildSource::Main),
        "preview" => Some(BuildSource::Preview),
        _ => None,
    }
}

fn stage_named(word: &str) -> Option<BuildStage> {
    match word.trim() {
        "merging" => Some(BuildStage::Merging),
        "fetching_main" => Some(BuildStage::FetchingMain),
        "building_fleet" => Some(BuildStage::BuildingFleet),
        "building_bridge" => Some(BuildStage::BuildingBridge),
        "restarting_fleet" => Some(BuildStage::RestartingFleet),
        "reopening_bridge" => Some(BuildStage::ReopeningBridge),
        "retrying" => Some(BuildStage::Retrying),
        _ => None,
    }
}

fn status_in(support: &Path, now: SystemTime) -> Option<Status> {
    let path = support.join(STATUS);
    let text = std::fs::read_to_string(&path).ok()?;
    let mut lines = text.lines();
    match lines.next()?.trim() {
        "running" => {
            let build = build_named(lines.next()?)?;
            let written = std::fs::metadata(&path).ok()?.modified().ok()?;
            let age = now.duration_since(written).unwrap_or_default();
            // A third line is the stage; a file without one, or with one this does not know, has none.
            let stage = lines.next().and_then(stage_named);
            (age <= RUNNING_FOR_AT_MOST).then_some(Status::Running(build, stage))
        }
        "failed" => {
            let _build = lines.next()?;
            let commit = lines.next()?.trim().to_string();
            let message = lines.next()?.trim().to_string();
            (!message.is_empty()).then_some(Status::Failed { commit, message })
        }
        _ => None,
    }
}

fn build_word(build: BuildSource) -> &'static str {
    match build {
        BuildSource::Main => "main",
        BuildSource::Preview => "preview",
    }
}

impl Fetched {
    /// Whether `origin main` is due, and if it is, now counted as started.
    fn take_if_due(&mut self, now: Instant) -> bool {
        let due = self.at.is_none_or(|last| now.duration_since(last) >= FETCH_EVERY);
        if due {
            self.at = Some(now);
        }
        due
    }
}

impl Building {
    /// The build as the files and the repository say it is. **Never waits on the
    /// network**: a fetch that is due runs in the background, and this answers
    /// with the refs as they stand.
    pub(crate) async fn report(
        &self,
        checkout: Option<&Path>,
        support: &Path,
        now: SystemTime,
    ) -> Result<FleetBuildReport, NotOffered> {
        let Some(checkout) = checkout else {
            return Err(NotOffered::NoWrapper);
        };
        let on = source_in(support)?;
        let commit = commit_in(support);
        let root = checkout.to_string_lossy().into_owned();
        if self.fetched.lock().unwrap_or_else(PoisonError::into_inner).take_if_due(Instant::now()) {
            let root = root.clone();
            drop(tokio::task::spawn_blocking(move || {
                // A fetch that fails leaves the counts as they were.
                let _ = adapters::fetch_main(&root, FETCH_WITHIN);
            }));
        }
        let position = match commit.clone() {
            None => None,
            Some(commit) => tokio::task::spawn_blocking(move || adapters::position_against_main(&root, &commit))
                .await
                .ok()
                .flatten()
                .map(|(ahead, behind)| BuildPosition { ahead, behind }),
        };
        let (restarting, stage, failed) = match status_in(support, now) {
            Some(Status::Running(build, stage)) => (Some(build), stage, None),
            // A failure belongs to the build it failed on; once that has moved, it is history.
            Some(Status::Failed { commit: at, message })
                if (!at.is_empty()).then_some(at.as_str()) == commit.as_deref() =>
            {
                (None, None, Some(message))
            }
            _ => (None, None, None),
        };
        Ok(FleetBuildReport {
            on,
            commit,
            position,
            restarting,
            stage,
            failed,
        })
    }

    /// Start the wrapper detached and answer. **The status is written first**,
    /// so a read between the answer and the wrapper's first line already says a
    /// restart is under way.
    pub(crate) fn change(
        &self,
        checkout: Option<&Path>,
        support: &Path,
        asked: &ChangeFleetBuild,
        now: SystemTime,
    ) -> Result<FleetBuildChanging, NotChanged> {
        let Some(checkout) = checkout else {
            return Err(NotChanged::NotOffered(NotOffered::NoWrapper));
        };
        source_in(support).map_err(NotChanged::NotOffered)?;
        if let Some(Status::Running(build, _)) = status_in(support, now) {
            return Err(NotChanged::Restarting(build));
        }
        let status = support.join(STATUS);
        std::fs::create_dir_all(support).map_err(|why| NotChanged::NotStarted(why.to_string()))?;
        std::fs::write(&status, format!("running\n{}\n", build_word(asked.build)))
            .map_err(|why| NotChanged::NotStarted(why.to_string()))?;
        let mut wrapper = Detached::program(checkout.join(WRAPPER))
            .arg(build_word(asked.build))
            .in_directory(checkout)
            .outliving_fleet()
            .ignoring_output();
        if asked.adopt {
            wrapper = wrapper.arg("--adopt");
        }
        if let Err(why) = wrapper.spawn() {
            let _ = std::fs::remove_file(&status);
            return Err(NotChanged::NotStarted(why.to_string()));
        }
        Ok(FleetBuildChanging { build: asked.build })
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
    fn build_checkout(&self) -> Option<PathBuf> {
        let roots = self.ledger_roots();
        checkout_of(roots.iter().map(|(_, root)| root.as_str()))
    }

    /// `get_fleet_build`.
    pub(crate) async fn fleet_build(&self) -> Result<FleetBuildReport, Refusal> {
        let support = crate::runtime::support_dir(&self.host().home);
        self.building()
            .report(self.build_checkout().as_deref(), &support, SystemTime::now())
            .await
            .map_err(|why| self.build_refusal(why))
    }

    /// `change_fleet_build`.
    pub(crate) fn change_fleet_build_now(
        &self,
        asked: &ChangeFleetBuild,
    ) -> Result<FleetBuildChanging, Refusal> {
        let support = crate::runtime::support_dir(&self.host().home);
        self.building()
            .change(self.build_checkout().as_deref(), &support, asked, SystemTime::now())
            .map_err(|why| match why {
                NotChanged::NotOffered(why) => self.build_refusal(why),
                NotChanged::Restarting(build) => Refusal::IllegalMove(WireError::raised(
                    RESTARTING,
                    format!("restarting onto {}", build_word(build)),
                    self.run_id(),
                )),
                NotChanged::NotStarted(said) => {
                    Refusal::Fault(WireError::raised(NOT_STARTED, said, self.run_id()))
                }
            })
    }

    fn build_refusal(&self, why: NotOffered) -> Refusal {
        let said = match why {
            NotOffered::NoWrapper => String::from("no served repository holds scripts/restart-build"),
            NotOffered::OtherTree(tree) => format!("built from {tree}"),
        };
        Refusal::IllegalMove(WireError::raised(NOT_OFFERED, said, self.run_id()))
    }
}
