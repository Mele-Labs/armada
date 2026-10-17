-- A pick names the Session it started, not a Job. `started_job` stays for the rows written before
-- it and is read by nothing: a pick of an earlier run opens no Session, and counts toward no cap.
ALTER TABLE autopilot_picks ADD COLUMN started_session TEXT;
