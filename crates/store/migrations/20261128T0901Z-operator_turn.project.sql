-- The project of the Operator a card belongs to, by id: the project of the session that raised it. Additive:
-- NULL on every item Fleet raised itself and on every row kept before each project had an Operator of its own.
ALTER TABLE operator_turn ADD COLUMN project TEXT;
