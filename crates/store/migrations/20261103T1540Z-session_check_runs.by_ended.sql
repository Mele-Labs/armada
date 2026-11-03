-- The Usage page reads Check runs by the hour and by the build, a window of time across every Job;
-- the (session) index leaves it scanning the whole table once for each build. Additive.
CREATE INDEX IF NOT EXISTS session_check_runs_by_ended ON session_check_runs (ended_at);
