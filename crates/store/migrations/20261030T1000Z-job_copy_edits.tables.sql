-- What a Job changed in its own copy. `job_copy_workflows` is the text of its workflow once it has
-- changed a step: one row for each Job that did, so a Job that changed none reads the workflow as
-- the repository holds it. `job_copy_marks` is what the lanes mark as this Job's change, one row
-- for each node or rule, `lane` the lane it is on and `item` the step, state or rule, `by` who made
-- it (`job` or `you`). A mark is state, and the timeline is the log: the latest change to a node
-- is the row, and undoing it takes the row away.
-- Additive.
CREATE TABLE job_copy_workflows (
    session_id  TEXT PRIMARY KEY,
    workflow_id TEXT NOT NULL,
    path        TEXT NOT NULL DEFAULT '',
    text        TEXT NOT NULL
) STRICT;

CREATE TABLE job_copy_marks (
    session_id TEXT NOT NULL,
    lane       TEXT NOT NULL,
    item       TEXT NOT NULL,
    text       TEXT NOT NULL,
    by         TEXT NOT NULL,
    at         TEXT NOT NULL,
    PRIMARY KEY (session_id, lane, item)
) STRICT;
