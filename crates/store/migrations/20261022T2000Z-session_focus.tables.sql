-- What a Job was last asked and is doing now: the thread row id of the person's latest message and the
-- plan task in progress. Fleet works both out; a version of an attachment reads them at the moment it is
-- written (`ledger_attachment_version_origins`). A task is kept only while it is in progress, so
-- finishing the last one leaves none. No foreign key, as `ledger_attachments` has none. Additive.
CREATE TABLE session_focus (
    session_id TEXT NOT NULL PRIMARY KEY,
    message_id TEXT,
    task_id    TEXT
) STRICT;
