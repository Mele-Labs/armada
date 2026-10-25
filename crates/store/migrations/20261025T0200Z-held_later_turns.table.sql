-- The turns that reach a Job on its start card after its first, kept off its thread until the person
-- presses Start, oldest first by `seq`. `body` is Fleet's, opaque here, as `held_first_turns.body` is.
-- Taken when the Job begins. Additive.
CREATE TABLE held_later_turns (
    seq        INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL,
    body       TEXT NOT NULL
) STRICT;
