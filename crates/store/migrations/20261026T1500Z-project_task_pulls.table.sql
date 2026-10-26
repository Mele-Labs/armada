-- The pull requests linked to a written task, by number only: their title and whether they are open
-- are the forge's, read when the rows are. `position` is the order they were linked. `asked` is 1 once
-- this link's merge has asked about the task, so a task opened again is not asked about the same merge twice.
CREATE TABLE project_task_pulls (
    task_id  TEXT NOT NULL,
    pull     INTEGER NOT NULL,
    position INTEGER NOT NULL,
    asked    INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (task_id, pull)
) STRICT;
