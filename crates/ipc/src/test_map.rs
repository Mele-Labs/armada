//! The test map, read for a set of paths: which tests reach each, and which paths nothing reaches.
//! `docs/concepts/test-map.md`. `get_tests_reaching`.

use serde::{Deserialize, Serialize};

use crate::enums::{EdgeGrain, EdgeSource};

/// What `get_tests_reaching` answers: `GET /test_map/reaching?manifest_id=&paths=`, the paths
/// newline-separated as `armada check test --changed` reads them on stdin.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestsReaching {
    /// Grouped by the Check's key and the member the tests are in, so a caller runs one Check per
    /// group. Checks by key, members by name.
    pub checks: Vec<ReachingCheck>,
    /// The asked paths no edge names, in the order they were asked. **Not an empty `checks`**: a
    /// path with no edge is a file to run wider for or write a test for, and the caller says so.
    pub unreached: Vec<String>,
    /// Whether the map holds any edge for the repository at all. **False is a
    /// map nothing has fed yet**, where every asked path is in `unreached` and
    /// none of them is known to be reached by nothing.
    pub holds_edges: bool,
}

/// One Check and member, with every test of it that reaches one of the asked paths.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReachingCheck {
    /// As `armada check` takes it: bare at the root, `<dir>:<name>` in a workspace.
    pub check: String,
    /// As the Check's `narrow.under` spells it. Empty where the Check has no members.
    pub member: String,
    pub tests: Vec<ReachingTest>,
}

/// One edge: this test reaches this file, as the sharpest source that saw it read it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReachingTest {
    /// The test's name, or its file where `grain` is `file`.
    pub test: String,
    pub file: String,
    pub source: EdgeSource,
    pub grain: EdgeGrain,
    /// The commit the edge was read at.
    pub commit: String,
    /// The file changed since the edge was read. Still an answer, said as stale.
    pub stale: bool,
}

/// The laps one run of a Check recorded, as the file `armada test-map record` writes and Fleet
/// reads back from the forge: **the one thing that travels between the runner that measured and
/// the machine that keeps the map**. Not an operation; it rides as an artifact of the run.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordedLaps {
    /// The commit the repository was at when the laps were read. Every edge made from them
    /// carries it, whatever Fleet's own checkout is at.
    pub commit: String,
    /// The Check, as `armada check` takes it.
    pub check: String,
    /// What read them, the same for every lap.
    pub source: EdgeSource,
    pub laps: Vec<RecordedLap>,
}

/// One test's files, as the collector saw them. A lap that reached nothing has no files and is
/// still a lap.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordedLap {
    /// As the Check's `narrow.under` spells it. Empty where the Check has no members.
    pub member: String,
    /// The test's name, or its file where `grain` is `file`.
    pub test: String,
    pub grain: EdgeGrain,
    /// Relative to the repository root, `/`-separated.
    pub files: Vec<String>,
}
