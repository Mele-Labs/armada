-- Where Fleet nudged a Job off its workflow, one row for each step a Job drifted from, so the
-- workflow's page can list them under the step. Additive.
CREATE TABLE workflow_nudges (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    workflow_id  TEXT NOT NULL,
    source       TEXT NOT NULL,
    step_id      TEXT NOT NULL,
    session_id   TEXT NOT NULL,
    fact         TEXT NOT NULL,
    at           TEXT NOT NULL,
    dismissed_at TEXT,
    UNIQUE (session_id, workflow_id, step_id, fact)
) STRICT;
CREATE INDEX workflow_nudges_by_step ON workflow_nudges (workflow_id, source, step_id, at);
