-- The sentences Fleet read from a reply as asking the person that he confirmed with thumbs up, one row
-- each for good: the item reads confirmed, and the trainer reads them as asking. Additive.
CREATE TABLE reply_confirmed (
    sentence   TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    marked_at  TEXT NOT NULL
) STRICT;
