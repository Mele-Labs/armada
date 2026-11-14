//! The build Fleet runs on, where it stands against `main`, and the request that
//! moves it to the other build.
//!
//! **Fleet-wide, and not a Job's field**, `FleetCapacity`'s reason: a fact about
//! the process being asked is not about any Job. The restart itself is not
//! served here. `change_fleet_build` starts `scripts/restart-build` detached and
//! answers at once, because Fleet is the thing the script stops; what it came to
//! is read back from this report once a Fleet is up again.

use serde::{Deserialize, Serialize};

use crate::Instant;

/// Which tree Fleet and Bridge were built from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BuildSource {
    /// `main` as the checkout holds it, fast-forwarded to `origin/main`.
    Main,
    /// `main` with every in-flight branch merged in, in `.armada/preview`.
    Preview,
}

/// Where a restart under way has got to, in the order a restart takes them.
/// `Merging` is the preview's alone and `FetchingMain` is main's.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BuildStage {
    Merging,
    FetchingMain,
    BuildingFleet,
    BuildingBridge,
    RestartingFleet,
    ReopeningBridge,
    /// The build ran again after a stale build script was cleaned.
    Retrying,
}

/// Commits the running build holds that `origin/main` lacks, and commits
/// `origin/main` holds that it lacks. Both nought is aligned.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildPosition {
    pub ahead: u32,
    pub behind: u32,
}

/// Which act the restart button is. **The build it targets**: the one a restart under way is moving
/// onto, else the one in use. Bridge's `Refresh preview` and `Update to main`, whose present-tense
/// words follow `restarting`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RestartAct {
    /// Merge the in-flight branches again and restart on the preview.
    RefreshPreview,
    /// Restart on the latest `origin/main`.
    UpdateToMain,
}

/// What the build switcher draws, without its words. Carried on `FleetBuildReport::switcher`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildSwitcher {
    /// The running build is level with `origin/main`: a position that is known, with none ahead and none
    /// behind. **A position that cannot be counted is not aligned.**
    pub aligned: bool,
    /// Nothing is restarting, the build is `main` and it is aligned: there is nothing to restart onto,
    /// so the restart button is disabled rather than absent.
    pub idle_on_main: bool,
    pub restart: RestartAct,
}

/// What `get_fleet_build` answers.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FleetBuildReport {
    /// The tree the running build came from, read off `Armada/restart-source`.
    pub on: BuildSource,
    /// The commit the running build was built from, as `scripts/restart` last
    /// recorded it. **Absent where no restart has recorded one**, which is a
    /// Fleet started by hand.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub commit: Option<String>,
    /// Where that commit stands against `origin/main` as this Fleet last
    /// fetched it. **Absent where it cannot be counted**: no commit, a commit
    /// the repository no longer holds, or no `origin/main`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<BuildPosition>,
    /// The build a restart under way is moving Fleet onto. Absent when none is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub restarting: Option<BuildSource>,
    /// Where that restart has got to. **Absent where the restart has not said**,
    /// which is one only just asked for, or a wrapper that records none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stage: Option<BuildStage>,
    /// Why the last restart did not take, in one line of plain facts. Absent
    /// when it took, and when the build has moved since it failed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failed: Option<String>,
    /// The branches `scripts/preview` left out of this build, by branch, and why each was. Only
    /// the preview leaves branches out, so **empty on `main`**, with no record, and where the
    /// record was written for another build.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub left_out: Vec<LeftOutBranch>,
    /// The kept build `builds/current` names, which Fleet runs. Absent where none is kept.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub running: Option<KeptBuildRunning>,
    /// The kept build `builds/previous` names, which a revert swaps onto, and what that changes.
    /// **Absent where no previous build is kept, and where launchd does not run Fleet from
    /// `builds/current`**: a revert would then change nothing Fleet runs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub previous: Option<KeptBuildPrevious>,
    /// The titles of the hosted Jobs with a turn under way now, which their keepers carry across a
    /// restart. Empty where none is.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub carried: Vec<String>,
    /// A revert is under way: `revert_fleet_build` started it and it has not said it is done.
    /// Absent where false. A revert whose status is two minutes old is not under way.
    #[serde(default, skip_serializing_if = "is_false")]
    pub reverting: bool,
    /// The branches picked for the next snapshot, each with the title of the live Job holding it.
    /// Fleet fills `title`. Defaults to the running build's own Jobs until the person picks.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub picked: Vec<PickedJob>,
    /// The snapshot of picked Jobs: being built, ready to swap onto, or failed. Absent where none
    /// is kept or building.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snapshot: Option<KeptSnapshot>,
    /// The titles of the Jobs the running build holds that the person has not yet pressed Keep on,
    /// or the branch where no Job holds it. Empty once Keep is pressed.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub trying: Vec<String>,
    /// A swap onto the snapshot is under way: `swap_fleet_build` started it and it has not said it
    /// is done. Absent where false.
    #[serde(default, skip_serializing_if = "is_false")]
    pub swapping: bool,
    /// What the build switcher draws, read by Fleet from `on`, `position` and `restarting`. Absent from
    /// a report built without it. Since 9 Nov 2026. `docs/practices/protocol/the-title-bars-views-fleet-reads.md`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub switcher: Option<BuildSwitcher>,
    /// Branches on origin a snapshot can take that no Job here holds and none picked: ahead of
    /// `origin/main` and committed in the last fourteen days, newest first. Read off the refs Fleet
    /// last fetched, so a Mac with no Job on a branch can still try it. Since 13 Nov 2026.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub origin: Vec<OriginBranch>,
}

