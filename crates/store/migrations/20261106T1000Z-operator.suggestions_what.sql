-- The next piece of work a suggestion on an end or a merge would start or file, in a line. Null on every
-- suggestion kept before this column, and on one that starts nothing. Additive.
ALTER TABLE operator_suggestions ADD COLUMN what TEXT;
