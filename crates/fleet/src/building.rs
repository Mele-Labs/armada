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
//!
//! **Fleet never restarts the preview by itself.** A branch `scripts/preview` left out that has
//! moved past the tip it was left out at is [`left_out_moved`], which Fleet offers to the Job holding
//! it (`session_host::preview_offers`); only the owner's press starts the restart.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant, SystemTime};

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{
    BuildPosition, BuildSource, BuildStage, ChangeFleetBuild, FleetBuildChanging, FleetBuildReport, FleetBuildReverting,
    Instant as WireInstant, KeptBuildPrevious, KeptBuildRunning, LeftOutBranch, WireError,
};

use crate::daemon::Fleet;
use crate::kept_builds::{self, RevertStatus};
use crate::Detached;

/// No served repository can be restarted from, or the running build came from a tree this does not offer. A 409.
const NOT_OFFERED: &str = "fleet.build_not_offered";
/// A restart is already under way. A 409.
const RESTARTING: &str = "fleet.build_restarting";
/// `scripts/restart-build` would not start. A 500.
const NOT_STARTED: &str = "fleet.build_not_started";
/// There is no kept build to revert onto, or Fleet does not run from the kept builds. A 409.
const NOT_REVERTIBLE: &str = "fleet.build_not_revertible";
/// A revert is already under way. A 409.
const REVERTING: &str = "fleet.build_reverting";
/// `builds/previous/armada build revert` would not start. A 500.
const REVERT_NOT_STARTED: &str = "fleet.build_revert_not_started";

/// The script that wraps `scripts/restart` and `scripts/preview --restart`.
const WRAPPER: &str = "scripts/restart-build";
/// Which tree the running build came from. Written by `scripts/restart --from`
/// and removed by a plain restart.
const SOURCE: &str = "restart-source";
/// The commit `scripts/restart` last installed from.
const COMMIT: &str = "restart-commit";
/// The branches `scripts/preview` left out of the preview, each with why.
const LEFT_OUT: &str = "preview-left-out";
/// The main commit a preview stands on and the branches merged into it, as `scripts/preview` records them.
const HELD: &str = "preview-held";
/// What the wrapper is doing, or why it stopped.
const STATUS: &str = "restart-build.status";
/// The preview tree's tail, as `scripts/restart` records it.
const PREVIEW_TREE: &str = ".armada/preview";

/// How often `origin main` is fetched. A read never waits for one.
const FETCH_EVERY: Duration = Duration::from_secs(10 * 60);
/// A fetch still running after this is stopped.
const FETCH_WITHIN: Duration = Duration::from_secs(60);
/// How `scripts/preview` words a branch it left out for a migration that is not on main.
const BREAKING_MIGRATION: &str = "breaking migration";
/// A `running` status older than this is a wrapper that died, not one working.
const RUNNING_FOR_AT_MOST: Duration = Duration::from_secs(20 * 60);

/// Why a Fleet has no build to offer.
#[derive(Debug, PartialEq, Eq)]
pub enum NotOffered {
    /// No served repository holds `scripts/restart-build`.
    NoWrapper,
    /// `restart-source` names a tree other than the preview.
    OtherTree(String),
}

/// Why a restart was not started.
#[derive(Debug, PartialEq, Eq)]
pub enum NotChanged {
    NotOffered(NotOffered),
    /// One is under way.
    Restarting(BuildSource),
    /// The wrapper would not start.
    NotStarted(String),
}

/// Why a revert was not started.
#[derive(Debug, PartialEq, Eq)]
pub enum NotReverted {
    /// No previous build is kept, or launchd's Fleet is not on `builds/current`: the reason in a line.
    NotRevertible(String),
    /// A restart is under way.
    Restarting(BuildSource),
    /// A revert is under way.
    Reverting,
    /// The undo would not start.
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

/// What Fleet holds of its build: when it last fetched.
#[derive(Clone, Default)]
pub struct Building {
    fetched: Arc<Mutex<Fetched>>,
}

/// The first served root that holds the wrapper: the checkout a restart runs from.
pub fn checkout_of<'a>(roots: impl IntoIterator<Item = &'a str>) -> Option<PathBuf> {
    roots
        .into_iter()
        .map(PathBuf::from)
        .find(|root| root.join(WRAPPER).is_file())
}

