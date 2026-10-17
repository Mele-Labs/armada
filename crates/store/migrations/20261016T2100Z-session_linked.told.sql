-- What Fleet has already told a Session about a Session it follows, so a restart does not tell it
-- again: one row for each (linked Session, event, key). `done` has no key; `merged` is keyed by the
-- pull request's number. No foreign key, as `pull_notices` has none. Additive.
-- docs/concepts/session.md, *Linked Sessions*.
CREATE TABLE session_linked_told (
    child   TEXT NOT NULL,
    event   TEXT NOT NULL,
    key     TEXT NOT NULL,
    told_at TEXT NOT NULL,
    PRIMARY KEY (child, event, key)
) STRICT;
