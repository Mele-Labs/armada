-- The owner's notes on a pull request's lines, kept in Fleet so a Job a note was sent to can resolve
-- it. `repository` is as the forge names it. `sent_*` are empty until Bridge sends the note to a
-- Job. Additive.
CREATE TABLE pull_request_notes (
    id         TEXT NOT NULL PRIMARY KEY,
    repository TEXT NOT NULL,
    number     INTEGER NOT NULL,
    path       TEXT NOT NULL,
    line       INTEGER NOT NULL,
    side       TEXT NOT NULL,
    body       TEXT NOT NULL,
    status     TEXT NOT NULL,
    sent_to    TEXT,
    sent_title TEXT,
    sent_at    TEXT,
    created_at TEXT NOT NULL
) STRICT;
CREATE INDEX pull_request_notes_by_pull_request ON pull_request_notes (repository, number);
