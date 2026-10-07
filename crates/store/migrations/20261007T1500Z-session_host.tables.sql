-- A session Fleet hosts for Bridge: what it was started with, whether its
-- process has ever run, and its thread. docs/concepts/session.md.
--
-- The ledger's own `sessions` row is the session; this is what only a hosted
-- one has. `lease_slot` and `lease_branch` are written when the first write
-- leases a slot, and they are what a restarted Fleet reads to put the session
-- back in it.

CREATE TABLE hosted_sessions (
    session_id   TEXT PRIMARY KEY,
    manifest_id  TEXT NOT NULL,
    model        TEXT,
    effort       TEXT,
    mode         TEXT NOT NULL,
    -- 1 once the agent's process has started, so the next one resumes it.
    ran          INTEGER NOT NULL DEFAULT 0,
    lease_slot   INTEGER,
    lease_branch TEXT
) STRICT;

-- One row of the thread, in the order it arrived. `body` is the row as the
-- wire carries it, kept whole and never queried; `row_id` is what replaces it.
CREATE TABLE session_rows (
    session_id TEXT NOT NULL,
    seq        INTEGER NOT NULL,
    row_id     TEXT NOT NULL,
    body       TEXT NOT NULL,
    PRIMARY KEY (session_id, seq)
) STRICT;

CREATE INDEX session_rows_by_id ON session_rows (session_id, row_id);
