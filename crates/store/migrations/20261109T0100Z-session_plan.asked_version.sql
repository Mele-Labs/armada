-- The newest plan version that asked the person to approve it. A `plan_approved` close is kept for
-- the version asked, so a later version recorded without asking again stands on that approval and a
-- version that asks again is open until it is approved. 0 is a plan kept before it: its own version.
ALTER TABLE session_plan ADD COLUMN asked_version INTEGER NOT NULL DEFAULT 0;
