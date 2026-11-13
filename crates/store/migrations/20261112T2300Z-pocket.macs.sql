-- The owner's other Macs this Mac is paired with, or waiting on a Confirm from. A linked Mac's
-- `public_key` verifies its signed calls; the private half of this Mac's key toward it is in the
-- Keychain, never here. Additive. docs/concepts/pocket.md, *Another Mac*.
CREATE TABLE pocket_macs (
    mac_id        TEXT PRIMARY KEY,
    name          TEXT NOT NULL,
    colour        TEXT NOT NULL,
    address       TEXT NOT NULL,
    -- SPKI DER. Absent while this Mac waits for Confirm on the other.
    public_key    BLOB,
    build         TEXT,
    paired_at     INTEGER NOT NULL,
    -- `linked` or `confirm_waiting`.
    state         TEXT NOT NULL,
    back_code     TEXT,
    last_heard_at INTEGER,
    asleep_since  INTEGER,
    why           TEXT
) STRICT;
