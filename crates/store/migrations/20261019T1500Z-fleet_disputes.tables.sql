-- A job's word that a Fleet notice was wrong, and the review job that settles it (`dispute_fleet`).
-- `notice_row` is the Fleet message row's id in the job's thread and `notice` its text; `spotted` is
-- `called` (the agent called `dispute_fleet`) or `read` (Fleet read it off the agent's replies);
-- `notice_key` is the notice's text with its numbers taken out, which a second dispute of the same notice
-- joins an open review by; `review` the job started or joined, empty where none was.
-- `fleet_notices` is every notice Fleet sent a hosted job that asks it something, news apart, so
-- the read of the agent's replies knows which to read; `read_at` says it was read, once.
-- No foreign key. Additive.
CREATE TABLE fleet_disputes (
    id          TEXT NOT NULL PRIMARY KEY,
    session_id  TEXT NOT NULL,
    notice_row  TEXT NOT NULL,
    notice      TEXT NOT NULL,
    why         TEXT NOT NULL,
    spotted     TEXT NOT NULL,
    at          TEXT NOT NULL,
    notice_key  TEXT NOT NULL,
    review      TEXT
) STRICT;
CREATE INDEX fleet_disputes_by_notice ON fleet_disputes (session_id, notice_row);
CREATE INDEX fleet_disputes_by_key ON fleet_disputes (notice_key);
CREATE TABLE fleet_notices (
    session_id TEXT NOT NULL,
    row_id     TEXT NOT NULL,
    at         TEXT NOT NULL,
    text       TEXT NOT NULL,
    read_at    TEXT,
    PRIMARY KEY (session_id, row_id)
) STRICT;
CREATE INDEX fleet_notices_unread ON fleet_notices (read_at);
