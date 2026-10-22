//! The test map: collectors as plugs, their laps written as edges, and the map
//! read for a set of paths. `docs/concepts/test-map.md`.
//!
//! **Fleet owns the map; a collector owns only how a lap is read.** Each lap
//! is stamped here with the collector's source, the commit the directory it
//! ran in is at, and the clock, and the store ranks sources on the way back
//! out, so two collectors that disagree about one (test, file) both keep their
//! row and the sharper one answers.

use std::path::Path;
use std::sync::Arc;

use adapter_traits::{
    AgentHarness, CollectorSource, Delivery, Lap, LapGrain, TestCollector, TestsToRecord, Vcs,
    WorkProduct,
};
use core_model::{EdgeGrain, EdgeSource, ManifestId, TestMapEdge};

use crate::daemon::Fleet;

/// Every collector this Fleet can record with, sharpest source first, so a
/// caller that wants the best reading a file can afford walks the list once.
#[derive(Clone, Default)]
pub struct Collectors(Vec<Arc<dyn TestCollector>>);

impl Collectors {
    pub fn none() -> Collectors {
        Collectors::default()
    }

    /// Add one, keeping the list sharpest first. **Two of one name are
    /// two**: a repository may add a collector beside Fleet's own.
    pub fn add(&mut self, collector: Arc<dyn TestCollector>) {
        let at = self
            .0
            .iter()
            .position(|held| held.source() < collector.source())
            .unwrap_or(self.0.len());
        self.0.insert(at, collector);
    }

    pub fn iter(&self) -> impl Iterator<Item = &Arc<dyn TestCollector>> {
        self.0.iter()
    }

    pub fn named(&self, name: &str) -> Option<&Arc<dyn TestCollector>> {
        self.0.iter().find(|held| held.name() == name)
    }
}

/// Why laps were not recorded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NotMapped {
    /// `dir` is not inside a git repository, or git would not say where it is.
    NoCommit { dir: String, said: String },
    /// The collector's own words.
    Collector { name: String, said: String },
    /// The store would not keep them.
    NotKept(String),
}

impl std::fmt::Display for NotMapped {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NotMapped::NoCommit { dir, said } => write!(out, "no commit to read the map at in {dir}: {said}"),
            NotMapped::Collector { name, said } => write!(out, "the {name} collector recorded nothing: {said}"),
            NotMapped::NotKept(said) => write!(out, "the edges were not kept: {said}"),
        }
    }
}

/// The edge source a collector's laps carry.
pub(crate) fn source_of(source: CollectorSource) -> EdgeSource {
    match source {
        CollectorSource::Reading => EdgeSource::Reading,
        CollectorSource::Static => EdgeSource::Static,
        CollectorSource::Measured => EdgeSource::Measured,
    }
}

fn grain_of(grain: LapGrain) -> EdgeGrain {
    match grain {
        LapGrain::Test => EdgeGrain::Test,
        LapGrain::File => EdgeGrain::File,
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
    /// Record `tests` of the Check `check` with `collector`, run in `dir`, and
    /// keep each lap as edges of `repository`'s map at the commit `dir` is at.
    /// Answers how many edges were kept. The collector runs on a thread of
    /// its own, since it blocks.
    pub async fn record_laps(
        &self,
        repository: &ManifestId,
        check: &str,
        dir: &str,
        collector: Arc<dyn TestCollector>,
        tests: TestsToRecord,
    ) -> Result<usize, NotMapped> {
        let at_dir = dir.to_string();
        let commit = tokio::task::spawn_blocking(move || adapters::territory::head_of(Path::new(&at_dir), "HEAD"))
            .await
            .map_err(|why| NotMapped::NoCommit { dir: dir.to_string(), said: why.to_string() })?
            .map_err(|why| NotMapped::NoCommit { dir: dir.to_string(), said: why.to_string() })?;
        let (check_key, at_dir, named) = (check.to_string(), dir.to_string(), collector.clone());
        let laps = tokio::task::spawn_blocking(move || named.record(&check_key, &at_dir, &tests))
            .await
            .map_err(|why| NotMapped::Collector { name: collector.name().to_string(), said: why.to_string() })?
            .map_err(|why| NotMapped::Collector { name: collector.name().to_string(), said: why.0 })?;
        let edges = self.edges_of(repository, check, source_of(collector.source()), &commit, &laps);
        let kept = edges.len();
        self.store()
            .lock()
            .await
            .record_test_map_edges(&edges)
            .map_err(|why| NotMapped::NotKept(why.to_string()))?;
        Ok(kept)
    }

    fn edges_of(
        &self,
        repository: &ManifestId,
        check: &str,
        source: EdgeSource,
        commit: &str,
        laps: &[Lap],
    ) -> Vec<TestMapEdge> {
        let at = self.now();
        laps.iter()
            .flat_map(|lap| {
                let at = &at;
                lap.files.iter().map(move |file| TestMapEdge {
                    repository: repository.clone(),
                    check: check.to_string(),
                    member: lap.member.clone(),
                    test: lap.test.clone(),
                    file: file.clone(),
                    source,
                    grain: grain_of(lap.grain),
                    commit: commit.to_string(),
                    at: at.clone(),
                    stale: false,
                })
            })
            .collect()
    }

    /// The map read for `paths`, grouped by Check and member for the wire.
    pub(crate) async fn tests_reaching(
        &self,
        repository: &ManifestId,
        paths: &[String],
    ) -> Result<ipc::TestsReaching, store::ReadError> {
        let reaching = self.store().lock().await.tests_reaching(repository, paths)?;
        let mut checks: Vec<ipc::ReachingCheck> = Vec::new();
        let mut edges = reaching.edges;
        edges.sort_by(|a, b| (&a.check, &a.member, &a.test, &a.file).cmp(&(&b.check, &b.member, &b.test, &b.file)));
        for edge in edges {
            let test = ipc::ReachingTest {
                test: edge.test,
                file: edge.file,
                source: edge.source.into(),
                grain: edge.grain.into(),
                commit: edge.commit,
                stale: edge.stale,
            };
            match checks.last_mut() {
                Some(group) if group.check == edge.check && group.member == edge.member => group.tests.push(test),
                _ => checks.push(ipc::ReachingCheck { check: edge.check, member: edge.member, tests: vec![test] }),
            }
        }
        Ok(ipc::TestsReaching { checks, unreached: reaching.unreached, holds_edges: reaching.holds_edges })
    }
}
