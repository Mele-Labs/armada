-- When each step of a Session's workflow began, in order, one `step<TAB>time` line each. A step the
-- Job went back to appears again. Additive; a Session kept before this reads as no visits.
ALTER TABLE session_rail ADD COLUMN visits TEXT NOT NULL DEFAULT '';
