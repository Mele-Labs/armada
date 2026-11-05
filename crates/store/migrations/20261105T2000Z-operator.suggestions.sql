-- What the Operator would have done where its role stands at Suggest: one row each, kept until the
-- person takes it or skips it. An answer's row names the item it would settle and the option it would
-- pick, and shows on that question; any other is a row of its own on `session_id`'s Needs you list,
-- whose id is `suggestion:<n>`. `settled_at` is null while it stands, and `taken` says which press
-- settled it. Additive.
CREATE TABLE operator_suggestions (
    n             INTEGER PRIMARY KEY AUTOINCREMENT,
    arrival       TEXT NOT NULL,
    arrived       TEXT NOT NULL,
    way           TEXT NOT NULL,
    session_id    TEXT NOT NULL,
    manifest_id   TEXT NOT NULL,
    subject       TEXT NOT NULL,
    words         TEXT NOT NULL,
    group_id      TEXT,
    other_session TEXT,
    shared        TEXT NOT NULL DEFAULT '',
    first_session TEXT,
    step          TEXT,
    item          TEXT,
    pick          TEXT,
    because       TEXT NOT NULL,
    text          TEXT NOT NULL,
    raised_at     TEXT NOT NULL,
    settled_at    TEXT,
    taken         INTEGER NOT NULL DEFAULT 0
) STRICT;
CREATE INDEX operator_suggestions_by_session ON operator_suggestions (session_id);
CREATE INDEX operator_suggestions_by_arrival ON operator_suggestions (arrival);
