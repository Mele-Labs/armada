-- Every landing the person chose for a kind of work in a repository, oldest first by id. The
-- usual landing is read off the last ten. Additive: a new table.
CREATE TABLE delivery_picks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    manifest_id TEXT NOT NULL,
    workflow TEXT NOT NULL,
    choice TEXT NOT NULL,
    picked_at TEXT NOT NULL
) STRICT;
CREATE INDEX delivery_picks_by_kind ON delivery_picks (manifest_id, workflow, id);
