-- When the person took back a ruling the Operator made by following something he said: the Job is told it is
-- withdrawn and its question is his to answer. Null on every row before this and on every row not undone. Additive.
ALTER TABLE operator_turn ADD COLUMN undone_at TEXT;
