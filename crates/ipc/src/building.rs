//! The build Fleet runs on, where it stands against `main`, and the request that
//! moves it to the other build.
//!
//! **Fleet-wide, and not a Job's field**, `FleetCapacity`'s reason: a fact about
//! the process being asked is not about any Job. The restart itself is not
//! served here. `change_fleet_build` starts `scripts/restart-build` detached and
//! answers at once, because Fleet is the thing the script stops; what it came to
//! is read back from this report once a Fleet is up again.

use serde::{Deserialize, Serialize};

/// Which tree Fleet and Bridge were built from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BuildSource {
    /// `main` as the checkout holds it, fast-forwarded to `origin/main`.
    Main,
    /// `main` with every in-flight branch merged in, in `.armada/preview`.
    Preview,
}

/// Commits the running build holds that `origin/main` lacks, and commits
/// `origin/main` holds that it lacks. Both nought is aligned.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildPosition {
    pub ahead: u32,
    pub behind: u32,
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
    /// Why the last restart did not take, in one line of plain facts. Absent
    /// when it took, and when the build has moved since it failed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failed: Option<String>,
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
