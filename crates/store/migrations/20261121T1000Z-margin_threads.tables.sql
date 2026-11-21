-- His margin threads on a Job's pages, each on a passage of one page by its file name, and the rounds of margin talk
-- the Job's thread draws one line each for. Kept apart from the page's attachment, which a new revision rewrites
-- whole. Additive. docs/concepts/document.md, *Margin threads*.
CREATE TABLE margin_threads (
    session_id       TEXT NOT NULL,
    id               TEXT NOT NULL,
    document         TEXT NOT NULL,
    quote            TEXT NOT NULL,
    block            TEXT,
    section          TEXT,
    -- open, answered, suggested, held or settled.
    state            TEXT NOT NULL,
    gone             INTEGER NOT NULL DEFAULT 0,
    struck           TEXT,
    added            TEXT,
    suggested_at     TEXT,
    -- accepted or rejected, once he decided the suggestion.
    decided          TEXT,
    decided_at       TEXT,
    decided_revision INTEGER,
    outcome          TEXT,
    opened_at        TEXT NOT NULL,
    PRIMARY KEY (session_id, id)
) STRICT;

CREATE TABLE margin_messages (
    session_id TEXT NOT NULL,
    thread     TEXT NOT NULL,
    seq        INTEGER NOT NULL,
    -- you or agent.
    voice      TEXT NOT NULL,
    text       TEXT NOT NULL,
    at         TEXT NOT NULL,
    held       INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (session_id, thread, seq)
) STRICT;

-- One round: a message Fleet sent the Job for his margin talk, and the row it wrote, which the round's line stands in for.
CREATE TABLE margin_rounds (
    session_id TEXT NOT NULL,
    seq        INTEGER NOT NULL,
    at         TEXT NOT NULL,
    document   TEXT NOT NULL,
    title      TEXT NOT NULL,
    kind       TEXT NOT NULL,
    row_id     TEXT,
    -- The threads it named, as a JSON array of their ids.
    threads    TEXT NOT NULL,
    PRIMARY KEY (session_id, seq)
) STRICT;

CREATE TABLE margin_round_notes (
    session_id TEXT NOT NULL,
    round      INTEGER NOT NULL,
    seq        INTEGER NOT NULL,
    thread     TEXT,
    section    TEXT,
    passage    TEXT,
    note       TEXT NOT NULL,
    PRIMARY KEY (session_id, round, seq)
) STRICT;
