-- What this Mac syncs with each profile's shared home, one switch a kind. A kind with no row is off. Additive.
CREATE TABLE home_kinds (
    login  TEXT NOT NULL,
    kind   TEXT NOT NULL,
    synced INTEGER NOT NULL,
    PRIMARY KEY (login, kind)
) STRICT;
