-- A rule Fleet watches a Job's token usage against was crossed, and the Job was told
-- (docs/concepts/session.md, *Where the tokens went*). A row a telling: the Job, the rule, when, the
-- figure that crossed (tokens, a percent or a count, by rule) and the burn per hour of active time in the
-- 60 minutes up to it, in cost units (active time: the gaps between consecutive turns, each counted as at
-- most 5 minutes), so what telling a Job did can be measured after. No foreign key, as
-- session_turn_usage has none. Additive.
CREATE TABLE usage_rule_trips (
    session_id           TEXT NOT NULL,
    rule                 TEXT NOT NULL,
    at                   TEXT NOT NULL,
    figure               REAL NOT NULL,
    burn_per_hour_before REAL NOT NULL,
    PRIMARY KEY (session_id, rule, at)
) STRICT, WITHOUT ROWID;
