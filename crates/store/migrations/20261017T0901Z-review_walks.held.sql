-- A walk Fleet holds up: the bay a pulled-down change was checked out in and who holds it, so a Fleet
-- that died holding one can give it back, and when it was opened, so one nobody closed is closed after
-- a day. A row is deleted when the walk is closed or fails. Additive.
-- docs/concepts/pull-review.md, *Walk*.
CREATE TABLE review_walks (
    id          TEXT    NOT NULL PRIMARY KEY,
    manifest_id TEXT    NOT NULL,
    number      INTEGER NOT NULL,
    slot        INTEGER NOT NULL,
    holder      TEXT    NOT NULL,
    path        TEXT    NOT NULL,
    opened_at   TEXT    NOT NULL
) STRICT;
