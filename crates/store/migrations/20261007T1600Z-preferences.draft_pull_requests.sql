-- Whether this machine offers a pull request as a draft: a preference with a
-- table of its own, one row or none. `preferences` names its set in a CHECK,
-- which SQLite cannot widen without a rebuild, and a rebuild is breaking.

CREATE TABLE draft_pull_requests (
    id    INTEGER PRIMARY KEY CHECK (id = 1),
    value INTEGER NOT NULL CHECK (value IN (0, 1))
) STRICT;
