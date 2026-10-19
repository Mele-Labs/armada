-- A hosted session the person turned Auto land on for: Fleet enables auto-merge on its pull
-- requests as they attach. 1 is on. Additive: the column defaults to 0.
ALTER TABLE hosted_sessions ADD COLUMN auto_land INTEGER NOT NULL DEFAULT 0;
