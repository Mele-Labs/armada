-- What happened that needed no decision from the person, for the Operator page's feed: the Operator started a Job, a Job
-- asked it or told it something, it settled a card. A Job that ended and a pull request that merged are read from the
-- ledger, so no row is kept for them. `kind` is `dispatched`, `asked`, `told` or `settled`.
CREATE TABLE operator_feed (
    n INTEGER PRIMARY KEY AUTOINCREMENT,
    at TEXT NOT NULL,
    kind TEXT NOT NULL,
    title TEXT NOT NULL,
    session_id TEXT,
    said TEXT
);
CREATE INDEX operator_feed_at ON operator_feed (at);
