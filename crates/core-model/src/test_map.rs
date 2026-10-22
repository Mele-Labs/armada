//! The test map: which tests reach each file in a repository, one edge a
//! (test, file). `docs/concepts/test-map.md`.
//!
//! **Every edge carries its source and the commit it was read at**, so a
//! reader can rank two answers for one file, refresh the older one, and doubt
//! either. A collector that cannot fence one test names the test file, and the
//! edge says so through its grain.

use alloc::string::String;

use crate::envelope::Timestamp;
use crate::ManifestId;

/// Where an edge was learned. **Declared sharpest last, so the derived order
/// is the rank**: a measured edge outranks a static one, a static one a
/// model's reading, and the Manifest's declaration is the floor.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EdgeSource {
    /// `when` and `narrow` on the Check. What runs today.
    Declared,
    /// A model read the test and the code and named what it reaches. A guess,
    /// and it says so.
    Reading,
    /// Imports and calls, as a static graph reads them.
    Static,
    /// A run with coverage on: the runtime's own collector, one lap a test.
    Measured,
}

impl EdgeSource {
    pub const ALL: &'static [EdgeSource] = &[
        EdgeSource::Declared,
        EdgeSource::Reading,
        EdgeSource::Static,
        EdgeSource::Measured,
    ];

    pub fn as_wire(self) -> &'static str {
        match self {
            EdgeSource::Declared => "declared",
            EdgeSource::Reading => "reading",
            EdgeSource::Static => "static",
            EdgeSource::Measured => "measured",
        }
    }

    pub fn from_wire(value: &str) -> Option<EdgeSource> {
        EdgeSource::ALL
            .iter()
            .copied()
            .find(|source| source.as_wire() == value)
    }

    /// Whether an edge from this source is the one to keep over one from
    /// `other` for the same (test, file).
    pub fn outranks(self, other: EdgeSource) -> bool {
        self > other
    }
}

/// What an edge's test names: one test, or the test file where the collector
/// could not fence one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EdgeGrain {
    Test,
    File,
}

impl EdgeGrain {
    pub const ALL: &'static [EdgeGrain] = &[EdgeGrain::Test, EdgeGrain::File];

    pub fn as_wire(self) -> &'static str {
        match self {
            EdgeGrain::Test => "test",
            EdgeGrain::File => "file",
        }
    }

    pub fn from_wire(value: &str) -> Option<EdgeGrain> {
        EdgeGrain::ALL
            .iter()
            .copied()
            .find(|grain| grain.as_wire() == value)
    }
}

/// One edge of a repository's map: this test reaches this file, as one source
/// saw it at one commit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TestMapEdge {
    pub repository: ManifestId,
    /// The Check's key as `armada check` takes it: bare at the root,
    /// `<dir>:<name>` in a workspace.
    pub check: String,
    /// The member the test is in, as the Check's `narrow.under` spells it: a
    /// crate name, a package. Empty where the Check has no members.
    pub member: String,
    /// The test's name as the Check's runner takes it, or the test file where
    /// `grain` is [`EdgeGrain::File`].
    pub test: String,
    /// Relative to the repository root, `/`-separated.
    pub file: String,
    pub source: EdgeSource,
    pub grain: EdgeGrain,
    /// The commit the repository was at when the edge was read.
    pub commit: String,
    pub at: Timestamp,
    /// The file changed since the edge was read. Still an answer, said as such.
    pub stale: bool,
}
