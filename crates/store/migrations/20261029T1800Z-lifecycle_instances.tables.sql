-- What a Job holds, standing in the states of its lifecycles, and every move between them.
-- `lifecycle_jobs` names the Jobs that take instances on: a Job started before it existed has no
-- row and holds none. An instance is a lifecycle id and the key its source gives it (`ticket` and
-- `ARM-41`, `pr` and `812`). `seen_source` and `seen` are the last thing a reader saw of it, in the
-- words the source used and empty until it is read: the standing is worked out from them again
-- rather than kept, so it can only be what a read produced. `watch_id` names the watch that follows
-- the instance, empty for one no watch follows. `lifecycle_moves` is append-only, one row a change
-- of standing, `from_standing` and `to_standing` spelled as `Standing::as_wire` spells them.
-- Additive.
CREATE TABLE lifecycle_jobs (
    session_id TEXT PRIMARY KEY,
    since      TEXT NOT NULL DEFAULT ''
) STRICT;

CREATE TABLE lifecycle_instances (
    session_id  TEXT NOT NULL,
    lifecycle   TEXT NOT NULL,
    key         TEXT NOT NULL,
    taken_on_at TEXT NOT NULL DEFAULT '',
    seen_source TEXT NOT NULL DEFAULT '',
    seen        TEXT NOT NULL DEFAULT '',
    watch_id    TEXT NOT NULL DEFAULT '',
    PRIMARY KEY (session_id, lifecycle, key)
) STRICT;

CREATE TABLE lifecycle_links (
    session_id   TEXT NOT NULL,
    lifecycle    TEXT NOT NULL,
    key          TEXT NOT NULL,
    to_lifecycle TEXT NOT NULL,
    to_key       TEXT NOT NULL,
    PRIMARY KEY (session_id, lifecycle, key, to_lifecycle, to_key)
) STRICT;

CREATE TABLE lifecycle_moves (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id    TEXT NOT NULL,
    lifecycle     TEXT NOT NULL,
    key           TEXT NOT NULL,
    from_standing TEXT NOT NULL,
    to_standing   TEXT NOT NULL,
    seen_source   TEXT NOT NULL DEFAULT '',
    seen          TEXT NOT NULL DEFAULT '',
    at            TEXT NOT NULL DEFAULT ''
) STRICT;

CREATE INDEX lifecycle_moves_by_job ON lifecycle_moves (session_id, id);
CREATE INDEX lifecycle_instances_by_watch ON lifecycle_instances (watch_id) WHERE watch_id <> '';
