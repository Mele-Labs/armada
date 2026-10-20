-- The last plan reading a Session reported, encoded as `ipc::PlanUsage`, so a restarted Fleet can
-- serve it until a Session reports a newer one. One row; NULL for none yet. Additive.
-- docs/concepts/machine.md, *Quota*.
CREATE TABLE plan_reading (
    only_row INTEGER PRIMARY KEY CHECK (only_row = 1),
    reading  TEXT
) STRICT;
INSERT INTO plan_reading (only_row) VALUES (1);