/// A branch on origin a snapshot can take, and when its tip was committed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OriginBranch {
    pub branch: String,
    pub committed_at: Instant,
}

/// A branch picked for the next snapshot.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PickedJob {
    pub branch: String,
    /// The title of the live Job holding the branch. Absent where no Job does.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

/// Where the snapshot of picked Jobs stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SnapshotState {
    /// `scripts/restart-build snapshot` is working.
    Building,
    /// A kept build exists that Swap can move onto.
    Ready,
    /// The last snapshot did not build.
    Failed,
}

/// The snapshot of picked Jobs, built and kept without swapping.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeptSnapshot {
    pub state: SnapshotState,
    /// Where a build under way has got to. Absent where it has not said, and when not building.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stage: Option<BuildStage>,
    /// When the kept snapshot was built. Present when `state` is `ready`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub built_at: Option<Instant>,
    /// The full commit the kept snapshot was built from. Present when `state` is `ready`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub commit: Option<String>,
    /// The Jobs the kept snapshot holds, by title, or by branch where no Job holds it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub jobs: Vec<String>,
    /// Swapping onto it closes Bridge and opens it on the snapshot's. Absent where false.
    #[serde(default, skip_serializing_if = "is_false")]
    pub bridge_reopens: bool,
    /// Why the snapshot did not build, in one line of plain facts. Present when `state` is `failed`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failed: Option<String>,
}

/// The build Fleet runs, as `builds/current/build.toml` records it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeptBuildRunning {
    /// When it was built, as `scripts/keep-build` stamped it.
    pub built_at: Instant,
    /// The full commit it was built from.
    pub commit: String,
}

/// The build before the running one, and what swapping onto it changes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeptBuildPrevious {
    pub built_at: Instant,
    pub commit: String,
    /// Its Bridge differs from the running one's, or either build lacks the hash, so Bridge
    /// closes and opens on it. Absent where false.
    #[serde(default, skip_serializing_if = "is_false")]
    pub bridge_reopens: bool,
    /// The Jobs whose change the running build holds and this one does not: each Job holding the
    /// branch, by title, or the branch where no Job does. Empty where there are none.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub takes_out: Vec<String>,
    /// The Jobs whose change this one holds and the running build does not, named as `takes_out` is.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub brings_back: Vec<String>,
}

/// A branch the preview left out, and why, in one line of plain facts.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LeftOutBranch {
    pub branch: String,
    pub why: String,
    /// Its local tip is no longer the one it was left out at. Absent where false.
    #[serde(default, skip_serializing_if = "is_false")]
    pub moved: bool,
}

fn is_false(value: &bool) -> bool {
    !*value
}

/// `change_fleet_build`'s body: move Fleet and Bridge onto a build.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangeFleetBuild {
    /// `main` restarts onto the latest `origin/main`; `preview` merges the
    /// in-flight branches onto it first and restarts onto that.
    pub build: BuildSource,
    /// Restart with a Drone working, and adopt it. **A person's say-so**: the
    /// script refuses without it, and the answer is the refusal.
    #[serde(default)]
    pub adopt: bool,
}

/// `change_fleet_build`'s answer: the restart has begun, and this is what it
/// moves onto.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FleetBuildChanging {
    pub build: BuildSource,
}

/// `update_mac`'s body: move this Mac, another of his Macs, or both onto one build, keeping running Jobs.
/// `POST /macs/update`. `docs/concepts/pocket.md`, *Builds that differ*.
///
/// | Body | Who moves, onto what |
/// |---|---|
/// | `{ mac }` | Mac `mac`, onto the build this Mac runs: Update on an `older_build` row |
/// | `{ like }` | This Mac, onto the build Mac `like` runs: Update on a `newer_build` row |
/// | `{ mac, this_mac_too, build }` | Both: Refresh preview or main, "this Mac and the other" |
/// | `{ build, commit?, ref? }` | This Mac, onto `build`, at `commit` fetched from `ref` where given |
///
/// **A call from another Mac may only take the last form**, so an update is never sent on to a third.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateMac {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mac: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub like: Option<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub this_mac_too: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub build: Option<BuildSource>,
    /// The preview commit to build, pushed by the Mac that merged it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub commit: Option<String>,
    /// The branch on `origin` that holds `commit`: `preview/<mac id>`.
    #[serde(default, rename = "ref", skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
}

/// `revert_fleet_build`'s answer: the undo has begun, onto the build with this commit.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FleetBuildReverting {
    pub commit: String,
}

/// `pick_fleet_build_job`'s body: pick a Job's branch for the next snapshot, or take it back out.
/// **The branch, or the Job holding it**: a chip names its branch, and Try it on a Job's row names
/// the Job, whose branch Fleet reads off the slot it holds.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PickFleetBuildJob {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    pub picked: bool,
}

/// `pick_fleet_build_job`'s answer: the snapshot has begun, over these branches.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FleetBuildSnapshotting {
    pub picked: Vec<String>,
}

/// `swap_fleet_build`'s answer: the swap has begun, onto the snapshot with this commit.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FleetBuildSwapping {
    pub commit: String,
}

/// `keep_fleet_build`'s answer: the titles of the Jobs told to land.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FleetBuildKept {
    pub told: Vec<String>,
}
