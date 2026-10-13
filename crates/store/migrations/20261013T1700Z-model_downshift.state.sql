-- The default model's step down while the weekly plan window runs out: where it stands, and the last
-- window an offer was made for, so a restarted Fleet neither forgets a lowered default nor offers twice.
-- One row. `phase` is NULL for none, else offered, shifted or reset. `restore_to` is what settings.json
-- held for models.default before the step down, NULL where it held nothing. Additive.
-- docs/concepts/machine.md, *Quota*.
CREATE TABLE model_downshift (
    only_row    INTEGER PRIMARY KEY CHECK (only_row = 1),
    phase       TEXT,
    percent     INTEGER NOT NULL DEFAULT 0,
    was         TEXT,
    to_model    TEXT,
    until       TEXT,
    restore_to  TEXT,
    offered_for TEXT
) STRICT;
INSERT INTO model_downshift (only_row) VALUES (1);
