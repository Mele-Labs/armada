-- When the person put a recorded plan away to keep talking ("Not yet"). Null for a plan not put away, and
-- for every plan recorded before this column. A set-aside version holds nothing and calls for nothing;
-- recording again is a new version, which starts without it. Additive.
ALTER TABLE session_plan ADD COLUMN set_aside_at TEXT;
