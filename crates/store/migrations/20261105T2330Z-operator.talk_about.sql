-- The board item or Needs you item a message to the Operator was about, so a thread can be grouped by
-- item. Null on a message about none, and on every message kept before this column. Additive.
ALTER TABLE operator_talk ADD COLUMN about TEXT;
