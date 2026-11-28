-- The project of the Operator a feed line belongs to, by id. Additive: NULL on every line kept before each
-- project had an Operator of its own.
ALTER TABLE operator_feed ADD COLUMN project TEXT;
