-- breaking
-- A saved Trigger can run a Drone with a prompt, so a frozen Trigger resolves
-- to `drone` too, its prompt in `runs_name`. docs/concepts/trigger.md. SQLite
-- cannot widen a CHECK, so the table is rebuilt with its rows.
CREATE TABLE job_frozen_triggers_drone (
    job_id        TEXT NOT NULL REFERENCES jobs(job_id),
    ordinal       INTEGER NOT NULL,
    name          TEXT NOT NULL,
    moment        TEXT NOT NULL CHECK (moment IN ('step_starts', 'step_passes', 'pr_opened')),
    step_id       TEXT NOT NULL,
    source        TEXT NOT NULL CHECK (source IN ('armada', 'repository', 'machine')),
    resolution    TEXT NOT NULL CHECK (resolution IN ('command', 'skill', 'drone', 'not_in_this_repo')),
    runs_name     TEXT NOT NULL,
    asks_first    INTEGER NOT NULL CHECK (asks_first IN (0, 1)),
    block_on_fail INTEGER NOT NULL CHECK (block_on_fail IN (0, 1)),
    repair_on_fail INTEGER NOT NULL CHECK (repair_on_fail IN (0, 1)),
    PRIMARY KEY (job_id, ordinal)
) STRICT;
INSERT INTO job_frozen_triggers_drone
SELECT job_id, ordinal, name, moment, step_id, source, resolution, runs_name,
     asks_first, block_on_fail, repair_on_fail
FROM job_frozen_triggers;
DROP TABLE job_frozen_triggers;
ALTER TABLE job_frozen_triggers_drone RENAME TO job_frozen_triggers;
