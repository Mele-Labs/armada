-- The project of his an Autopilot was started for, by id (`AutopilotSettings.project`), so each Job
-- it starts is for that project. Additive: NULL on every row that was started for none.
ALTER TABLE autopilots ADD COLUMN project TEXT;
