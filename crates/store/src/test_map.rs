//! Which tests reach each file in a repository, one row an edge.
//! `docs/concepts/test-map.md`.
//!
//! **The repository's, not a Job's**, for `timings`' reason: no row points at
//! a Job, so forgetting one keeps what its runs measured.
//!
//! **One row a (test, file) per source.** A re-record from the same source
//! stands in for the last, so a lap run twice does not pile up, and two sources
//! that disagree about one (test, file) both keep their row: a read picks the
//! sharpest and a re-recorder knows which one went stale.

use std::collections::BTreeMap;

use core_model::{EdgeGrain, EdgeSource, ManifestId, TestMapEdge, Timestamp};
use rusqlite::params;

use crate::error::{fault, ReadError, WriteError};
use crate::open::Store;
use crate::row::enum_value;

const TABLE: &str = "test_map_edges";

/// What reaches a set of paths: the sharpest edge per (test, file) among the
/// paths that have one, and the paths that have none.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TestsReaching {
    pub edges: Vec<TestMapEdge>,
    /// The asked paths no edge names, in the order they were asked.
    pub unreached: Vec<String>,
}

/// One test whose edges have gone stale, with the source that read them, so
/// the collector for that source can read them again.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct StaleTest {
    pub check: String,
    pub member: String,
    pub test: String,
    pub source: EdgeSource,
}

impl Store {
    /// Keep these edges, each in place of what the same source last recorded
    /// for the same test. One transaction, so a crash mid-way leaves the last
    /// record whole.
    pub fn record_test_map_edges(&mut self, edges: &[TestMapEdge]) -> Result<(), WriteError> {
        let failed = |why| WriteError::Database(fault("keeping test map edges")(why));
        let kept = self.conn.transaction().map_err(failed)?;
        let mut cleared: Vec<(&str, &str, &str, &str, &str)> = Vec::new();
        for edge in edges {
            let test = (
                edge.repository.as_str(),
                edge.check.as_str(),
                edge.member.as_str(),
                edge.test.as_str(),
                edge.source.as_wire(),
            );
            if !cleared.contains(&test) {
                kept.execute(
                    "DELETE FROM test_map_edges
                     WHERE repository = ?1 AND check_key = ?2 AND member = ?3 AND test = ?4
                       AND source = ?5",
                    params![test.0, test.1, test.2, test.3, test.4],
                )
                .map_err(failed)?;
                cleared.push(test);
            }
            kept.execute(
                "INSERT INTO test_map_edges
                     (repository, check_key, member, test, file, source, grain, commit_id, at, stale)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                 ON CONFLICT (repository, check_key, member, test, file, source) DO UPDATE SET
                     grain = excluded.grain, commit_id = excluded.commit_id, at = excluded.at,
                     stale = excluded.stale",
                params![
                    edge.repository.as_str(),
                    edge.check,
                    edge.member,
                    edge.test,
                    edge.file,
                    edge.source.as_wire(),
                    edge.grain.as_wire(),
                    edge.commit,
                    edge.at.as_str(),
                    i64::from(edge.stale),
                ],
            )
            .map_err(failed)?;
        }
        kept.commit().map_err(failed)
    }

    /// The tests reaching each of `paths`: per (test, file) the edge from the
    /// sharpest source, stale or not, and the paths no edge names at all.
    pub fn tests_reaching(
        &self,
        repository: &ManifestId,
        paths: &[String],
    ) -> Result<TestsReaching, ReadError> {
        let unreadable = |why: rusqlite::Error| ReadError::Database(fault("reading the tests reaching a path")(why));
        let mut statement = self
            .conn
            .prepare(
                "SELECT repository, check_key, member, test, file, source, grain, commit_id, at, stale
                 FROM test_map_edges WHERE repository = ?1 AND file = ?2",
            )
            .map_err(unreadable)?;
        let mut reaching = TestsReaching::default();
        for path in paths {
            let rows = statement
                .query_map(params![repository.as_str(), path], read_edge)
                .map_err(unreadable)?;
            let mut found = Vec::new();
            for row in rows {
                found.push(row.map_err(unreadable)??);
            }
            if found.is_empty() {
                reaching.unreached.push(path.clone());
            } else {
                reaching.edges.extend(sharpest(found));
            }
        }
        Ok(reaching)
    }

