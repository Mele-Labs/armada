-- What Fleet has already done about a stranded slot: told its last holder, started its rescue
-- Scout. One row a slot, for the stranding it names, so a restarted Fleet does not do either
-- again. Additive. docs/concepts/fleet.md, *Rescuing a stranded slot*.
CREATE TABLE slot_strandings (
    manifest_id TEXT NOT NULL,
    slot        INTEGER NOT NULL,
    branch      TEXT NOT NULL,
    since       TEXT NOT NULL,
    told_at     TEXT,
    scouted     INTEGER NOT NULL CHECK (scouted IN (0, 1)),
    PRIMARY KEY (manifest_id, slot)
) STRICT;
