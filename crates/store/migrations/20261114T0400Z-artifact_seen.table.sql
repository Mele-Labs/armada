-- The newest revision he opened of a Job's artifact, by the artifact's id (`sketch:<title>`, a page's path or
-- `draft:<title>`). One row per artifact, and it only moves forward. Kept apart from the artifact's attachment, which a
-- new revision rewrites whole. Additive. docs/concepts/artifact.md, *Seen and anchored notes*.
CREATE TABLE artifact_seen (
    session_id TEXT NOT NULL,
    artifact   TEXT NOT NULL,
    revision   INTEGER NOT NULL,
    PRIMARY KEY (session_id, artifact)
) STRICT;
