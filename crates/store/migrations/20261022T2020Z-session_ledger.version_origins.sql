-- Where a version of an attachment came from: the person's latest message and the plan task in progress
-- when it was written, read from `session_focus`. A side table keyed by the version's `seq`, because the
-- version row is written by triggers that cannot be changed, nor the row updated, in an additive
-- migration. A version with no row here came from nothing known: every one written before this, any a
-- holder other than a session wrote, and any a session wrote before it had a message or a task.
CREATE TABLE ledger_attachment_version_origins (
    seq          INTEGER NOT NULL PRIMARY KEY,
    from_message TEXT,
    from_task    TEXT
) STRICT;

-- A trigger on the versions, so that each of the writers of a version is covered, as the ones that write
-- the version itself are.
CREATE TRIGGER ledger_attachment_versions_origin AFTER INSERT ON ledger_attachment_versions
WHEN NEW.holder_kind = 'session'
BEGIN
    INSERT INTO ledger_attachment_version_origins (seq, from_message, from_task)
    SELECT NEW.seq, message_id, task_id FROM session_focus WHERE session_id = NEW.holder_id;
END;
