-- A Job's artifact the agent or the person archived as stale, by the artifact's id (`sketch:<title>`, a page's path or
-- `draft:<title>`). Kept apart from the artifact's attachment, which a new revision rewrites whole, so only a restore
-- lifts it. `archived_by` is `agent` or `owner`. `item` is the Needs you id the archive dismissed, null where it waited on
-- nothing, so a restore lifts only the dismissal the archive wrote. Additive. docs/concepts/artifact.md, *Archive*.
CREATE TABLE artifact_archives (
    session_id  TEXT NOT NULL,
    artifact    TEXT NOT NULL,
    archived_by TEXT NOT NULL,
    archived_at TEXT NOT NULL,
    item        TEXT,
    PRIMARY KEY (session_id, artifact)
) STRICT;
