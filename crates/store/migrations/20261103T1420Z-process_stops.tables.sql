-- When a Job's agent stopped on its own, so the split of a Job's time can
-- tell a process that died from one reaped after a quiet spell (docs/concepts/session.md).
-- `session_restarts` kept a death only while a turn ran, and only the process coming back after a
-- Fleet restart. A reaping is Fleet's own act and is not a stop. When Fleet was away is
-- `20261103T1310Z-fleet_outages.tables.sql`'s. Additive.
CREATE TABLE session_stops (
    session_id TEXT NOT NULL,
    at         TEXT NOT NULL,
    PRIMARY KEY (session_id, at)
) STRICT, WITHOUT ROWID;
