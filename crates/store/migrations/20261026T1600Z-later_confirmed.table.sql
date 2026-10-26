-- The follow-ups Fleet read from a reply that he confirmed as deferred with thumbs up, one row each
-- for good: the row reads confirmed, and the trainer reads the sentence as a follow-up. Additive.
CREATE TABLE later_confirmed (
    sentence   TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    marked_at  TEXT NOT NULL
) STRICT;
