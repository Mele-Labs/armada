-- The latest gate-check run of a Job's `checks_green` gate while it stands open: the commit Fleet ran
-- at, `running` or `red`, the tests that failed and the ones also red at the base on main. One row per
-- Job, replaced by the next run and removed when the gate closes.
CREATE TABLE session_gate_checks (
    session_id  TEXT NOT NULL PRIMARY KEY,
    commit_sha  TEXT NOT NULL,
    state       TEXT NOT NULL,
    failed      TEXT,
    already_red TEXT,
    at          TEXT NOT NULL
);