    /// Every file one test reaches, the sharpest source's edge per file.
    pub fn files_a_test_reaches(
        &self,
        repository: &ManifestId,
        check: &str,
        member: &str,
        test: &str,
    ) -> Result<Vec<TestMapEdge>, ReadError> {
        let unreadable = |why: rusqlite::Error| ReadError::Database(fault("reading the files a test reaches")(why));
        let mut statement = self
            .conn
            .prepare(
                "SELECT repository, check_key, member, test, file, source, grain, commit_id, at, stale
                 FROM test_map_edges
                 WHERE repository = ?1 AND check_key = ?2 AND member = ?3 AND test = ?4",
            )
            .map_err(unreadable)?;
        let rows = statement
            .query_map(params![repository.as_str(), check, member, test], read_edge)
            .map_err(unreadable)?;
        let mut found = Vec::new();
        for row in rows {
            found.push(row.map_err(unreadable)??);
        }
        Ok(sharpest(found))
    }

    /// Say every edge naming one of `paths` is stale. Answers how many edges
    /// that was.
    pub fn mark_test_map_stale(
        &mut self,
        repository: &ManifestId,
        paths: &[String],
    ) -> Result<usize, WriteError> {
        let failed = |why| WriteError::Database(fault("marking test map edges stale")(why));
        let marked = self.conn.transaction().map_err(failed)?;
        let mut count = 0;
        for path in paths {
            count += marked
                .execute(
                    "UPDATE test_map_edges SET stale = 1 WHERE repository = ?1 AND file = ?2",
                    params![repository.as_str(), path],
                )
                .map_err(failed)?;
        }
        marked.commit().map_err(failed)?;
        Ok(count)
    }

    /// Every test with a stale edge, once per source that read it.
    pub fn stale_tests(&self, repository: &ManifestId) -> Result<Vec<StaleTest>, ReadError> {
        let unreadable = |why: rusqlite::Error| ReadError::Database(fault("reading stale tests")(why));
        let mut statement = self
            .conn
            .prepare(
                "SELECT DISTINCT check_key, member, test, source FROM test_map_edges
                 WHERE repository = ?1 AND stale = 1
                 ORDER BY check_key, member, test, source",
            )
            .map_err(unreadable)?;
        let rows = statement
            .query_map(params![repository.as_str()], |row| {
                let source: String = row.get(3)?;
                Ok((
                    StaleTest {
                        check: row.get(0)?,
                        member: row.get(1)?,
                        test: row.get(2)?,
                        source: EdgeSource::Declared,
                    },
                    source,
                ))
            })
            .map_err(unreadable)?;
        let mut stale = Vec::new();
        for row in rows {
            let (mut test, source) = row.map_err(unreadable)?;
            test.source = source_column(&source)?;
            stale.push(test);
        }
        Ok(stale)
    }
}

/// One edge per (check, member, test, file): the sharpest source's, and the
/// latest where two sources tie.
fn sharpest(edges: Vec<TestMapEdge>) -> Vec<TestMapEdge> {
    let mut best: BTreeMap<(String, String, String, String), TestMapEdge> = BTreeMap::new();
    for edge in edges {
        let key = (edge.check.clone(), edge.member.clone(), edge.test.clone(), edge.file.clone());
        match best.get(&key) {
            Some(kept) if !edge.source.outranks(kept.source) && !(edge.source == kept.source && edge.at > kept.at) => {}
            _ => {
                best.insert(key, edge);
            }
        }
    }
    best.into_values().collect()
}

fn read_edge(row: &rusqlite::Row<'_>) -> rusqlite::Result<Result<TestMapEdge, ReadError>> {
    let source: String = row.get(5)?;
    let grain: String = row.get(6)?;
    let edge = TestMapEdge {
        repository: ManifestId::carried(core_model::Ulid::carried(row.get::<_, String>(0)?)),
        check: row.get(1)?,
        member: row.get(2)?,
        test: row.get(3)?,
        file: row.get(4)?,
        source: EdgeSource::Declared,
        grain: EdgeGrain::Test,
        commit: row.get(7)?,
        at: Timestamp::from_rfc3339(row.get::<_, String>(8)?),
        stale: row.get::<_, i64>(9)? != 0,
    };
    Ok(source_column(&source).and_then(|source| {
        enum_value(EdgeGrain::from_wire, TABLE, "grain", &grain)
            .map_err(ReadError::Unreadable)
            .map(|grain| TestMapEdge { source, grain, ..edge })
    }))
}

fn source_column(value: &str) -> Result<EdgeSource, ReadError> {
    enum_value(EdgeSource::from_wire, TABLE, "source", value).map_err(ReadError::Unreadable)
}
