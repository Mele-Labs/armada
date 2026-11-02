-- The boxes checked when an Autopilot starts: whether it works the backlog, whether it works what
-- is running, and the kinds of decision a helper may answer, as words joined by commas. Nothing
-- acts on them yet. A row from before reads as backlog on, running off and no decisions.
ALTER TABLE autopilots ADD COLUMN on_backlog INTEGER NOT NULL DEFAULT 1;
ALTER TABLE autopilots ADD COLUMN on_running INTEGER NOT NULL DEFAULT 0;
ALTER TABLE autopilots ADD COLUMN decisions TEXT NOT NULL DEFAULT '';
