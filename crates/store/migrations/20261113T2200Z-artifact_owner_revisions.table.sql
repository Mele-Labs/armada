-- Each revision of a Job's artifact that the person wrote with his Edit, by the artifact's id (a page's path or
-- `draft:<title>`) and the revision's number from 1. Every revision not here is the agent's. Kept apart from the
-- artifact's attachment, which a new revision rewrites whole. Additive. docs/concepts/artifact.md, *His Edit*.
CREATE TABLE artifact_owner_revisions (
    session_id TEXT NOT NULL,
    artifact   TEXT NOT NULL,
    revision   INTEGER NOT NULL,
    at         TEXT NOT NULL,
    PRIMARY KEY (session_id, artifact, revision)
) STRICT;
