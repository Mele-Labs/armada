-- Each press of an action a Job's page declares, by the page's artifact id (its path), the kept revision pressed
-- and the action's id. `did` is what the press did: `sent`, `started` or `answered`, and `target` the Job it
-- started or the ask it answered. Kept apart from the page's attachment, which a new revision rewrites whole.
-- Additive. docs/concepts/artifact.md, *Page actions*.
CREATE TABLE page_presses (
    session_id TEXT NOT NULL,
    artifact   TEXT NOT NULL,
    revision   INTEGER NOT NULL,
    action     TEXT NOT NULL,
    item       TEXT,
    act        TEXT NOT NULL,
    at         TEXT NOT NULL,
    did        TEXT NOT NULL,
    target     TEXT
) STRICT;
CREATE INDEX page_presses_session ON page_presses (session_id, artifact);
