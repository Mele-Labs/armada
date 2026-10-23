-- The Logbook: what Fleet learnt from every Session of a repository (docs/concepts/logbook.md).
-- `logbook_entries` is the working record. `section` is `repo`, `machine` or `habits`;
-- `repository` is the repository's key for `repo` and empty for the other two. `paths` is the
-- globs an entry applies to, one per line. Times are milliseconds since the epoch.
-- `logbook_evidence` is the Session and record rows behind each claim, and `logbook_history`
-- each text an entry had before a rewrite. `logbook_forgotten` is the deny list, keyed by the
-- text's words, so a forgotten entry never comes back. `logbook_distils` is one row per distil,
-- which says where a Session's next distil starts and counts a repository's Sessions for fading.
-- No foreign key. Additive.
CREATE TABLE logbook_entries (
    id               INTEGER PRIMARY KEY,
    section          TEXT NOT NULL,
    repository       TEXT NOT NULL,
    kind             TEXT NOT NULL,
    text             TEXT NOT NULL,
    paths            TEXT NOT NULL DEFAULT '',
    first_seen       INTEGER NOT NULL,
    last_confirmed   INTEGER NOT NULL,
    last_served      INTEGER,
    state            TEXT NOT NULL,
    pinned           INTEGER NOT NULL DEFAULT 0,
    edited_by_person INTEGER NOT NULL DEFAULT 0,
    tripped          INTEGER NOT NULL DEFAULT 0
) STRICT;
CREATE INDEX logbook_entries_by_repository ON logbook_entries (section, repository);
CREATE TABLE logbook_evidence (
    entry_id   INTEGER NOT NULL,
    n          INTEGER NOT NULL,
    session_id TEXT NOT NULL,
    cites      TEXT NOT NULL,
    how        TEXT NOT NULL,
    hard       INTEGER NOT NULL DEFAULT 0,
    at         INTEGER NOT NULL,
    PRIMARY KEY (entry_id, n)
) STRICT;
CREATE TABLE logbook_history (
    entry_id    INTEGER NOT NULL,
    n           INTEGER NOT NULL,
    text        TEXT NOT NULL,
    replaced_at INTEGER NOT NULL,
    why         TEXT NOT NULL,
    PRIMARY KEY (entry_id, n)
) STRICT;
CREATE TABLE logbook_forgotten (
    section      TEXT NOT NULL,
    repository   TEXT NOT NULL,
    words        TEXT NOT NULL,
    forgotten_at INTEGER NOT NULL,
    PRIMARY KEY (section, repository, words)
) STRICT;
CREATE TABLE logbook_distils (
    session_id TEXT NOT NULL,
    repository TEXT NOT NULL,
    covers_to  TEXT NOT NULL,
    at         INTEGER NOT NULL,
    PRIMARY KEY (session_id, covers_to)
) STRICT;
