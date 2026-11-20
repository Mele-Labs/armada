-- A card he pressed an option on is picked: `picked` is the option's label and `picked_at` when. It leaves the
-- front for the page's working cards until the Operator settles it. `steps` is what the Operator reported
-- doing on it, a JSON array of `{at, said}`. `returned_why` and `returned_at` are set when it came back to the
-- front: the Operator reported it failed, or nothing settled it in time. Additive: absent on every other row,
-- and an older row reads as not picked.
ALTER TABLE operator_turn ADD COLUMN picked TEXT;
ALTER TABLE operator_turn ADD COLUMN picked_at TEXT;
ALTER TABLE operator_turn ADD COLUMN steps TEXT;
ALTER TABLE operator_turn ADD COLUMN returned_why TEXT;
ALTER TABLE operator_turn ADD COLUMN returned_at TEXT;
