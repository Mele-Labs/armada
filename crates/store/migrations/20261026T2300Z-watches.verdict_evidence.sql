-- What the agent gathered when it set parts aside as not ours: the links, the log files it saved
-- and the exact error text, each a JSON list. Empty for a verdict given before, and for `ours`.
-- Additive.
ALTER TABLE watch_verdicts ADD COLUMN links TEXT NOT NULL DEFAULT '[]';
ALTER TABLE watch_verdicts ADD COLUMN logs TEXT NOT NULL DEFAULT '[]';
ALTER TABLE watch_verdicts ADD COLUMN errors TEXT NOT NULL DEFAULT '[]';
