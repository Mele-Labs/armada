-- When a snoozed Job is let back in front of the person. One row a Job; deleted when the time comes
-- or the person wakes it, and kept when the Job ends, pauses or is repaired. A table of its own
-- rather than a column on `hosted_sessions`, because `keep_hosting` replaces the whole row.
-- docs/concepts/session/snoozing-a-job.md.
CREATE TABLE session_snooze (
    session_id TEXT NOT NULL PRIMARY KEY,
    until      TEXT NOT NULL
) STRICT;
