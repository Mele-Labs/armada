-- The procedure an agent's item asks the person to follow, in order: a JSON array of lines, empty when the item
-- is not a procedure. Additive: a row kept before this reads as no steps.
-- docs/concepts/session.md.
ALTER TABLE session_waiting ADD COLUMN steps TEXT NOT NULL DEFAULT '[]';
