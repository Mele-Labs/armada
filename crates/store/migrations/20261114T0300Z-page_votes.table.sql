-- Each cast of a vote a Job's page declares, by the page's artifact id (its path), the kept revision voted on
-- and the vote's id. `form` is `pick`, `rank` or `score`, and `result` the cast as the wire carries it, JSON.
-- Every cast is kept; the newest per vote is the one served. Kept apart from the page's attachment, which a new
-- revision rewrites whole. Additive. docs/concepts/artifact.md, *Page votes*.
CREATE TABLE page_votes (
    session_id TEXT NOT NULL,
    artifact   TEXT NOT NULL,
    revision   INTEGER NOT NULL,
    vote       TEXT NOT NULL,
    form       TEXT NOT NULL,
    result     TEXT NOT NULL,
    at         TEXT NOT NULL
) STRICT;
CREATE INDEX page_votes_session ON page_votes (session_id, artifact);
