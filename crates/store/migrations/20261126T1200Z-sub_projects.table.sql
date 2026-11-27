-- The sub projects the owner started from the Jobs list: a name over the Jobs he picked. Additive.
-- docs/concepts/feature-catalog.md, *Sub projects*.
--
-- One row a sub project. `jobs` is a JSON list of session ids in his order; `features` a JSON list of the feature slugs
-- whose live Jobs held one of them at the press. Ungroup deletes the row and touches no Job.
CREATE TABLE sub_projects (
    id         TEXT PRIMARY KEY,
    project    TEXT NOT NULL,
    name       TEXT NOT NULL,
    jobs       TEXT NOT NULL,
    features   TEXT NOT NULL,
    started_at TEXT NOT NULL
) STRICT;
