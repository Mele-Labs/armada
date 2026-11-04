-- Two Jobs the Operator keeps in touch: each was told the other's plan group and the paths they
-- share, and each is told again when the other changes its plan or pushes on them. `shared` is
-- the paths, one a line. One row a pair, `one` being the Job whose arrival was ruled.
CREATE TABLE operator_in_touch (
    one    TEXT NOT NULL,
    other  TEXT NOT NULL,
    shared TEXT NOT NULL,
    since  TEXT NOT NULL,
    PRIMARY KEY (one, other)
) STRICT;

CREATE INDEX operator_in_touch_by_other ON operator_in_touch (other);
