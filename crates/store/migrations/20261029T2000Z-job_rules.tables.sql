-- The copy of the lifecycles and rules a Job started with, and what the rules planned for it.
-- `job_copies` names the Jobs that froze one: a Job started before it existed has no row and
-- plans nothing. `job_copy_pieces` holds each file's text as it was read, `kind` `lifecycle` or
-- `rule`, so the copy is what was written then and not what a file says now. `job_copy_autonomy`
-- is the defaults laid over each other for the Job's repository, one row for each kind of act
-- that has one set. `rule_ran` is which rule has seen which occurrence (`OccurrenceKey` as it is
-- written out), so a restart replays nothing. `rule_acts` is one row for each act a rule planned,
-- `state` one of planned, asked, stuck, done, skipped, told or not_carried; `attempts` counts a
-- try before it is made, so one a crash cut short still counts. `rule_job_seen` is what the last
-- look at a Job showed, `step` the id of the step it was on (empty for none), for the step and Job
-- events that are found by comparing two looks.
-- Additive.
CREATE TABLE job_copies (
    session_id TEXT PRIMARY KEY,
    since      TEXT NOT NULL DEFAULT ''
) STRICT;

CREATE TABLE job_copy_pieces (
    session_id TEXT NOT NULL,
    kind       TEXT NOT NULL,
    piece_id   TEXT NOT NULL,
    path       TEXT NOT NULL DEFAULT '',
    text       TEXT NOT NULL,
    PRIMARY KEY (session_id, kind, piece_id)
) STRICT;

CREATE TABLE job_copy_autonomy (
    session_id TEXT NOT NULL,
    act_kind   TEXT NOT NULL,
    autonomy   TEXT NOT NULL,
    PRIMARY KEY (session_id, act_kind)
) STRICT;

CREATE TABLE rule_ran (
    session_id TEXT NOT NULL,
    rule_id    TEXT NOT NULL,
    occurrence TEXT NOT NULL,
    PRIMARY KEY (session_id, rule_id, occurrence)
) STRICT;

CREATE TABLE rule_acts (
    id                INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id        TEXT NOT NULL,
    rule_id           TEXT NOT NULL,
    occurrence        TEXT NOT NULL,
    position          INTEGER NOT NULL,
    on_lifecycle      TEXT NOT NULL DEFAULT '',
    on_key            TEXT NOT NULL DEFAULT '',
    entered_lifecycle TEXT NOT NULL DEFAULT '',
    entered_key       TEXT NOT NULL DEFAULT '',
    course            TEXT NOT NULL,
    state             TEXT NOT NULL,
    attempts          INTEGER NOT NULL DEFAULT 0,
    text              TEXT NOT NULL DEFAULT '',
    said              TEXT NOT NULL DEFAULT '',
    raised_at         TEXT NOT NULL DEFAULT '',
    settled_at        TEXT NOT NULL DEFAULT '',
    UNIQUE (session_id, rule_id, occurrence, position, on_lifecycle, on_key)
) STRICT;

CREATE INDEX rule_acts_by_job ON rule_acts (session_id, id);

CREATE TABLE rule_job_seen (
    session_id          TEXT PRIMARY KEY,
    step                TEXT NOT NULL DEFAULT '',
    pull_request_opened INTEGER NOT NULL DEFAULT 0,
    landed              INTEGER NOT NULL DEFAULT 0,
    ended               INTEGER NOT NULL DEFAULT 0,
    events              INTEGER NOT NULL DEFAULT 0
) STRICT;
