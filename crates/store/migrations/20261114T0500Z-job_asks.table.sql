-- What a Job asked the Operator with `raise_conflict` or `ask_held`, kept on this Mac until it is answered.
-- `id` names the thing asked, `conflict:<a>:<b>` with the two addresses sorted or `held:<repository>:<pull or
-- branch>`, and `round` counts the asks about it, so a question put again after it was answered is a new row and an
-- open one is folded into. `session_id` is the Job that asked first and `other_session_id` the Job a conflict names.
-- `files`, `wire` and `also_raised_by` are each a JSON array of strings. `raised` is set once the ask is on the
-- sender's Needs you or in Your turn: at once while the Operator is off, and where the Operator is on when it raises
-- it. `state` is `open` or `answered`. No foreign key: an ask outlives its Jobs. Additive.
CREATE TABLE job_asks (
    id               TEXT NOT NULL,
    round            INTEGER NOT NULL,
    kind             TEXT NOT NULL,
    session_id       TEXT NOT NULL,
    job_title        TEXT NOT NULL,
    other_session_id TEXT,
    other_title      TEXT,
    manifest_id      TEXT NOT NULL,
    files            TEXT NOT NULL,
    wire             TEXT NOT NULL,
    mine             TEXT NOT NULL,
    theirs           TEXT NOT NULL,
    breaks           TEXT NOT NULL,
    pull             INTEGER,
    branch           TEXT,
    title            TEXT,
    checks           TEXT,
    also_raised_by   TEXT NOT NULL,
    raised           INTEGER NOT NULL,
    at               TEXT NOT NULL,
    state            TEXT NOT NULL,
    answered_at      TEXT,
    answer           TEXT,
    PRIMARY KEY (id, round)
) STRICT;
CREATE INDEX job_asks_by_session ON job_asks (session_id, state);
