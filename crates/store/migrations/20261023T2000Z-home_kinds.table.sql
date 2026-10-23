-- What this Mac syncs with the shared home, one switch a kind. A kind with no row is off. Additive.
CREATE TABLE home_kinds (
    kind   TEXT PRIMARY KEY,
    synced INTEGER NOT NULL
) STRICT;
