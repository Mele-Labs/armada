-- Each gate of a Job's plan that closed, written once from the fact Fleet saw and never worked out
-- again. `version` is the plan version for `plan_approved` and 0 for every other gate, so a plan
-- recorded again keeps what closed while its approval is asked again. `source` names the fact.
CREATE TABLE session_gates (
    session_id  TEXT NOT NULL,
    group_id    TEXT NOT NULL,
    gate        TEXT NOT NULL,
    version     INTEGER NOT NULL DEFAULT 0,
    closed_at   TEXT NOT NULL,
    fact        TEXT NOT NULL,
    source      TEXT NOT NULL,
    commit_sha  TEXT,
    already_red TEXT,
    skipped     INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (session_id, group_id, gate, version)
);
