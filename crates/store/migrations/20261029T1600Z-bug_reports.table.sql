-- A fault in Armada an agent reported (`report_bug`), kept on this Mac and posted nowhere. `session_id`
-- is the Job that reported and `job_title` its title at that moment, so a report reads after the Job
-- is gone. `steps`, `errors` and `logs` are each a JSON array of strings. `kept` is when the owner kept
-- it, empty until then. Whether he has read one is Bridge's own. No foreign key: a report outlives its
-- Job. Additive.
CREATE TABLE bug_reports (
    id         TEXT NOT NULL PRIMARY KEY,
    session_id TEXT NOT NULL,
    job_title  TEXT NOT NULL,
    title      TEXT NOT NULL,
    what       TEXT NOT NULL,
    steps      TEXT NOT NULL,
    errors     TEXT NOT NULL,
    logs       TEXT NOT NULL,
    at         TEXT NOT NULL,
    kept       TEXT
) STRICT;
CREATE INDEX bug_reports_by_time ON bug_reports (at);
