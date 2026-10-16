-- A Session's plan and the files it and its subagents touched (docs/concepts/session.md, *The plan*).
-- The plan is the current version only: `record_plan` replaces the groups and tasks whole and keeps
-- what changed against the version before in `session_plan_changes`. A task keeps what the agent's own
-- task list last said of it (`listed`) and whether an edit was seen under its group's paths while it
-- was worked; its state is derived on read. Counters keep an id from being given twice. No foreign
-- keys, as `session_build` has none. Additive.
CREATE TABLE session_plan (
    session_id   TEXT NOT NULL PRIMARY KEY,
    version      INTEGER NOT NULL,
    recorded_at  TEXT NOT NULL,
    approved_at  TEXT,
    sketch_id    TEXT,
    sketch_title TEXT,
    changed      INTEGER NOT NULL CHECK (changed IN (0, 1)),
    next_group   INTEGER NOT NULL,
    next_task    INTEGER NOT NULL
) STRICT;

CREATE TABLE session_plan_groups (
    session_id TEXT NOT NULL,
    position   INTEGER NOT NULL,
    group_id   TEXT NOT NULL,
    agent      TEXT NOT NULL,
    title      TEXT NOT NULL,
    paths      TEXT NOT NULL,
    worked_at  TEXT,
    PRIMARY KEY (session_id, position)
) STRICT;

CREATE TABLE session_plan_tasks (
    session_id   TEXT NOT NULL,
    group_id     TEXT NOT NULL,
    position     INTEGER NOT NULL,
    task_id      TEXT NOT NULL,
    text         TEXT NOT NULL,
    listed       TEXT CHECK (listed IN ('pending', 'in_progress', 'completed')),
    completed_at TEXT,
    edited       INTEGER NOT NULL CHECK (edited IN (0, 1)),
    PRIMARY KEY (session_id, task_id)
) STRICT;

CREATE TABLE session_plan_changes (
    session_id TEXT NOT NULL,
    position   INTEGER NOT NULL,
    kind       TEXT NOT NULL CHECK (kind IN ('added', 'renamed', 'moved', 'removed')),
    ref_id     TEXT NOT NULL,
    other      TEXT NOT NULL,
    PRIMARY KEY (session_id, position)
) STRICT;

-- The files a Session and its subagents wrote, newest last, trimmed to a window on every write.
CREATE TABLE session_touches (
    seq        INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL,
    agent      TEXT,
    agent_id   TEXT,
    path       TEXT NOT NULL,
    act        TEXT NOT NULL CHECK (act IN ('write', 'edit', 'remove')),
    at         TEXT NOT NULL
) STRICT;

CREATE INDEX session_touches_by_session ON session_touches (session_id, seq);
