-- A hosted Job's turn that is open, so a Fleet that comes back can tell a turn cut off by a dead
-- process from one that finished. `ended_at` is when the process was found dead with the turn open,
-- and `started_again` is how many times Fleet has started it since. No foreign key, as
-- session_limit has none. Additive. docs/concepts/session/a-job-fleet-hosts.md, *What a restart
-- does to a job*.
CREATE TABLE session_open_turn (
    session_id    TEXT NOT NULL PRIMARY KEY,
    since         TEXT NOT NULL,
    ended_at      TEXT,
    started_again INTEGER NOT NULL DEFAULT 0
) STRICT;
