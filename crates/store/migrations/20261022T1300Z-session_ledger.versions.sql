-- Every version a ledger attachment has stood in. `ledger_attachments` keeps one
-- row a thing and replaces it in place, so a Job's artifacts had no history.
--
-- Triggers write it, because six writers change the row's state directly and a
-- seventh would be missed by any hook in Rust. `at` is the row's `changed_at`, which
-- each writer stamps when it moves state or detail. The holder is not a foreign key,
-- as in `ledger_attachments`, so nothing here cascades: the store deletes a
-- session's versions where it deletes the session.
CREATE TABLE ledger_attachment_versions (
    seq         INTEGER PRIMARY KEY AUTOINCREMENT,
    holder_kind TEXT NOT NULL,
    holder_id   TEXT NOT NULL,
    kind        TEXT NOT NULL,
    manifest_id TEXT NOT NULL,
    target      TEXT NOT NULL,
    state       TEXT NOT NULL,
    detail      TEXT NOT NULL,
    at          TEXT NOT NULL
) STRICT;

CREATE INDEX ledger_attachment_versions_by_holder
    ON ledger_attachment_versions (holder_kind, holder_id, seq);

-- A row kept before this stands in one version, the one it is in, from when it
-- began to be in it. What came before is not known.
INSERT INTO ledger_attachment_versions
    (holder_kind, holder_id, kind, manifest_id, target, state, detail, at)
SELECT holder_kind, holder_id, kind, manifest_id, target, state, detail, since
FROM ledger_attachments
ORDER BY since, holder_id, kind, target;

CREATE TRIGGER ledger_attachments_version_taken AFTER INSERT ON ledger_attachments
BEGIN
    INSERT INTO ledger_attachment_versions
        (holder_kind, holder_id, kind, manifest_id, target, state, detail, at)
    VALUES (NEW.holder_kind, NEW.holder_id, NEW.kind, NEW.manifest_id, NEW.target,
            NEW.state, NEW.detail, NEW.changed_at);
END;

-- Only a move of state or detail: a writer that restamps `changed_at` alone, as a
-- subagent stopping twice does, is not a new version.
CREATE TRIGGER ledger_attachments_version_changed AFTER UPDATE ON ledger_attachments
WHEN OLD.state IS NOT NEW.state OR OLD.detail IS NOT NEW.detail
BEGIN
    INSERT INTO ledger_attachment_versions
        (holder_kind, holder_id, kind, manifest_id, target, state, detail, at)
    VALUES (NEW.holder_kind, NEW.holder_id, NEW.kind, NEW.manifest_id, NEW.target,
            NEW.state, NEW.detail, NEW.changed_at);
END;
