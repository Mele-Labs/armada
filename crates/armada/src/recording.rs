//! `armada test-map record`: the laps of one Check, measured in the repository the caller is
//! standing in and written to a file. `docs/concepts/test-map.md`, *How it stays current*.
//!
//! **Run where a rebuild costs nobody a turn**, which is the forge's runner after a merge:
//! the Rust collector builds the workspace again with coverage on. Fleet reads the file back
//! from the forge (`fleet::test_map_ci`), so the commit the laps were read at is written into
//! it and no checkout of Fleet's is trusted to say.
//!
//! **The collector is picked here, behind [`Pick`]**, so a test hands the command a collector
//! that answers from memory and never runs a coverage build on the machine that tests it.

use std::fmt;
use std::path::{Path, PathBuf};

use adapter_traits::{Lap, LapGrain, TestCollector, TestsToRecord};
use adapters::NextestCoverage;
use checks_runner::CheckWidth;
use core_model::EdgeGrain;

use crate::manifests::Manifests;

/// Which collector answers for a Check, or none. Given the Check and the width it may run at.
pub type Pick<'a> = &'a dyn Fn(&config::Check, CheckWidth) -> Option<Box<dyn TestCollector>>;

/// What was written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Recorded {
    pub collector: String,
    pub commit: String,
    pub laps: usize,
    pub out: PathBuf,
}

impl fmt::Display for Recorded {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            out,
            "{} laps from the {} collector, read at {}, written to {}",
            self.laps,
            self.collector,
            self.commit,
            self.out.display()
        )
    }
}

/// Why nothing was written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NotRecorded {
    /// The Manifest would not load, or the key names no Check in it.
    Declared(String),
    /// No collector answers for the Check. Only a Check that runs `cargo nextest` has one.
    NoCollector { check: String, run: String },
    NoCommit(String),
    /// The collector's own words.
    Collector { name: String, said: String },
    NotWritten { out: PathBuf, said: String },
}

impl fmt::Display for NotRecorded {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NotRecorded::Declared(said) => write!(out, "{said}"),
            NotRecorded::NoCollector { check, run } => write!(
                out,
                "no collector measures the Check `{check}`, which runs `{run}` — \
                 the Rust one answers for a Check that runs `cargo nextest`"
            ),
            NotRecorded::NoCommit(said) => write!(out, "no commit to read the laps at: {said}"),
            NotRecorded::Collector { name, said } => write!(out, "the {name} collector recorded nothing: {said}"),
            NotRecorded::NotWritten { out: file, said } => write!(out, "{} was not written: {said}", file.display()),
        }
    }
}

impl std::error::Error for NotRecorded {}

/// The collector Armada picks for a Check. A Check that runs `cargo nextest` gets coverage at its
/// own narrowing and width; any other Check has no collector yet.
pub fn collector_for(check: &config::Check, width: CheckWidth) -> Option<Box<dyn TestCollector>> {
    let runs_nextest = check.run().split_whitespace().take(2).eq(["cargo", "nextest"]);
    runs_nextest.then(|| Box::new(NextestCoverage::for_check(check.narrow(), width)) as Box<dyn TestCollector>)
}

/// Record every test of the Check `key` of the repository at `root`, and write what was read to
/// `out` as [`ipc::RecordedLaps`].
pub fn record(root: &Path, key: &str, out: &Path, width: CheckWidth, pick: Pick<'_>) -> Result<Recorded, NotRecorded> {
    let manifests = Manifests::load(root).map_err(|why| NotRecorded::Declared(why.to_string()))?;
    let (manifest, name) = manifests.named(key);
    let check = manifest
        .check(name)
        .ok_or_else(|| NotRecorded::Declared(format!("`{key}` is not a Check declared in {}", manifest.path().display())))?;
    let width = width.narrowed_to(check.width());
    let collector = pick(check, width).ok_or_else(|| NotRecorded::NoCollector {
        check: key.to_string(),
        run: check.run().to_string(),
    })?;
    let dir = match manifest.dir() {
        "" => root.to_path_buf(),
        below => root.join(below),
    };
    let commit = adapters::territory::head_of(&dir, "HEAD").map_err(|why| NotRecorded::NoCommit(why.to_string()))?;
    let laps = collector
        .record(key, &dir.to_string_lossy(), &TestsToRecord::All)
        .map_err(|why| NotRecorded::Collector { name: collector.name().to_string(), said: why.0 })?;

    let recorded = ipc::RecordedLaps {
        commit: commit.clone(),
        check: key.to_string(),
        source: source_of(&*collector).into(),
        laps: laps.iter().map(recorded_lap).collect(),
    };
    let written = ipc::encode(&recorded).map_err(|why| NotRecorded::NotWritten { out: out.to_path_buf(), said: why.to_string() })?;
    if let Some(parent) = out.parent().filter(|parent| !parent.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)
            .map_err(|why| NotRecorded::NotWritten { out: out.to_path_buf(), said: why.to_string() })?;
    }
    std::fs::write(out, written).map_err(|why| NotRecorded::NotWritten { out: out.to_path_buf(), said: why.to_string() })?;
    Ok(Recorded { collector: collector.name().to_string(), commit, laps: laps.len(), out: out.to_path_buf() })
}

fn source_of(collector: &dyn TestCollector) -> core_model::EdgeSource {
    use adapter_traits::CollectorSource;
    match collector.source() {
        CollectorSource::Reading => core_model::EdgeSource::Reading,
        CollectorSource::Static => core_model::EdgeSource::Static,
        CollectorSource::Measured => core_model::EdgeSource::Measured,
    }
}

fn recorded_lap(lap: &Lap) -> ipc::RecordedLap {
    ipc::RecordedLap {
        member: lap.member.clone(),
        test: lap.test.clone(),
        grain: match lap.grain {
            LapGrain::Test => EdgeGrain::Test,
            LapGrain::File => EdgeGrain::File,
        }
        .into(),
        files: lap.files.clone(),
    }
}
