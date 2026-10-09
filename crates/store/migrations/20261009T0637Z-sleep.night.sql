-- Sleep mode: whether the owner has it on, since when, and what the night decided, held, landed and
-- showed. Turning it on starts a new night and clears the last one's rows. `kind` is one of decided,
-- blocked, landed, walk. `session_id` is the session a decision answered, for an override.
CREATE TABLE sleep_state (
    only_row INTEGER PRIMARY KEY CHECK (only_row = 1),
    on_now   INTEGER NOT NULL,
    since    TEXT
) STRICT;
INSERT INTO sleep_state (only_row, on_now, since) VALUES (1, 0, NULL);

CREATE TABLE sleep_rows (
    position   INTEGER PRIMARY KEY AUTOINCREMENT,
    id         TEXT NOT NULL UNIQUE,
    kind       TEXT NOT NULL,
    who        TEXT NOT NULL,
    text       TEXT NOT NULL,
    session_id TEXT,
    chose      TEXT,
    corrected  TEXT,
    pr         TEXT
) STRICT;
