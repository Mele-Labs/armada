-- The line a hosted Job joins when its first write finds every slot of its repository held, kept so
-- a Fleet that restarts serves it in the same order. `place` orders a repository's line, lowest
-- first: the Line's order, which is the order Jobs joined until something places one. `since` is
-- when it joined.
CREATE TABLE slot_line (
    session_id  TEXT PRIMARY KEY,
    manifest_id TEXT NOT NULL,
    place       INTEGER NOT NULL,
    since       TEXT NOT NULL
) STRICT;

CREATE INDEX slot_line_by_place ON slot_line (manifest_id, place);
