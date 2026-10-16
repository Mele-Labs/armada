-- A concern keeps the three parts of the judge's refusal beside the one string the agent is sent,
-- so Bridge can draw them apart. An empty part is one the concern was raised without: a record
-- from before the columns reads as `text` alone.
ALTER TABLE session_concerns ADD COLUMN expected TEXT NOT NULL DEFAULT '';
ALTER TABLE session_concerns ADD COLUMN produced TEXT NOT NULL DEFAULT '';
ALTER TABLE session_concerns ADD COLUMN consequence TEXT NOT NULL DEFAULT '';
