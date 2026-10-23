-- A Fleet notice a Fleet mod's rule says to raise as a call: one row each, kept for the Needs you
-- list of `session_id`. `notice` is the notice's word, `text` what the Session was told, `raised_at`
-- when. The item's id is `call:<n>`; whether it is still open is read from the dismissed ids, so a
-- dismissed call stays kept. Additive.
CREATE TABLE fleet_calls (
    n          INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL,
    notice     TEXT NOT NULL,
    text       TEXT NOT NULL,
    raised_at  TEXT NOT NULL
) STRICT;
CREATE INDEX fleet_calls_by_session ON fleet_calls (session_id);
