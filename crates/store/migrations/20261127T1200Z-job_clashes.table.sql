-- Two Jobs that write the same files, and the order they settled: agreed between them with `settle_clash`, or raised
-- with `raise_conflict` naming files and ruled by the person. One row a pair: `pair_a` and `pair_b` are the two
-- session ids sorted, and a later settling or raise replaces the row, its `files` the union of both. `recorded_by` is
-- the Job that settled or raised it last. `files` is a JSON array of paths. `standing` is `agreed`, `ruled` or
-- `waiting`; `first` is the session id that lands first, NULL where both go on or while it waits. No foreign key: a
-- clash outlives its Jobs. Additive. docs/concepts/fleet.md, *Claims a Job holds*.
CREATE TABLE job_clashes (
    pair_a      TEXT NOT NULL,
    pair_b      TEXT NOT NULL,
    recorded_by TEXT NOT NULL,
    a_title     TEXT NOT NULL,
    b_title     TEXT NOT NULL,
    files       TEXT NOT NULL,
    standing    TEXT NOT NULL,
    first       TEXT,
    why         TEXT NOT NULL,
    at          TEXT NOT NULL,
    PRIMARY KEY (pair_a, pair_b)
) STRICT;
CREATE INDEX job_clashes_by_b ON job_clashes (pair_b);
