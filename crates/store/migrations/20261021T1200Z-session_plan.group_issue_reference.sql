-- The issue a plan group names, as written: the forge's number as digits, or a tracker's key such as
-- `ABC-12`. `issue` is INTEGER in a STRICT table, which refuses a key. A group kept before this has
-- its number in `issue` and none here, and reads as that number. Additive.
ALTER TABLE session_plan_groups ADD COLUMN issue_reference TEXT;
