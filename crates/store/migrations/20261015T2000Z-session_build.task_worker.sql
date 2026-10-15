-- Who wrote a task of a Session's list when a subagent did, as the worker's label (`Type: what it was
-- asked`), kept because the subagent's attachment is spent by the time its pieces are last read
-- (docs/concepts/session.md, *The build record*). Null for the Session's own tasks. Additive: a row kept
-- before this reads as the Session's own.
ALTER TABLE session_build_tasks ADD COLUMN made_by TEXT;
