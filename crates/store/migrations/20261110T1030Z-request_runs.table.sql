-- One row for each send of a saved request, whoever sent it, kept after the Job that sent it
-- ends. Keyed by the repository and the request's own ids, not by a Job, so no foreign key.
-- Fleet keeps the newest few for each request and drops the rest as it writes. Every text here
-- already carries a secret as `{{name}}`. `request_headers` and `response_headers` are JSON
-- arrays of name and value pairs. Additive.
CREATE TABLE request_runs (
    id               INTEGER PRIMARY KEY AUTOINCREMENT,
    repository       TEXT NOT NULL,
    collection_id    TEXT NOT NULL,
    request_id       TEXT NOT NULL,
    sent_at          TEXT NOT NULL,
    sent_by          TEXT NOT NULL,
    session_id       TEXT,
    session_title    TEXT,
    environment      TEXT,
    method           TEXT NOT NULL,
    url              TEXT NOT NULL,
    request_headers  TEXT NOT NULL,
    request_body     TEXT,
    status           INTEGER,
    ms               INTEGER NOT NULL,
    response_headers TEXT NOT NULL,
    response_body    TEXT NOT NULL,
    response_bytes   INTEGER NOT NULL,
    response_cut     INTEGER NOT NULL,
    error            TEXT
) STRICT;
CREATE INDEX request_runs_by_request ON request_runs (repository, collection_id, request_id, id DESC);
