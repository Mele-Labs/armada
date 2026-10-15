-- A pick names its issue by the tracker's own reference (`#12`, `ABC-12`), not by a number, which
-- a tracker with more than one project cannot keep unique. An empty reference is a pick from before
-- the column; it reads as `#` and its number. `issue_number` stays for those rows, and is written
-- as 0 for a reference that is not a number.
ALTER TABLE autopilot_picks ADD COLUMN issue_reference TEXT NOT NULL DEFAULT '';
