-- A Job's live graphs and the rows of its feeds, so a restart keeps both and runs each recipe on from when it last
-- ran. `docs/practices/protocol/a-job-shows-a-live-graph.md`. Additive.
--
-- One `job_graphs` row a (Job, graph id): `shown` is the `show_graph` body Fleet checked, as JSON, and `every_seconds`
-- its recipe's, null where the agent feeds it. Showing the id again updates the row in place, so `rowid` is the order
-- first shown. `job_graph_rows` holds each feed's newest rows; `n` counts every row the feed was given.
CREATE TABLE job_graphs (
    session_id    TEXT NOT NULL,
    id            TEXT NOT NULL,
    shown         TEXT NOT NULL,
    shown_at      TEXT NOT NULL,
    every_seconds INTEGER,
    live          INTEGER NOT NULL,
    tried_at      TEXT,
    read_at       TEXT,
    broken        TEXT,
    PRIMARY KEY (session_id, id)
) STRICT;
CREATE TABLE job_graph_rows (
    session_id TEXT NOT NULL,
    feed       TEXT NOT NULL,
    n          INTEGER NOT NULL,
    row        TEXT NOT NULL,
    PRIMARY KEY (session_id, feed, n)
) STRICT;
