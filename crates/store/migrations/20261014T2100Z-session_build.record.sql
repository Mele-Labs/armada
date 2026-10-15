-- What Fleet observed of a Session's work, for its build record (docs/concepts/session.md, *The build
-- record*): one row of instants, and the agent's task list with when each item was first seen completed
-- and whether an edit was seen while it was worked. Tables of their own because `keep_session` replaces
-- its row whole. No foreign key, as `session_limit` has none. The index is for the road: a pull request's
-- or a Session's timeline entries by kind. Additive.
CREATE TABLE session_build (
    session_id    TEXT NOT NULL PRIMARY KEY,
    begun_at      TEXT NOT NULL,
    first_edit_at TEXT,
    last_edit_at  TEXT,
    last_tool_at  TEXT,
    plan_left_at  TEXT,
    tasks_at      TEXT
) STRICT;

CREATE TABLE session_build_tasks (
    session_id    TEXT NOT NULL,
    position      INTEGER NOT NULL,
    task_id       TEXT NOT NULL,
    text          TEXT NOT NULL,
    status        TEXT NOT NULL CHECK (status IN ('pending', 'in_progress', 'completed')),
    first_seen_at TEXT NOT NULL,
    completed_at  TEXT,
    edited        INTEGER NOT NULL CHECK (edited IN (0, 1)),
    PRIMARY KEY (session_id, position)
) STRICT;

CREATE INDEX timeline_events_by_subject ON timeline_events (subject_kind, subject_id, kind);
