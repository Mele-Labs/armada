-- The branches Jobs offer for the owner's preview, one row a (Job, branch). `tip` is where the branch
-- stood when the Job offered it, `about` the Job's line on it. `state` is checking, ready or failed;
-- `why` and `files` (a JSON array) say how a failed one failed. `told` is the last (tip, state) the
-- Job was told, so a state is told once. Additive.
CREATE TABLE preview_offered (
    session_id TEXT NOT NULL,
    branch     TEXT NOT NULL,
    tip        TEXT NOT NULL,
    about      TEXT NOT NULL,
    offered_at TEXT NOT NULL,
    state      TEXT NOT NULL,
    why        TEXT,
    files      TEXT NOT NULL DEFAULT '[]',
    checked_at TEXT,
    told       TEXT,
    PRIMARY KEY (session_id, branch)
) STRICT;

-- The Refresh row the owner skipped: its id, a hash of the ready set, so a different set shows it again.
CREATE TABLE preview_refresh_skipped (
    id         TEXT NOT NULL PRIMARY KEY,
    skipped_at TEXT NOT NULL
) STRICT;
