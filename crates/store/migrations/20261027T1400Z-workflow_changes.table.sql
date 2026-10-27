-- A change to a workflow that an agent drafted and the owner has not yet settled: one row for each
-- request or suggestion, holding the definition as it was and as drafted. Additive.
CREATE TABLE workflow_changes (
    id             INTEGER PRIMARY KEY AUTOINCREMENT,
    manifest_id    TEXT NOT NULL,
    workflow_id    TEXT NOT NULL,
    source         TEXT NOT NULL,
    step_id        TEXT,
    asked          TEXT,
    nudges         TEXT NOT NULL DEFAULT '',
    before_text    TEXT NOT NULL,
    after_text     TEXT,
    state          TEXT NOT NULL,
    failed_because TEXT,
    notes          TEXT,
    created_at     TEXT NOT NULL,
    changed_at     TEXT NOT NULL
) STRICT;
CREATE INDEX workflow_changes_by_workflow ON workflow_changes (workflow_id, source, state);
