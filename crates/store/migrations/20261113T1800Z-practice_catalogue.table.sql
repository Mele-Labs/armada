-- The practices a repository's own docs state for one kind of change in one area, read once and
-- reused by every later pull request touching the same area and kind. One row a (repository, area,
-- kind). `items` and `sources` are the JSON fleet validated; `sources` holds each doc's blob at the
-- base it was read from, so a blob that moved is read again. Additive.
-- docs/concepts/pull-review.md, *Change sets*.
CREATE TABLE practice_catalogue (
    manifest_id TEXT NOT NULL,
    area        TEXT NOT NULL,
    kind        TEXT NOT NULL,
    items       TEXT NOT NULL,
    sources     TEXT NOT NULL,
    read_at     TEXT NOT NULL,
    PRIMARY KEY (manifest_id, area, kind)
) STRICT;
