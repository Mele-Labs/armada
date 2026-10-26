-- The follow-ups Fleet read from a reply that he marked Not deferred with thumbs down, one row each
-- for good: the sentence is not kept as one again, and the trainer reads it as no follow-up. Additive.
CREATE TABLE later_misreads (
    sentence   TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    marked_at  TEXT NOT NULL
) STRICT;
