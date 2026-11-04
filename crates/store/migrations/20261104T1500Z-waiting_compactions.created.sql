-- Fleet sent `/compact` to a hosted Job that was waiting on the owner, before its prompt cache ran out
-- (docs/concepts/session/where-the-tokens-went.md, *A Job compacted while it waits*). A row a compaction:
-- the Job, when Fleet sent it, and the context of the Job's last main-thread turn before it (input,
-- cache-creation and cache-read tokens). The `next_` columns are the Job's first main-thread turn after
-- it, filled once that turn is read: when, and its input, cache-creation and cache-read tokens, so the
-- trial can set what the compaction cost against what the Job's next request wrote. Empty until then.
-- The index is for reading a window of time without reading the table. Additive.
CREATE TABLE waiting_compactions (
    session_id       TEXT NOT NULL,
    at               TEXT NOT NULL,
    context_before   INTEGER NOT NULL,
    next_at          TEXT,
    next_input       INTEGER,
    next_cache_write INTEGER,
    next_cache_read  INTEGER,
    PRIMARY KEY (session_id, at)
) STRICT, WITHOUT ROWID;
CREATE INDEX waiting_compactions_by_time ON waiting_compactions (at);
