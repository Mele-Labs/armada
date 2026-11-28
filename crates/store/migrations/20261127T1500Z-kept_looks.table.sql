-- The look the owner approved on a native walk: the pictures Fleet took from the build he approved, kept so the
-- screen that lands is checked against them. Additive. docs/concepts/session/showing-a-window.md, *The look he approved*.
--
-- One row a (Job, walk): a newer approval of the same walk replaces it. `folder` holds one picture a step.
CREATE TABLE kept_looks (
    session_id TEXT NOT NULL,
    walk       TEXT NOT NULL,
    url        TEXT NOT NULL,
    commit_sha TEXT NOT NULL,
    at         TEXT NOT NULL,
    folder     TEXT NOT NULL,
    PRIMARY KEY (session_id, walk)
) STRICT;
