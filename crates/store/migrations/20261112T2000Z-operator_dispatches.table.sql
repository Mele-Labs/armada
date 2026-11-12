-- A dispatch the Operator holds before any Job exists: the whole body of the person's `dispatch_job`, opaque
-- here, kept until the Operator starts a Job on it, folds it into a live one, or he answers its go-ahead.
-- `asked` is 1 once it waits on his card, which a restart leaves standing. Additive.
CREATE TABLE operator_dispatches (
    id         TEXT NOT NULL PRIMARY KEY,
    body       TEXT NOT NULL,
    created_at TEXT NOT NULL,
    asked      INTEGER NOT NULL DEFAULT 0
) STRICT;
