-- A feature's page handed to a Job, or to one of its subagents, once each (docs/concepts/feature-catalog.md). Fleet
-- decides "already handed" by this row, so it holds across turns and restarts. `holder` is the subagent's id, empty for
-- the Job's own thread; a subagent has a context of its own and is handed what the Job already was. `via` is `brief`,
-- where the brief named the feature, or `read`, where a file of it was read or searched. Additive.
CREATE TABLE feature_handoffs (
    session_id TEXT NOT NULL,
    holder     TEXT NOT NULL DEFAULT '',
    feature_id TEXT NOT NULL,
    via        TEXT NOT NULL,
    handed_at  TEXT NOT NULL,
    PRIMARY KEY (session_id, holder, feature_id)
) STRICT;
