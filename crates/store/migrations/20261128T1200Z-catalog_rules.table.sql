-- The owner's reasons for how a project's feature catalog is shaped, each kept as a rule the librarian is given whole.
-- One row a reason: `text` is his words, or for a reject he gave none for, what he rejected; `act` is what he did
-- (`make_area`, `split_area`, `merge_features`, `not_together` and the rest); `features`, `areas` and `paths` are JSON
-- arrays of what it was about, as they were named then. Additive. docs/concepts/feature-catalog.md, *His rules*.
CREATE TABLE catalog_rules (
    id       TEXT PRIMARY KEY,
    project  TEXT NOT NULL,
    text     TEXT NOT NULL,
    act      TEXT NOT NULL,
    features TEXT NOT NULL DEFAULT '[]',
    areas    TEXT NOT NULL DEFAULT '[]',
    paths    TEXT NOT NULL DEFAULT '[]',
    at       TEXT NOT NULL
) STRICT;
CREATE INDEX catalog_rules_by_project ON catalog_rules (project, at);
