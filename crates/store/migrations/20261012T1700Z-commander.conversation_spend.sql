-- What a Job's Commander spent, one row per turn it was woken for. Apart from job_drone_spend so
-- that the cost cap, which sums that table, can never be spent by the agent that raises it; the
-- Job's cost shown to a person is both. A turn is a process of its own, so its figure is its own.
CREATE TABLE job_commander_spend (
    job_id      TEXT    NOT NULL REFERENCES jobs(job_id),
    turn_id     TEXT    NOT NULL,
    cost_micros INTEGER,
    turns       INTEGER NOT NULL,
    PRIMARY KEY (job_id, turn_id)
) STRICT;
