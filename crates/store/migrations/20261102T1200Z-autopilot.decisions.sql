-- What a running Autopilot answered for the person, one row to review each. `id` is
-- `<session>:<item>` for a Job's own item, so a pass over an item already answered leaves it; a
-- decision with no item of a Job's takes the id of what it was about. `kind` is a box's word.
-- `chose` is what the helper picked and `because` its own reason. A correction replaces nothing:
-- it is kept beside the answer and the row leaves the review.
CREATE TABLE autopilot_decisions (
    position    INTEGER PRIMARY KEY AUTOINCREMENT,
    id          TEXT NOT NULL UNIQUE,
    manifest_id TEXT NOT NULL,
    session_id  TEXT,
    who         TEXT NOT NULL,
    kind        TEXT NOT NULL,
    asked       TEXT NOT NULL,
    chose       TEXT NOT NULL,
    because     TEXT NOT NULL,
    at          TEXT NOT NULL,
    reviewed    INTEGER NOT NULL DEFAULT 0,
    corrected   TEXT
) STRICT;
