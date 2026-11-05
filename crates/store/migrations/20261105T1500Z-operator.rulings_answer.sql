-- The answer a row gave to a question or a permission card, in its option's words: the Operator's,
-- or the person's own press, kept so a second Job asking the same is offered it. Null for every
-- other row.
ALTER TABLE operator_rulings ADD COLUMN answer TEXT;
CREATE INDEX operator_rulings_answered ON operator_rulings (manifest_id, subject) WHERE answer IS NOT NULL;
