-- What an agent said about one pull request, kept for the commit it read. One row a (repository,
-- pull request, head commit, kind, name): asking for a kept one runs nothing, and a new head leaves
-- the old rows where they were. `result` is the JSON of the validated answer, and NULL while the run
-- goes or after it failed. `slot` and `holder` name the bay a running row leased, so a restart can
-- give it back. Additive. docs/concepts/pull-review.md, *Agent runs*.
CREATE TABLE review_runs (
    manifest_id TEXT    NOT NULL,
    number      INTEGER NOT NULL,
    head        TEXT    NOT NULL,
    kind        TEXT    NOT NULL,
    name        TEXT    NOT NULL,
    label       TEXT    NOT NULL,
    state       TEXT    NOT NULL,
    started_at  TEXT    NOT NULL,
    finished_at TEXT,
    failed      TEXT,
    result      TEXT,
    cost_micros INTEGER,
    slot        INTEGER,
    holder      TEXT,
    PRIMARY KEY (manifest_id, number, head, kind, name)
) STRICT;
