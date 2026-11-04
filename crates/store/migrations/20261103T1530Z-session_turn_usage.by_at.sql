-- The Usage page reads turns by the hour and by the build, a window of time across every Job, which
-- the (session, at) index cannot serve: the turns table is read whole for it. Additive.
CREATE INDEX IF NOT EXISTS session_turn_usage_by_at ON session_turn_usage (at);
