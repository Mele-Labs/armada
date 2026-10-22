-- The follow-ups a Session's agent said it keeps for itself, one row each, until they are settled:
-- done, filed as an issue, or dropped. The same sentence is one row for good, so a reply read
-- again does not bring a settled one back. Additive.
CREATE TABLE session_later (
    session_id TEXT NOT NULL,
    n          INTEGER NOT NULL,
    sentence   TEXT NOT NULL,
    said_at    TEXT NOT NULL,
    how        TEXT,
    note       TEXT,
    settled_at TEXT,
    PRIMARY KEY (session_id, n),
    UNIQUE (session_id, sentence)
) STRICT;