pub fn source_in(support: &Path) -> Result<BuildSource, NotOffered> {
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
pub fn commit_in(support: &Path) -> Option<String> {
    let recorded = std::fs::read_to_string(support.join(COMMIT)).ok()?;
    let commit = recorded.trim();
    (commit.len() == 40 && commit.bytes().all(|b| b.is_ascii_hexdigit())).then(|| commit.to_string())
}

/// A branch `scripts/preview` left out of the build: where its tip stood then, and why.
#[derive(Debug, PartialEq, Eq)]
pub struct LeftOut {
    pub tip: String,
    pub why: String,
}

/// The branches `scripts/preview` left out of the build at `commit`: its first line, then
/// `branch<TAB>tip<TAB>why` lines. **Empty** where the file is missing or was written for another build.
pub fn left_out_in(support: &Path, commit: &str) -> BTreeMap<String, LeftOut> {
    let Ok(recorded) = std::fs::read_to_string(support.join(LEFT_OUT)) else {
        return BTreeMap::new();
    };
    let mut lines = recorded.lines();
    if lines.next().map(str::trim) != Some(commit) {
        return BTreeMap::new();
    }
    lines
        .filter_map(|line| {
            let mut fields = line.splitn(3, '\t').map(str::trim);
            let (branch, tip, why) = (fields.next()?, fields.next()?, fields.next()?);
            (!branch.is_empty() && !tip.is_empty() && !why.is_empty())
                .then(|| (branch.to_string(), LeftOut { tip: tip.to_string(), why: why.to_string() }))
        })
        .collect()
}

/// What a preview holds: the main commit under it and the branches merged into it.
#[derive(Debug, PartialEq, Eq)]
pub struct Held {
    pub main_commit: String,
    pub branches: Vec<String>,
}

/// What `scripts/preview` recorded the build at `commit` holds: its first line, then the main commit,
/// then `branch<TAB>tip` lines. **`None`** where the file is missing or was written for another build.
pub fn held_in(support: &Path, commit: &str) -> Option<Held> {
    let recorded = std::fs::read_to_string(support.join(HELD)).ok()?;
    let mut lines = recorded.lines();
    if lines.next().map(str::trim) != Some(commit) {
        return None;
    }
    let main_commit = lines.next()?.trim().to_string();
    let branches = lines.filter_map(|line| Some(line.split('\t').next()?.trim().to_string())).filter(|branch| !branch.is_empty()).collect();
    (!main_commit.is_empty()).then_some(Held { main_commit, branches })
}

/// When the restart wrote `restart-commit`, in milliseconds since the epoch: when the build began.
pub fn commit_written_in(support: &Path) -> Option<i64> {
    let written = std::fs::metadata(support.join(COMMIT)).ok()?.modified().ok()?;
    i64::try_from(written.duration_since(SystemTime::UNIX_EPOCH).ok()?.as_millis()).ok()
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

/// Whether `scripts/restart-build` is working now, so the commit on file is not yet the build.
pub fn restarting_in(support: &Path, now: SystemTime) -> bool {
    matches!(status_in(support, now), Some(Status::Running(..)))
}

pub fn build_word(build: BuildSource) -> &'static str {
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

/// A branch the running preview left out whose tip has moved since: worth a refresh, if the owner says.
#[derive(Debug, PartialEq, Eq)]
pub struct Moved {
    pub branch: String,
    /// Where `tip_of` reads it now.
    pub tip: String,
    pub why: String,
    /// The commit of the build that left it out.
    pub build: String,
}

/// The branches the running preview left out whose tip, as `tip_of` reads it, is not the one they were
/// left out at. **Only a reading**: none while a restart is under way, and none left out for a breaking
/// migration, which only main can bring back.
pub fn left_out_moved(support: &Path, now: SystemTime, tip_of: impl Fn(&str) -> Option<String>) -> Vec<Moved> {
    if !matches!(source_in(support), Ok(BuildSource::Preview)) || restarting_in(support, now) {
        return Vec::new();
    }
    let Some(build) = commit_in(support) else {
        return Vec::new();
    };
    left_out_in(support, &build)
        .into_iter()
        .filter(|(_, left)| !left.why.starts_with(BREAKING_MIGRATION))
        .filter_map(|(branch, left)| {
            let tip = tip_of(&branch).filter(|tip| *tip != left.tip)?;
            Some(Moved { branch, tip, why: left.why, build: build.clone() })
        })
        .collect()
}

impl Building {
    /// The build as the files and the repository say it is. **Never waits on the
    /// network**: a fetch that is due runs in the background, and this answers
    /// with the refs as they stand.
    pub async fn report(
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
            Some(commit) => {
                let root = root.clone();
                tokio::task::spawn_blocking(move || adapters::position_against_main(&root, &commit))
                    .await
                    .ok()
                    .flatten()
                    .map(|(ahead, behind)| BuildPosition { ahead, behind })
            }
        };
        let (restarting, stage, mut failed) = match status_in(support, now) {
            Some(Status::Running(build, stage)) => (Some(build), stage, None),
            // A failure belongs to the build it failed on; once that has moved, it is history.
            Some(Status::Failed { commit: at, message })
                if (!at.is_empty()).then_some(at.as_str()) == commit.as_deref() =>
            {
                (None, None, Some(message))
            }
            _ => (None, None, None),
        };
        let left_out = match (on, commit.as_deref()) {
            (BuildSource::Preview, Some(commit)) => {
                let left = left_out_in(support, commit);
                let branches: Vec<String> = left.keys().cloned().collect();
                // Local refs only, so the report never waits on a fetch. A branch git cannot find has not moved.
                let tips: Vec<Option<String>> = tokio::task::spawn_blocking(move || {
                    branches.iter().map(|branch| adapters::branch_tip(&root, branch)).collect()
                })
                .await
                .unwrap_or_default();
                left.into_iter()
                    .zip(tips.into_iter().chain(std::iter::repeat(None)))
                    .map(|((branch, left), tip)| {
                        let moved = tip.is_some_and(|tip| tip != left.tip);
                        LeftOutBranch { branch, why: left.why, moved }
                    })
                    .collect()
            }
            _ => Vec::new(),
        };
        let revert = kept_builds::revert_status_in(support, now);
        // A restart's failure is the one the panel already follows; a revert's shows where it has none.
        if failed.is_none() {
            if let Some(RevertStatus::Failed(line)) = &revert {
                failed = Some(line.clone());
            }
        }
        let label = kept_builds::fleet_label();
        let running = kept_builds::kept_at(&support.join(kept_builds::BUILDS), kept_builds::CURRENT).map(|kept| KeptBuildRunning {
            built_at: WireInstant::carried(kept.built_at),
            commit: kept.commit,
        });
        // **Branches here**; `Fleet::fleet_build` names them by the Job that holds each.
        let previous = kept_builds::revertible(support, &label).ok().map(|(from, to)| {
            let undo = kept_builds::undoing(&from, &to);
            KeptBuildPrevious {
                built_at: WireInstant::carried(undo.built_at),
                commit: undo.commit,
                bridge_reopens: undo.bridge_reopens,
                takes_out: undo.takes_out,
                brings_back: undo.brings_back,
            }
        });
        Ok(FleetBuildReport {
            on,
            commit,
            position,
            restarting,
            stage,
            failed,
            left_out,
            running,
            previous,
            carried: Vec::new(),
            reverting: matches!(revert, Some(RevertStatus::Running)),
        })
    }

    /// Start the undo, `builds/previous/armada build revert`, detached and answer. **Run by the build
    /// reverted to**, so a broken build never has to run its own undo; **the status is written first**,
    /// as `change` writes its own.
    pub fn revert(&self, support: &Path, label: &str, now: SystemTime) -> Result<FleetBuildReverting, NotReverted> {
        if let Some(Status::Running(build, _)) = status_in(support, now) {
            return Err(NotReverted::Restarting(build));
        }
        if kept_builds::revert_status_in(support, now) == Some(RevertStatus::Running) {
            return Err(NotReverted::Reverting);
        }
        let (_, to) = kept_builds::revertible(support, label).map_err(NotReverted::NotRevertible)?;
        kept_builds::revert_begun(support).map_err(|why| NotReverted::NotStarted(why.to_string()))?;
        let undo = support.join(kept_builds::BUILDS).join(kept_builds::PREVIOUS).join("armada");
        let started = Detached::program(undo)
            .arg("build")
            .arg("revert")
            .in_directory(support)
            .outliving_fleet()
            .ignoring_output();
        if let Err(why) = started.spawn() {
            let _ = std::fs::remove_file(support.join(kept_builds::STATUS));
            return Err(NotReverted::NotStarted(why.to_string()));
        }
        Ok(FleetBuildReverting { commit: to.commit })
    }

    /// Start the wrapper detached and answer. **The status is written first**,
    /// so a read between the answer and the wrapper's first line already says a
    /// restart is under way.
    pub fn change(
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

impl Fleet
{
    pub fn build_checkout(&self) -> Option<PathBuf> {
        let roots = self.ledger_roots();
        checkout_of(roots.iter().map(|(_, root)| root.as_str()))
    }

    /// `get_fleet_build`. **Names the Jobs a revert moves** and lists the Jobs a restart would carry,
    /// which the files cannot: both are read off the store and the host.
    pub async fn fleet_build(&self) -> Result<FleetBuildReport, Refusal> {
        let support = crate::runtime::support_dir(&self.host().home);
        let checkout = self.build_checkout();
        let mut report = self
            .building()
            .report(checkout.as_deref(), &support, SystemTime::now())
            .await
            .map_err(|why| self.build_refusal(why))?;
        let titles = self.titles_of_held(checkout.as_deref()).await;
        if let Some(previous) = report.previous.as_mut() {
            let named = |branches: &mut Vec<String>| {
                for one in branches.iter_mut() {
                    if let Some(title) = titles.get(one.as_str()) {
                        *one = title.clone();
                    }
                }
            };
            named(&mut previous.takes_out);
            named(&mut previous.brings_back);
        }
        report.carried = self.carried_titles().await;
        Ok(report)
    }

    /// The title of the live Job that holds each branch of the repository `checkout` is in. **A Job
    /// without a title is its id**, as the Operator names one; a branch no Job holds is not here.
    async fn titles_of_held(&self, checkout: Option<&Path>) -> BTreeMap<String, String> {
        let Some(root) = checkout.map(|one| one.to_string_lossy().into_owned()) else {
            return BTreeMap::new();
        };
        let Some(served) = self.repositories().served().into_iter().find(|one| one.root() == root) else {
            return BTreeMap::new();
        };
        let store = self.store().lock().await;
        let mut titles = BTreeMap::new();
        for (session, branches) in crate::build_notices::branches_held(&store, &served) {
            let title = store.session(&session).ok().flatten().and_then(|one| one.title).unwrap_or_else(|| session.clone());
            for branch in branches {
                titles.entry(branch).or_insert_with(|| title.clone());
            }
        }
        titles
    }

    /// The titles of the hosted Jobs with a turn under way, sorted. **The narrowest reading of "at
    /// work"**: `SessionTurn::Working`, which is a turn the agent is taking. A live process between
    /// turns, a subagent or a background task after the turn has ended is not counted.
    async fn carried_titles(&self) -> Vec<String> {
        let working: Vec<String> = self
            .hosts()
            .all()
            .into_iter()
            .filter(|(_, runtime)| matches!(runtime.state().turn, ipc::SessionTurn::Working { .. }))
            .map(|(id, _)| id)
            .collect();
        let store = self.store().lock().await;
        let mut titles: Vec<String> = working
            .into_iter()
            .map(|id| store.session(&id).ok().flatten().and_then(|one| one.title).unwrap_or(id))
            .collect();
        titles.sort();
        titles
    }

    /// `revert_fleet_build`.
    pub fn revert_fleet_build_now(&self) -> Result<FleetBuildReverting, Refusal> {
        let support = crate::runtime::support_dir(&self.host().home);
        self.building()
            .revert(&support, &kept_builds::fleet_label(), SystemTime::now())
            .map_err(|why| match why {
                NotReverted::NotRevertible(said) => Refusal::IllegalMove(WireError::raised(NOT_REVERTIBLE, said, self.run_id())),
                NotReverted::Restarting(build) => Refusal::IllegalMove(WireError::raised(
                    RESTARTING,
                    format!("restarting onto {}", build_word(build)),
                    self.run_id(),
                )),
                NotReverted::Reverting => {
                    Refusal::IllegalMove(WireError::raised(REVERTING, "a revert is under way", self.run_id()))
                }
                NotReverted::NotStarted(said) => Refusal::Fault(WireError::raised(REVERT_NOT_STARTED, said, self.run_id())),
            })
    }

    /// `change_fleet_build`.
    pub fn change_fleet_build_now(
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
