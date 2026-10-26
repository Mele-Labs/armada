-- The issues linked to a written task, by the tracker's reference only: their title and whether
-- they are open are the tracker's, read when the rows are. `position` is the order they were linked.
CREATE TABLE project_task_issues (
    task_id  TEXT NOT NULL,
    issue    TEXT NOT NULL,
    position INTEGER NOT NULL,
    PRIMARY KEY (task_id, issue)
) STRICT;
