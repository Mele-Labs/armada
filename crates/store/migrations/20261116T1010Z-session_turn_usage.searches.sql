-- How many code searches a turn ran before its first Edit or Write, so a Job's searches before its first edit can be
-- counted a week at a time (docs/concepts/feature-catalog.md, *How we know it worked*). A label count, never a command.
-- `searches` is absent for a turn read before it was counted; `searched` marks a transcript read by a build that counts,
-- so one that was not is read again from the top, once. Additive.
ALTER TABLE session_turn_usage ADD COLUMN searches INTEGER;
ALTER TABLE transcript_offsets ADD COLUMN searched INTEGER NOT NULL DEFAULT 0;
