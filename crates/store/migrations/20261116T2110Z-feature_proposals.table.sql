-- A feature an act named that the project does not hold, kept for the Operator to confirm or merge
-- (docs/concepts/feature-catalog.md). `project` is a project's id and `slug` the name as the act wrote it. `kind` is the link
-- the artifact would have made (`sketch`, `page`, `walk` or `draft`), `target` the artifact's path or title, `job` the Job that
-- made it. Additive.
CREATE TABLE feature_proposals (
    project TEXT NOT NULL,
    slug    TEXT NOT NULL,
    kind    TEXT NOT NULL,
    target  TEXT NOT NULL,
    job     TEXT NOT NULL,
    title   TEXT NOT NULL DEFAULT '',
    at      TEXT NOT NULL,
    PRIMARY KEY (project, slug, kind, target, job)
) STRICT;
