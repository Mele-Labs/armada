-- A Session dispatched to wait on another: its brief, what it waits for and where it starts from,
-- kept until the condition is met. `condition` is `ends`, `group_done`, `task_done`, `pr_pushed` or
-- `pr_lands`; `part` is the plan group or task a `group_done` or `task_done` names. `pull_number` and
-- `pull_branch` are the pull request a `pr_*` condition is on once the Session waited on has one.
-- `held` is a Session an agent started, whose first message waits for the person once the condition is
-- met. `told` is set once the Session waited on has been told of the wait. `started_at` is set when it started, and the row stays. No foreign key. Additive.
-- docs/concepts/session.md, *A Session that waits on another*.
CREATE TABLE session_waits (
    session_id  TEXT NOT NULL PRIMARY KEY,
    on_session  TEXT NOT NULL,
    condition   TEXT NOT NULL,
    part        TEXT NOT NULL DEFAULT '',
    base        TEXT NOT NULL,
    needs       TEXT NOT NULL DEFAULT '',
    brief       TEXT NOT NULL,
    held        INTEGER NOT NULL DEFAULT 0,
    told        INTEGER NOT NULL DEFAULT 0,
    by          TEXT NOT NULL,
    pull_number TEXT,
    pull_branch TEXT,
    created_at  TEXT NOT NULL,
    started_at  TEXT
) STRICT;
CREATE INDEX session_waits_on ON session_waits (on_session);
