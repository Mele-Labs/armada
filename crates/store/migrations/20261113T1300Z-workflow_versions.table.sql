-- The versions of a workflow in one place, and what a copy of one of Armada's was based on. `source` and `project` say where
-- it is kept (`project` is empty unless `source` is `project`); `manifest_id` is the repository, and only where `source` is
-- `repository`; `item_id` is the workflow's id. A version is a publish, or the text that was in the place when the first
-- publish came (`origin` is `published` or `found`), numbered from 1 in the order they were written; `hash` is the sha256 of
-- `text` in hex. `workflow_bases` is one row for each copy of Armada's workflow whose base is known: the `version` Armada's
-- carried definition declared, and the `hash` and `text` of what the copy began from. Additive.
CREATE TABLE workflow_versions (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    source      TEXT NOT NULL,
    project     TEXT NOT NULL DEFAULT '',
    manifest_id TEXT NOT NULL DEFAULT '',
    item_id     TEXT NOT NULL,
    version     INTEGER NOT NULL,
    text        TEXT NOT NULL,
    hash        TEXT NOT NULL,
    at          TEXT NOT NULL,
    origin      TEXT NOT NULL,
    UNIQUE (source, project, manifest_id, item_id, version)
) STRICT;
CREATE TABLE workflow_bases (
    source      TEXT NOT NULL,
    project     TEXT NOT NULL DEFAULT '',
    manifest_id TEXT NOT NULL DEFAULT '',
    item_id     TEXT NOT NULL,
    version     INTEGER NOT NULL,
    hash        TEXT NOT NULL,
    text        TEXT NOT NULL,
    PRIMARY KEY (source, project, manifest_id, item_id)
) STRICT;
