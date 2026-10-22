-- The test map: which tests reach each file in a repository, one row an edge, with the source it
-- was learned from and the commit it was read at. One row a (test, file) per source, so a re-record
-- from the same source stands in for the last. Additive. `docs/concepts/test-map.md`.
CREATE TABLE test_map_edges (
    repository TEXT NOT NULL,
    check_key  TEXT NOT NULL,
    member     TEXT NOT NULL,
    test       TEXT NOT NULL,
    file       TEXT NOT NULL,
    source     TEXT NOT NULL,
    grain      TEXT NOT NULL,
    commit_id  TEXT NOT NULL,
    at         TEXT NOT NULL,
    stale      INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (repository, check_key, member, test, file, source)
) STRICT;
CREATE INDEX test_map_edges_by_file ON test_map_edges (repository, file);
