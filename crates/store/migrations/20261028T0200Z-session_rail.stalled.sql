-- Steps Fleet told the Job it had stopped on, when only the Job's own act ends them: comma-separated
-- indexes, as `nudged` is. Additive; a Session kept before this reads as told about none.
ALTER TABLE session_rail ADD COLUMN stalled TEXT NOT NULL DEFAULT '';
