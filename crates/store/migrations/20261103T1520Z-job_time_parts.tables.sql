-- A Job's split cut where each build began, and the wait of each pull request it had. The Usage
-- page reads figures a build and an hour at a time; these let it do that without folding every
-- Job again. Written with the Job's `job_times` row, which they sum to, and replaced with it.
-- `build_id` is `fleet_builds.id`, 0 for the stretch before the first build known. Never pruned.
-- `wait_ms` is the pull request bucket's time inside the stint, which is why it is kept and not
-- recomputed from the stint: other work in it takes the second first. Additive.
CREATE TABLE job_time_builds (
    session_id      TEXT NOT NULL,
    build_id        INTEGER NOT NULL,
    model_ms        INTEGER NOT NULL,
    tools_ms        INTEGER NOT NULL,
    checks_ms       INTEGER NOT NULL,
    subagents_ms    INTEGER NOT NULL,
    slot_ms         INTEGER NOT NULL,
    you_ms          INTEGER NOT NULL,
    pull_request_ms INTEGER NOT NULL,
    stopped_ms      INTEGER NOT NULL,
    idle_ms         INTEGER NOT NULL,
    PRIMARY KEY (session_id, build_id)
) STRICT, WITHOUT ROWID;
CREATE INDEX job_time_builds_by_build ON job_time_builds (build_id);

CREATE TABLE job_time_pull_requests (
    session_id TEXT NOT NULL,
    target     TEXT NOT NULL,
    began_at   TEXT NOT NULL,
    ended_at   TEXT NOT NULL,
    wait_ms    INTEGER NOT NULL,
    PRIMARY KEY (session_id, target, began_at)
) STRICT, WITHOUT ROWID;
CREATE INDEX job_time_pull_requests_by_end ON job_time_pull_requests (ended_at);
