-- What a Session's Check run built and what it cost, so two runs of the same tree can be matched.
-- `tree` is the git tree of the slot as the run began, uncommitted changes included. `queue_wait_ms`
-- is the wait for a place, apart from `took_ms`; `cpu_ms` is the user and system time of the run's
-- child processes. Null on runs kept before this, and where `armada check` could not tell. Additive.
ALTER TABLE session_check_runs ADD COLUMN tree TEXT;
ALTER TABLE session_check_runs ADD COLUMN queue_wait_ms INTEGER;
ALTER TABLE session_check_runs ADD COLUMN cpu_ms INTEGER;
