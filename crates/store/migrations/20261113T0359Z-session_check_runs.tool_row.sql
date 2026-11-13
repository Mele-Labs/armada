-- The thread row of the agent's call that ran a Session's Check run, where Fleet could tell it. Null
-- where no call or several were running `armada check` as the run began. Additive.
ALTER TABLE session_check_runs ADD COLUMN tool_row TEXT;
