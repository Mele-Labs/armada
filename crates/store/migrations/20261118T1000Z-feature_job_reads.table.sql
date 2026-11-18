-- A document, page or ticket a Job read, kept until a write of the Job's reaches a feature: only then is it linked
-- (docs/concepts/feature-catalog.md, *Docs and tickets a Job read*). `kind` is `doc` or `ticket`; `target` is the
-- address, path or id; `title` and `gist` are the mod's cut of the result, about 300 characters together, empty where
-- the result was not visible. `at` is the first read: a repeat keeps it. Additive.
CREATE TABLE feature_job_reads (
    job    TEXT NOT NULL,
    kind   TEXT NOT NULL,
    target TEXT NOT NULL,
    title  TEXT NOT NULL DEFAULT '',
    gist   TEXT NOT NULL DEFAULT '',
    at     TEXT NOT NULL,
    PRIMARY KEY (job, kind, target)
) STRICT;
