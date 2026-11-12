-- This Mac as its other Macs and phones know it: an id minted once, the name
-- it was given then, and its one colour. One row at most. Additive.
-- docs/practices/store-migrations.md.
CREATE TABLE pocket_this_mac (
    only      INTEGER PRIMARY KEY CHECK (only = 1),
    mac_id    TEXT NOT NULL,
    name      TEXT NOT NULL,
    colour    TEXT NOT NULL,
    minted_at INTEGER NOT NULL
) STRICT;
