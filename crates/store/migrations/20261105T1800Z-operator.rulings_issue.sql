-- The open issue a row linked a bug report or a Later item to, by its reference. Null for every
-- other row.
ALTER TABLE operator_rulings ADD COLUMN issue TEXT;
