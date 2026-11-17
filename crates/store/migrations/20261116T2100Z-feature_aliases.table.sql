-- A slug a feature was known by before it was renamed or merged into another, so a `Feature:` trailer already in git, or an
-- act that still names it, files under the feature it became and does not make a new pending one. `slug` is the feature's
-- slug now. No foreign key: the row is a name, and a feature kept again under its old slug takes the alias away. Additive.
CREATE TABLE feature_aliases (
    project TEXT NOT NULL,
    alias   TEXT NOT NULL,
    slug    TEXT NOT NULL,
    PRIMARY KEY (project, alias)
) STRICT;
