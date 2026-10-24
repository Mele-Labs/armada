-- The bead each group and task of a Job's approved plan was mirrored as, so a plan recorded again
-- finds the issue it already filed instead of filing a second. One row a plan id under the beads
-- issue the Job holds. `status` and `title` are what was last written to the bead, and
-- `blocked_by` the plan id of the group it was last ordered after: a mirror writes where the plan
-- differs from them. Additive.
CREATE TABLE plan_beads (
    session_id TEXT NOT NULL,
    issue      TEXT NOT NULL,
    plan_id    TEXT NOT NULL,
    bead       TEXT NOT NULL,
    title      TEXT NOT NULL,
    status     TEXT NOT NULL,
    blocked_by TEXT NOT NULL,
    PRIMARY KEY (session_id, issue, plan_id)
) STRICT;
