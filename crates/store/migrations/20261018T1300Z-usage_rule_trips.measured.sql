-- What happened after a Job was told (docs/concepts/session.md, *Where the tokens went*). Filled once
-- the Job's turns cover the 60 minutes after the trip. `after_turns` is how many turns it made in that
-- hour and is what says it was measured; `burn_per_hour_after` is the hour's burn in cost units over
-- the Job's active time, left empty where the Job made fewer than 5 turns. Both are empty until then.
-- The index is for reading the trips of a window of time without reading the table. Additive.
ALTER TABLE usage_rule_trips ADD COLUMN burn_per_hour_after REAL;
ALTER TABLE usage_rule_trips ADD COLUMN after_turns INTEGER;
CREATE INDEX usage_rule_trips_by_time ON usage_rule_trips (at);
