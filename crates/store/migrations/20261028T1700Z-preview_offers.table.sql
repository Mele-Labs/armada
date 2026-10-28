-- A refresh of the preview Fleet offered on a Job's Needs you list: one row a (session, branch, tip,
-- build), so a moved tip is offered once for the build and a Fleet restart does not offer it again.
-- `text` is the item's words, `raised_at` when. Whether it is still open is read from the dismissed
-- ids; rows for a build that is no longer the running one are deleted. Additive.
CREATE TABLE preview_offers (
    session_id TEXT NOT NULL,
    branch     TEXT NOT NULL,
    tip        TEXT NOT NULL,
    build      TEXT NOT NULL,
    text       TEXT NOT NULL,
    raised_at  TEXT NOT NULL,
    PRIMARY KEY (session_id, branch, tip, build)
) STRICT;
