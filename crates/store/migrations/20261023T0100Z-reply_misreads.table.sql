-- The sentences Fleet read from a reply as asking the person that he marked Not a question, one row
-- each for good: the reader holds near ones off, and the trainer reads them as asking nothing. Additive.
CREATE TABLE reply_misreads (
    sentence   TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    marked_at  TEXT NOT NULL
) STRICT;
