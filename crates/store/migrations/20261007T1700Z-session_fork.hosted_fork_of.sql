-- The session a hosted one was forked from, so its first process starts as a
-- copy of that conversation. docs/concepts/session.md, *A forked session*.
ALTER TABLE hosted_sessions ADD COLUMN fork_of TEXT;
