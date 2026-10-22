//! A collector: one way Fleet learns which files a test reaches, as a plug.
//! `docs/concepts/test-map.md`, *Where an edge comes from*.
//!
//! **Blocking, and it says so.** A collector runs a test suite, reads a graph
//! or calls a model, none of which is Fleet's runtime's to await; a caller on
//! an async runtime runs it on a thread of its own, as [`TextModel`]'s caller
//! does. That is what keeps this crate free of a runtime.
//!
//! **[`CollectorSource`] is `core_model::EdgeSource` less one variant, and it
//! is deliberately not that type**, for [`Model`]'s reason: this crate names
//! no domain crate. Fleet, which holds both, turns one into the other. The
//! missing variant is the Manifest's declaration, which no collector reads: it
//! is the floor Fleet lays itself, so a collector cannot claim it.
//!
//! [`TextModel`]: crate::TextModel
//! [`Model`]: crate::Model

use alloc::string::String;
use alloc::vec::Vec;

/// How a collector learned a lap, sharpest last.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum CollectorSource {
    /// A model read the test and the code and named what it reaches.
    Reading,
    /// Imports and calls, as a static graph reads them.
    Static,
    /// A run with coverage on.
    Measured,
}

/// What a lap's test names: one test, or the test file where the collector
/// could not fence one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LapGrain {
    Test,
    File,
}

/// One test's list of files, as one collector saw it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Lap {
    /// The member the test is in, as the Check's `narrow.under` spells it.
    /// Empty where the Check has no members.
    pub member: String,
    /// The test's name as the Check's runner takes it, or its file where
    /// `grain` is [`LapGrain::File`].
    pub test: String,
    pub grain: LapGrain,
    /// Relative to the repository root, `/`-separated.
    pub files: Vec<String>,
}

/// What recording one lap costs, so Fleet can pick the cheapest collector that
/// can answer for a file and keep the dear ones for a free bay.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum LapCost {
    /// Reading a graph that is already there.
    Nothing,
    /// A run of the test.
    Run,
    /// A model call.
    ModelCall,
}

/// Which tests to record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TestsToRecord {
    All,
    /// By name, as the Check's runner takes them.
    Named(Vec<String>),
}

/// Why a collector recorded nothing. Its own words, for the log.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NotRecorded(pub String);

/// One source of test map edges. Fleet owns the map and ranks what each
/// collector hands it by [`source`](TestCollector::source); a collector owns
/// only how a lap is read.
pub trait TestCollector: Send + Sync {
    /// For the log and the registry.
    fn name(&self) -> &str;

    /// The source every lap this collector records carries.
    fn source(&self) -> CollectorSource;

    /// Whether recording needs the thing under test built again: an
    /// instrumented build does, a runtime shim does not. One that does records
    /// only where a rebuild costs nobody a turn — `test-map.md`, *How it stays
    /// current*.
    fn needs_rebuild(&self) -> bool;

    fn lap_cost(&self) -> LapCost;

    /// Record the laps of `tests` for the Check `check`, run in `dir`. A test
    /// that reaches nothing is a lap with no files, not a missing lap.
    fn record(&self, check: &str, dir: &str, tests: &TestsToRecord) -> Result<Vec<Lap>, NotRecorded>;
}

/// What the forge holds of the laps a commit's CI run recorded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LapsOnForge {
    /// The run is not there yet, or is still going, and the file is not uploaded. Ask again later.
    NotYet,
    /// The run finished and uploaded none, or no run of that commit records laps. Asking again
    /// will not change it.
    Never,
    /// The file the run uploaded, as its bytes. **Not read here**: this crate decodes nothing,
    /// and the one that does is the crate that knows what the file says.
    Found(Vec<u8>),
}

/// Why the forge could not be asked, in the words the tool used: no tool, not signed in, no
/// such workflow. Not an answer about the commit, so a caller does not ask again for it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NotAsked(pub String);
