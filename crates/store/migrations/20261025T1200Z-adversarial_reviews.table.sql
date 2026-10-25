-- A Job's adversarial reviews of its plan, one a plan version: starting again on a version replaces its
-- row and its findings, and recording the plan again leaves the old version's where they were. The Job's
-- record carries the latest. A finding's anchor is `plan`, `group` or `task` with the id it names, `part`
-- the words of that part as the review read them, `cites` one per line, and `how` and `settled_at` NULL
-- while it waits. Additive. docs/concepts/adversarial-review.md.
CREATE TABLE adversarial_reviews (
    session_id  TEXT    NOT NULL,
    version     INTEGER NOT NULL,
    state       TEXT    NOT NULL,
    started_at  TEXT    NOT NULL,
    finished_at TEXT,
    failed      TEXT,
    cost_micros INTEGER,
    PRIMARY KEY (session_id, version)
) STRICT;

CREATE TABLE adversarial_findings (
    session_id  TEXT    NOT NULL,
    version     INTEGER NOT NULL,
    position    INTEGER NOT NULL,
    id          TEXT    NOT NULL,
    anchor      TEXT    NOT NULL,
    anchor_id   TEXT    NOT NULL DEFAULT '',
    part        TEXT    NOT NULL DEFAULT '',
    consequence TEXT    NOT NULL,
    expected    TEXT    NOT NULL,
    found       TEXT    NOT NULL,
    cites       TEXT    NOT NULL DEFAULT '',
    how         TEXT,
    settled_at  TEXT,
    PRIMARY KEY (session_id, version, id)
) STRICT;
