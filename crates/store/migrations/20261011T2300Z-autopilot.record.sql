-- Autopilot: a repository works its own backlog. One row per repository, replaced when a stopped
-- one is started again, and the picks of each run. #2092, `docs/concepts/autopilot.md`.
-- `state` is running, paused or stopped. `capped_by` is the cap that stopped it (until, spend, jobs),
-- absent where a person did. `started_job` is the Job a pick started, named so a forgotten Job does
-- not take its pick with it. A pick's `state` is started, set_aside, withdrawn or dismissed.
CREATE TABLE autopilots (
    manifest_id      TEXT PRIMARY KEY,
    run_id           TEXT NOT NULL,
    instruction      TEXT NOT NULL,
    at_once          INTEGER NOT NULL,
    until            TEXT,
    spend_cap_micros INTEGER,
    job_cap          INTEGER,
    state            TEXT NOT NULL,
    started_at       TEXT NOT NULL,
    ended_at         TEXT,
    capped_by        TEXT
) STRICT;

CREATE TABLE autopilot_picks (
    pick_id      TEXT PRIMARY KEY,
    run_id       TEXT NOT NULL,
    manifest_id  TEXT NOT NULL,
    issue_number INTEGER NOT NULL,
    issue_title  TEXT NOT NULL,
    issue_url    TEXT NOT NULL,
    workflow_id  TEXT,
    started_job  TEXT,
    state        TEXT NOT NULL,
    reason       TEXT NOT NULL,
    at           TEXT NOT NULL
) STRICT;

CREATE INDEX autopilot_picks_by_run ON autopilot_picks (run_id, pick_id);
