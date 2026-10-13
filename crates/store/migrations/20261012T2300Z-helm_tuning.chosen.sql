-- The model and effort a person chose for a Helm conversation, read at every launch. One row a
-- conversation; a null column is the machine's own. Not a column on helm_sessions, because
-- starting fresh forgets that row and the choice outlives it.
CREATE TABLE helm_tuning (
    conversation TEXT NOT NULL PRIMARY KEY,
    model        TEXT,
    effort       TEXT
) STRICT;
