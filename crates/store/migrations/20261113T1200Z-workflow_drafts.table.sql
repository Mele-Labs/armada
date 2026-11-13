-- A draft of a workflow, or of a lifecycle, rule or autonomy default, that the owner or an agent edits before it is
-- published: one row for each thing drafted, and the changes made to it under it, oldest first. `kind` is `workflow` or a
-- piece's kind; `source` and `project` say where it is kept (`project` is empty unless `source` is `project`); `manifest_id`
-- is the repository, and only where `source` is `repository`, since Kit's, a project's and Armada's are the same in every
-- repository; `scope` is a piece's scope (`everywhere` or `within:<id>`) and empty for a workflow; `item_id` is the
-- workflow's or the piece's id. `base` is the published text the draft began from, which Publish is held against.
-- A draft with no changes is never kept: its text is its last change's. Additive.
CREATE TABLE workflow_drafts (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    kind        TEXT NOT NULL,
    source      TEXT NOT NULL,
    project     TEXT NOT NULL DEFAULT '',
    manifest_id TEXT NOT NULL DEFAULT '',
    scope       TEXT NOT NULL DEFAULT '',
    item_id     TEXT NOT NULL,
    base        TEXT NOT NULL,
    started_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL,
    UNIQUE (kind, source, project, manifest_id, scope, item_id)
) STRICT;
CREATE TABLE workflow_draft_changes (
    id        INTEGER PRIMARY KEY AUTOINCREMENT,
    draft_id  INTEGER NOT NULL,
    made_by   TEXT NOT NULL,
    at        TEXT NOT NULL,
    text      TEXT NOT NULL,
    summary   TEXT,
    change_id INTEGER
) STRICT;
CREATE INDEX workflow_draft_changes_by_draft ON workflow_draft_changes (draft_id, id);
