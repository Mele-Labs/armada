-- What Armada lacks to make a change a person asked for, one line each, kept when the drafting agent
-- answers that it can't. Additive; a change kept before this lacks nothing.
ALTER TABLE workflow_changes ADD COLUMN missing TEXT NOT NULL DEFAULT '';
