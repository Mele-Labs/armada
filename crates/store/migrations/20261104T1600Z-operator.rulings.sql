-- The Operator's Log: one row for each arrival it ruled, or passed on untouched. `arrived` and
-- `source` are words (`start`, `dispatch`), and `source_session` is the Job a start came from.
-- `outcome` is the way named, or `passed_on`. `shared` is the paths the two Jobs share, one a line.
-- `call` is the model call that ruled the row, the same for items ruled together, and null where
-- no call was made: the day's calls are counted from it against the ceiling. `by` is the signer.
CREATE TABLE operator_rulings (
    position       INTEGER PRIMARY KEY AUTOINCREMENT,
    arrival        TEXT NOT NULL,
    arrived        TEXT NOT NULL,
    source         TEXT NOT NULL,
    source_session TEXT,
    session_id     TEXT NOT NULL,
    manifest_id    TEXT NOT NULL,
    subject        TEXT NOT NULL,
    outcome        TEXT NOT NULL,
    other_session  TEXT,
    shared         TEXT NOT NULL,
    because        TEXT NOT NULL,
    call           TEXT,
    by             TEXT NOT NULL,
    at             TEXT NOT NULL
) STRICT;

CREATE INDEX operator_rulings_by_at ON operator_rulings (at);
