-- The reports box checked when an Autopilot starts, and the weekly usage cap in percent. A row from
-- before reads as reports off and no weekly cap.
ALTER TABLE autopilots ADD COLUMN on_reports INTEGER NOT NULL DEFAULT 0;
ALTER TABLE autopilots ADD COLUMN weekly_cap_percent INTEGER;
